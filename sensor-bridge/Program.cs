using System.Text.Json;
using System.Text;
using LibreHardwareMonitor.Hardware;
using System.Linq;
using System.Security.Principal;
using System.IO;

class Program
{
    static void Main()
    {
        Console.OutputEncoding = Encoding.UTF8;
        var jsonOptions = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull,
            WriteIndented = false
        };

        // 初始化硬件枚举
        var computer = MakeComputer();
        // 自愈相关状态
        DateTime lastGood = DateTime.UtcNow; // 最近一次有有效读数的时间
        DateTime lastReopen = DateTime.UtcNow; // 最近一次重建 Computer 的时间
        int consecutiveExceptions = 0;
        // 环境变量阈值：
        // BRIDGE_SELFHEAL_IDLE_SEC: 在此秒数内若无有效温度/风扇读数则重建（默认 300s）
        // BRIDGE_SELFHEAL_EXC_MAX: 连续异常次数达到该值则重建（默认 5 次）
        // BRIDGE_PERIODIC_REOPEN_SEC: 周期性强制重建（0 表示关闭，默认 0）
        int idleSecThreshold = ReadEnvInt("BRIDGE_SELFHEAL_IDLE_SEC", 300, 30, 3600);
        int excThreshold = ReadEnvInt("BRIDGE_SELFHEAL_EXC_MAX", 5, 1, 100);
        int periodicReopenSec = ReadEnvInt("BRIDGE_PERIODIC_REOPEN_SEC", 0, 0, 86400);

        // 日志控制：
        // BRIDGE_SUMMARY_EVERY_TICKS: 每 N 次循环输出状态摘要到 stderr/日志文件（默认 60，0 表示关闭）
        // BRIDGE_DUMP_EVERY_TICKS: 每 N 次循环转储完整传感器树到 stderr/日志文件（默认 0 关闭）
        // BRIDGE_LOG_FILE: 若设置，则将日志追加写入到此文件（自动创建目录）
        int summaryEvery = ReadEnvInt("BRIDGE_SUMMARY_EVERY_TICKS", 60, 0, 360000);
        int dumpEvery = ReadEnvInt("BRIDGE_DUMP_EVERY_TICKS", 0, 0, 360000);
        try
        {
            var lf = Environment.GetEnvironmentVariable("BRIDGE_LOG_FILE");
            if (!string.IsNullOrWhiteSpace(lf))
            {
                s_logFilePath = lf;
                TryEnsureLogDir(lf);
            }
        }
        catch { }

        bool isAdminStart = false;
        try { isAdminStart = new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator); } catch { }
        Log($"[start] idleSec={idleSecThreshold} excMax={excThreshold} periodicReopenSec={periodicReopenSec} summaryEvery={summaryEvery} dumpEvery={dumpEvery} isAdmin={isAdminStart}");

        // 可选：通过环境变量 BRIDGE_TICKS 限定循环次数（便于自动化测试）
        int tick = 0;
        int? maxTicks = null;
        try
        {
            var envTicks = Environment.GetEnvironmentVariable("BRIDGE_TICKS");
            if (!string.IsNullOrWhiteSpace(envTicks) && int.TryParse(envTicks, out var t) && t > 0)
            {
                maxTicks = t;
            }
        }
        catch { }
        bool? lastHasTempValue = null;
        bool? lastHasFanValue = null;
        while (true)
        {
            try
            {
                // 使用访问者统一刷新全树
                computer.Accept(new UpdateVisitor());
                if (dumpEvery > 0 && tick % dumpEvery == 0) DumpSensors(computer);

                float? cpuTemp = PickCpuTemperature(computer);
                float? moboTemp = PickMotherboardTemperature(computer);
                var fans = CollectFans(computer);

                // Flags
                bool anyTempSensor = HasSensor(computer, SensorType.Temperature);
                bool anyTempValue = HasSensorValue(computer, SensorType.Temperature);
                bool anyFanSensor = HasSensor(computer, SensorType.Fan) || HasFanLikeControl(computer);
                bool anyFanValue = HasSensorValue(computer, SensorType.Fan) || HasFanLikeControlWithValue(computer);
                bool isAdmin = new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator);

                // 记录是否有有效读数（温度/风扇任一有值即为“好”）
                if (anyTempValue || anyFanValue || cpuTemp.HasValue || moboTemp.HasValue)
                {
                    lastGood = DateTime.UtcNow;
                }

                // 状态变更日志（有无有效温度/风扇值）
                if (lastHasTempValue == null || lastHasTempValue != anyTempValue)
                {
                    Log($"[state] hasTempValue {lastHasTempValue}->{anyTempValue}");
                    lastHasTempValue = anyTempValue;
                }
                if (lastHasFanValue == null || lastHasFanValue != anyFanValue)
                {
                    Log($"[state] hasFanValue {lastHasFanValue}->{anyFanValue}");
                    lastHasFanValue = anyFanValue;
                }

                // 周期摘要
                if (summaryEvery > 0 && tick % summaryEvery == 0)
                {
                    int idleSec = (int)(DateTime.UtcNow - lastGood).TotalSeconds;
                    Log($"[summary] tick={tick} cpuTemp={Fmt(cpuTemp)} moboTemp={Fmt(moboTemp)} fansCount={(fans?.Count ?? 0)} hasTemp={anyTempSensor}/{anyTempValue} hasFan={anyFanSensor}/{anyFanValue} idleSec={idleSec}");
                }

                var payload = new
                {
                    cpuTempC = cpuTemp,
                    moboTempC = moboTemp,
                    fans = fans.Count > 0 ? fans : null,
                    isAdmin = isAdmin,
                    hasTemp = anyTempSensor,
                    hasTempValue = anyTempValue,
                    hasFan = anyFanSensor,
                    hasFanValue = anyFanValue,
                };

                Console.WriteLine(JsonSerializer.Serialize(payload, jsonOptions));
                Console.Out.Flush();
                // 正常一轮结束，异常计数清零
                consecutiveExceptions = 0;
            }
            catch (Exception ex)
            {
                // 累计异常；达到阈值触发自愈
                consecutiveExceptions++;
                Log($"[error] exception #{consecutiveExceptions}: {ex}");
                if (consecutiveExceptions >= excThreshold)
                {
                    Log($"[selfheal] consecutive exceptions >= {excThreshold}, reopening Computer...");
                    try { computer.Close(); } catch { }
                    computer = MakeComputer();
                    lastReopen = DateTime.UtcNow;
                    consecutiveExceptions = 0;
                    // 重建后进入下一轮
                }
            }
            tick++;
            if (maxTicks.HasValue && tick >= maxTicks.Value)
            {
                break;
            }
            // 闲置自愈与周期重建
            try
            {
                var now = DateTime.UtcNow;
                int idleSec = (int)(now - lastGood).TotalSeconds;
                int sinceReopenSec = (int)(now - lastReopen).TotalSeconds;
                bool needIdleReopen = idleSecThreshold > 0 && idleSec >= idleSecThreshold;
                bool needPeriodicReopen = periodicReopenSec > 0 && sinceReopenSec >= periodicReopenSec;
                if (needIdleReopen || needPeriodicReopen)
                {
                    Log($"[selfheal] idle={idleSec}s, sinceReopen={sinceReopenSec}s -> reopening Computer...");
                    try { computer.Close(); } catch { }
                    computer = MakeComputer();
                    lastReopen = now;
                    lastGood = now; // 避免立即再次触发
                }
            }
            catch { }
            Thread.Sleep(1000);
        }
    }

    // 创建并开启 Computer，统一初始化开关
    static Computer MakeComputer()
    {
        var c = new Computer
        {
            IsCpuEnabled = true,
            IsMotherboardEnabled = true,
            IsControllerEnabled = true,
            IsMemoryEnabled = false,
            IsStorageEnabled = false,
            IsNetworkEnabled = false,
            IsGpuEnabled = false,
        };
        c.Open();
        return c;
    }

    // 从环境变量读取 int，并限定范围
    static int ReadEnvInt(string name, int @default, int min, int max)
    {
        try
        {
            var s = Environment.GetEnvironmentVariable(name);
            if (!string.IsNullOrWhiteSpace(s) && int.TryParse(s, out var v))
            {
                if (v < min) return min;
                if (v > max) return max;
                return v;
            }
        }
        catch { }
        return @default;
    }

    // 日志设施：同时写入 stderr 与可选文件
    static string? s_logFilePath;
    static readonly object s_logLock = new object();
    static void Log(string msg)
    {
        var line = $"[bridge]{DateTime.UtcNow:O} {msg}";
        try { Console.Error.WriteLine(line); Console.Error.Flush(); } catch { }
        var path = s_logFilePath;
        if (!string.IsNullOrWhiteSpace(path))
        {
            try { lock (s_logLock) { File.AppendAllText(path, line + Environment.NewLine, Encoding.UTF8); } } catch { }
        }
    }

    static void TryEnsureLogDir(string path)
    {
        try
        {
            var dir = Path.GetDirectoryName(path);
            if (!string.IsNullOrEmpty(dir)) Directory.CreateDirectory(dir);
        }
        catch { }
    }

    static string Fmt(float? v) => v.HasValue ? v.Value.ToString("0.0") : "—";

    static bool HasSensor(IComputer computer, SensorType type)
    {
        foreach (var hw in computer.Hardware)
        {
            if (hw.Sensors.Any(s => s.SensorType == type)) return true;
            foreach (var sh in hw.SubHardware)
                if (sh.Sensors.Any(s => s.SensorType == type)) return true;
        }
        return false;
    }

    static bool HasSensorValue(IComputer computer, SensorType type)
    {
        foreach (var hw in computer.Hardware)
        {
            if (hw.Sensors.Any(s => s.SensorType == type && s.Value.HasValue)) return true;
            foreach (var sh in hw.SubHardware)
                if (sh.Sensors.Any(s => s.SensorType == type && s.Value.HasValue)) return true;
        }
        return false;
    }

    static bool HasFanLikeControl(IComputer computer)
    {
        foreach (var hw in computer.Hardware)
        {
            if (hw.Sensors.Any(IsFanLikeControl)) return true;
            foreach (var sh in hw.SubHardware)
                if (sh.Sensors.Any(IsFanLikeControl)) return true;
        }
        return false;
    }

    static bool HasFanLikeControlWithValue(IComputer computer)
    {
        foreach (var hw in computer.Hardware)
        {
            if (hw.Sensors.Any(s => IsFanLikeControl(s) && s.Value.HasValue)) return true;
            foreach (var sh in hw.SubHardware)
                if (sh.Sensors.Any(s => IsFanLikeControl(s) && s.Value.HasValue)) return true;
        }
        return false;
    }

    static bool IsFanLikeControl(ISensor s)
    {
        if (s.SensorType != SensorType.Control) return false;
        var name = s.Name ?? string.Empty;
        // 常规命名匹配
        if (name.IndexOf("fan", StringComparison.OrdinalIgnoreCase) >= 0
            || name.IndexOf("pwm", StringComparison.OrdinalIgnoreCase) >= 0
            || name.IndexOf("duty", StringComparison.OrdinalIgnoreCase) >= 0
            || name.IndexOf("cool", StringComparison.OrdinalIgnoreCase) >= 0)
            return true;

        // 兼容部分 NUC/EC：在 EC / Motherboard / SuperIO 下，Control 传感器若数值在 [0,100]，也视为风扇占空比
        try
        {
            var hwType = s.Hardware?.HardwareType;
            if (hwType == HardwareType.EmbeddedController || hwType == HardwareType.Motherboard || hwType == HardwareType.SuperIO)
            {
                if (s.Value.HasValue)
                {
                    var v = s.Value.Value;
                    if (v >= 0 && v <= 100) return true;
                }
            }
        }
        catch { }
        return false;
    }
    // 访问者：递归刷新所有硬件与子硬件
    class UpdateVisitor : IVisitor
    {
        public void VisitComputer(IComputer computer) => computer.Traverse(this);
        public void VisitHardware(IHardware hardware)
        {
            hardware.Update();
            foreach (var sh in hardware.SubHardware)
                sh.Accept(this);
        }
        public void VisitSensor(ISensor sensor) { }
        public void VisitParameter(IParameter parameter) { }
    }

    static void DumpSensors(IComputer computer)
    {
        try
        {
            var sb = new StringBuilder();
            sb.AppendLine("[bridge][dump] sensors:");
            foreach (var hw in computer.Hardware)
            {
                sb.AppendLine($"- HW {hw.HardwareType} | {hw.Name}");
                foreach (var s in hw.Sensors)
                {
                    sb.AppendLine($"  * {s.SensorType} | {s.Name} = {s.Value}");
                }
                foreach (var sh in hw.SubHardware)
                {
                    sb.AppendLine($"  - Sub {sh.HardwareType} | {sh.Name}");
                    foreach (var s in sh.Sensors)
                    {
                        sb.AppendLine($"    * {s.SensorType} | {s.Name} = {s.Value}");
                    }
                }
            }
            Console.Error.WriteLine(sb.ToString());
            Console.Error.Flush();
        }
        catch { }
    }

    static float? PickCpuTemperature(IComputer computer)
    {
        var temps = new List<float>();
        foreach (var hw in computer.Hardware)
        {
            if (hw.HardwareType != HardwareType.Cpu) continue;
            foreach (var s in hw.Sensors)
            {
                if (s.SensorType == SensorType.Temperature && s.Value.HasValue)
                {
                    temps.Add(s.Value.Value);
                }
            }
            foreach (var sh in hw.SubHardware)
            {
                foreach (var s in sh.Sensors)
                {
                    if (s.SensorType == SensorType.Temperature && s.Value.HasValue)
                    {
                        temps.Add(s.Value.Value);
                    }
                }
            }
        }
        if (temps.Count == 0) return null;
        // Prefer higher temps as package/core; cap to plausible range
        var v = temps.Max();
        if (v < -50 || v > 150) return null;
        return v;
    }

    static float? PickMotherboardTemperature(IComputer computer)
    {
        var preferred = new List<float>();
        var all = new List<float>();
        bool NamePreferred(string? n)
        {
            var name = n ?? string.Empty;
            if (name.Length == 0) return false;
            // 排除 CPU/GPU/SSD/VRM/内存等非主板环境温度
            string[] deny = { "cpu", "core", "package", "gpu", "ssd", "nvme", "hdd", "vrm", "dimm", "memory" };
            foreach (var d in deny)
                if (name.IndexOf(d, StringComparison.OrdinalIgnoreCase) >= 0) return false;
            // 倾向匹配主板/系统/EC/PCH/机箱等环境温度
            string[] allow = { "motherboard", "mainboard", "system", "pch", "board", "ambient", "chassis", "ec" };
            foreach (var a in allow)
                if (name.IndexOf(a, StringComparison.OrdinalIgnoreCase) >= 0) return true;
            return false;
        }

        void CollectFromHardware(IHardware hw)
        {
            foreach (var s in hw.Sensors)
            {
                if (s.SensorType == SensorType.Temperature && s.Value.HasValue)
                {
                    var v = s.Value.Value;
                    if (v > -50 && v < 150)
                    {
                        all.Add(v);
                        if (NamePreferred(s.Name)) preferred.Add(v);
                    }
                }
            }
            foreach (var sh in hw.SubHardware)
                CollectFromHardware(sh);
        }

        foreach (var hw in computer.Hardware)
        {
            // 扩大范围：Motherboard / SuperIO / EmbeddedController 下的温度
            if (hw.HardwareType == HardwareType.Motherboard || hw.HardwareType == HardwareType.SuperIO || hw.HardwareType == HardwareType.EmbeddedController)
            {
                CollectFromHardware(hw);
            }
        }
        var src = preferred.Count > 0 ? preferred : all;
        if (src.Count == 0) return null;
        var avg = src.Average();
        if (avg < -50 || avg > 120) return null;
        return avg;
    }

    class FanInfo
    {
        public string? Name { get; set; }
        public int? Rpm { get; set; }
        public int? Pct { get; set; }
    }

    static List<FanInfo> CollectFans(IComputer computer)
    {
        var fans = new List<FanInfo>();
        foreach (var hw in computer.Hardware)
        {
            // 直接遍历所有硬件与子硬件
            foreach (var s in hw.Sensors)
            {
                if (s.SensorType == SensorType.Fan)
                {
                    int? rpm = s.Value.HasValue ? (int?)Math.Round(s.Value.Value) : null;
                    fans.Add(new FanInfo { Name = s.Name, Rpm = rpm });
                }
                else if (s.SensorType == SensorType.Control && IsFanLikeControl(s))
                {
                    int? pct = s.Value.HasValue ? (int?)Math.Round(s.Value.Value) : null; // 0~100
                    fans.Add(new FanInfo { Name = s.Name, Pct = pct });
                }
            }
            foreach (var sh in hw.SubHardware)
            {
                foreach (var s in sh.Sensors)
                {
                    if (s.SensorType == SensorType.Fan)
                    {
                        int? rpm = s.Value.HasValue ? (int?)Math.Round(s.Value.Value) : null;
                        fans.Add(new FanInfo { Name = s.Name, Rpm = rpm });
                    }
                    else if (s.SensorType == SensorType.Control && IsFanLikeControl(s))
                    {
                        int? pct = s.Value.HasValue ? (int?)Math.Round(s.Value.Value) : null;
                        fans.Add(new FanInfo { Name = s.Name, Pct = pct });
                    }
                }
            }
        }
        // 按名称去重：保留最大 RPM 和最大占空比
        var dedup = fans
            .GroupBy(f => f.Name ?? "")
            .Select(g => new FanInfo {
                Name = string.IsNullOrEmpty(g.Key) ? null : g.Key,
                Rpm = g.Max(x => x.Rpm),
                Pct = g.Max(x => x.Pct)
            })
            .Where(f => f.Rpm.HasValue || f.Pct.HasValue)
            .ToList();
        return dedup;
    }
}
