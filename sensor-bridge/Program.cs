using System.Text.Json;
using System.Text;
using LibreHardwareMonitor.Hardware;
using System.Linq;
using System.Security.Principal;
using System.IO;
using System.Collections.Generic;
using System.Text.RegularExpressions;

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
        var startUtc = DateTime.UtcNow; // 进程启动时间（用于 uptimeSec）
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
                var storageTemps = CollectStorageTemps(computer);
                var gpus = CollectGpus(computer);

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

                var nowUtc = DateTime.UtcNow;
                int idleSecNow = (int)(nowUtc - lastGood).TotalSeconds;
                int uptimeSec = (int)(nowUtc - startUtc).TotalSeconds;

                // 收集 CPU 包功耗/频率/限频标志
                var cpuExtra = CollectCpuExtra(computer);
                // 收集 CPU 每核心 负载/频率/温度
                var cpuPerCore = CollectCpuPerCore(computer);

                var payload = new
                {
                    cpuTempC = cpuTemp,
                    moboTempC = moboTemp,
                    fans = (fans != null && fans.Count > 0) ? fans : null,
                    storageTemps = (storageTemps != null && storageTemps.Count > 0) ? storageTemps : null,
                    gpus = (gpus != null && gpus.Count > 0) ? gpus : null,
                    isAdmin = isAdmin,
                    hasTemp = anyTempSensor,
                    hasTempValue = anyTempValue,
                    hasFan = anyFanSensor,
                    hasFanValue = anyFanValue,
                    // 第二梯队：CPU 指标
                    cpuPkgPowerW = cpuExtra?.PkgPowerW,
                    cpuAvgFreqMhz = cpuExtra?.AvgCoreMhz,
                    cpuThrottleActive = cpuExtra?.ThrottleActive,
                    cpuThrottleReasons = (cpuExtra?.ThrottleReasons != null && cpuExtra.ThrottleReasons.Count > 0) ? cpuExtra.ThrottleReasons : null,
                    // CPU 每核心数组
                    cpuCoreLoadsPct = (cpuPerCore?.Loads != null && cpuPerCore.Loads.Count > 0) ? cpuPerCore.Loads : null,
                    cpuCoreClocksMhz = (cpuPerCore?.ClocksMhz != null && cpuPerCore.ClocksMhz.Count > 0) ? cpuPerCore.ClocksMhz : null,
                    cpuCoreTempsC = (cpuPerCore?.TempsC != null && cpuPerCore.TempsC.Count > 0) ? cpuPerCore.TempsC : null,
                    // 自愈健康指标（可选）
                    hbTick = tick,
                    idleSec = idleSecNow,
                    excCount = consecutiveExceptions,
                    uptimeSec = uptimeSec,
                    sinceReopenSec = (int)(nowUtc - lastReopen).TotalSeconds,
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
            IsStorageEnabled = true,
            IsNetworkEnabled = false,
            IsGpuEnabled = true,
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

    // 存储温度名称映射：将通用英文名转换为更具体的位置名称
    static string MapStorageTempName(string? sensorName)
    {
        var n = sensorName?.Trim() ?? string.Empty;
        if (string.IsNullOrEmpty(n)) return "温度";

        // 标准别名优先
        if (n.Equals("Temperature", StringComparison.OrdinalIgnoreCase)
            || n.IndexOf("Composite", StringComparison.OrdinalIgnoreCase) >= 0
            || n.IndexOf("Drive Temperature", StringComparison.OrdinalIgnoreCase) >= 0)
            return "复合"; // NVMe Composite/盘体综合温度

        if (n.Equals("Temperature 1", StringComparison.OrdinalIgnoreCase)
            || n.IndexOf("Controller", StringComparison.OrdinalIgnoreCase) >= 0)
            return "控制器";

        if (n.Equals("Temperature 2", StringComparison.OrdinalIgnoreCase)
            || n.IndexOf("NAND", StringComparison.OrdinalIgnoreCase) >= 0
            || n.IndexOf("Memory", StringComparison.OrdinalIgnoreCase) >= 0
            || n.IndexOf("Flash", StringComparison.OrdinalIgnoreCase) >= 0)
            return "闪存";

        if (n.IndexOf("Drive", StringComparison.OrdinalIgnoreCase) >= 0)
            return "盘体";

        // 未知则原样返回
        return n;
    }

    // 存储温度模型
    class StorageTemp
    {
        public string? Name { get; set; }
        public float? TempC { get; set; }
    }

    // 收集存储（NVMe/SSD）温度
    static List<StorageTemp> CollectStorageTemps(IComputer computer)
    {
        var list = new List<StorageTemp>();
        try
        {
            void CollectFromHw(IHardware hw, string? deviceName)
            {
                foreach (var s in hw.Sensors)
                {
                    if (s.SensorType == SensorType.Temperature && s.Value.HasValue)
                    {
                        var v = s.Value.Value;
                        if (v > -50 && v < 150)
                        {
                            var loc = MapStorageTempName(s.Name);
                            var dev = (deviceName ?? hw.Name ?? string.Empty).Trim();
                            var full = string.IsNullOrEmpty(dev) ? loc : ($"{dev} {loc}");
                            list.Add(new StorageTemp { Name = full, TempC = v });
                        }
                    }
                }
                foreach (var sh in hw.SubHardware)
                {
                    CollectFromHw(sh, deviceName);
                }
            }

            foreach (var hw in computer.Hardware)
            {
                if (hw.HardwareType == HardwareType.Storage)
                {
                    var dev = hw.Name;
                    CollectFromHw(hw, dev);
                }
            }

            // 不再按名称去重：避免多盘或同盘多位置（复合/控制器/闪存）被合并丢失
        }
        catch { }
        return list;
    }

    // CPU 每核心指标
    class CpuPerCore
    {
        public List<float?> Loads { get; set; } = new List<float?>();
        public List<double?> ClocksMhz { get; set; } = new List<double?>();
        public List<float?> TempsC { get; set; } = new List<float?>();
    }

    static bool TryParseCoreIndex(string? name, out int index1Based)
    {
        index1Based = -1;
        var n = name ?? string.Empty;
        if (n.Length == 0) return false;
        try
        {
            // 优先匹配 “#<num>”
            var m = Regex.Match(n, @"#\s*(?<idx>\d+)", RegexOptions.IgnoreCase);
            if (m.Success && int.TryParse(m.Groups["idx"].Value, out var idx1) && idx1 > 0)
            {
                index1Based = idx1;
                return true;
            }
            // 兼容 "Core 1", "CPU Core 2", "Core #3", "P-Core 4", "E-Core 5"
            m = Regex.Match(n, @"(?:cpu\s*)?(?:p-?core|e-?core|core)\s*#?\s*(?<idx>\d+)", RegexOptions.IgnoreCase);
            if (m.Success && int.TryParse(m.Groups["idx"].Value, out idx1) && idx1 > 0)
            {
                index1Based = idx1;
                return true;
            }
            return false;
        }
        catch { return false; }
    }

    static CpuPerCore? CollectCpuPerCore(IComputer computer)
    {
        try
        {
            var loadByIdx = new Dictionary<int, float?>();
            var clockByIdx = new Dictionary<int, double?>();
            var tempByIdx = new Dictionary<int, float?>();

            void Scan(IHardware h)
            {
                foreach (var s in h.Sensors)
                {
                    try
                    {
                        var name = s.Name ?? string.Empty;
                        if (!s.Value.HasValue) continue;
                        if (!TryParseCoreIndex(name, out var idx1)) continue;
                        var v = s.Value.Value;
                        if (s.SensorType == SensorType.Load)
                        {
                            if (v >= 0 && v <= 100)
                            {
                                if (!loadByIdx.TryGetValue(idx1, out var old) || (old ?? -1) < (float)v)
                                    loadByIdx[idx1] = (float)v;
                            }
                        }
                        else if (s.SensorType == SensorType.Clock)
                        {
                            if (v > 10 && v < 10000)
                            {
                                if (!clockByIdx.TryGetValue(idx1, out var old) || (old ?? -1) < v)
                                    clockByIdx[idx1] = v;
                            }
                        }
                        else if (s.SensorType == SensorType.Temperature)
                        {
                            if (v > -50 && v < 150)
                            {
                                if (!tempByIdx.TryGetValue(idx1, out var old) || (old ?? -999) < (float)v)
                                    tempByIdx[idx1] = (float)v;
                            }
                        }
                    }
                    catch { }
                }
                foreach (var sh in h.SubHardware) Scan(sh);
            }

            foreach (var hw in computer.Hardware)
            {
                if (hw.HardwareType == HardwareType.Cpu)
                    Scan(hw);
            }

            int maxIdx = 0;
            if (loadByIdx.Count > 0) maxIdx = Math.Max(maxIdx, loadByIdx.Keys.Max());
            if (clockByIdx.Count > 0) maxIdx = Math.Max(maxIdx, clockByIdx.Keys.Max());
            if (tempByIdx.Count > 0) maxIdx = Math.Max(maxIdx, tempByIdx.Keys.Max());
            if (maxIdx <= 0)
            {
                return new CpuPerCore();
            }

            var loads = Enumerable.Range(1, maxIdx).Select(i => loadByIdx.ContainsKey(i) ? loadByIdx[i] : (float?)null).ToList();
            var clocks = Enumerable.Range(1, maxIdx).Select(i => clockByIdx.ContainsKey(i) ? clockByIdx[i] : (double?)null).ToList();
            var temps = Enumerable.Range(1, maxIdx).Select(i => tempByIdx.ContainsKey(i) ? tempByIdx[i] : (float?)null).ToList();

            return new CpuPerCore
            {
                Loads = loads,
                ClocksMhz = clocks,
                TempsC = temps,
            };
        }
        catch { return null; }
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

    // GPU 信息
    class GpuInfo
    {
        public string? Name { get; set; }
        public float? TempC { get; set; }
        public float? LoadPct { get; set; }
        public double? CoreMhz { get; set; }
        public int? FanRpm { get; set; }
        public double? VramUsedMb { get; set; }
        public double? PowerW { get; set; }
    }

    static List<GpuInfo> CollectGpus(IComputer computer)
    {
        var list = new List<GpuInfo>();
        try
        {
            foreach (var hw in computer.Hardware)
            {
                if (hw.HardwareType != HardwareType.GpuNvidia &&
                    hw.HardwareType != HardwareType.GpuAmd &&
                    hw.HardwareType != HardwareType.GpuIntel)
                    continue;

                double? temp = null;
                double? load = null;
                double? coreMhz = null;
                int? fanRpm = null;
                double? vramMb = null;
                double? powerW = null;

                void Scan(IHardware h)
                {
                    foreach (var s in h.Sensors)
                    {
                        try
                        {
                            if (!s.Value.HasValue) continue;
                            var v = s.Value.Value;
                            var t = s.SensorType;
                            var name = s.Name ?? string.Empty;
                            var nameLc = name.ToLowerInvariant();
                            if (t == SensorType.Temperature)
                            {
                                if (v > -50 && v < 150)
                                {
                                    // 倾向 Core/HotSpot，更高者优先
                                    if (name.IndexOf("hot", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                        name.IndexOf("core", StringComparison.OrdinalIgnoreCase) >= 0)
                                        temp = Math.Max(temp ?? double.MinValue, v);
                                    else
                                        temp = Math.Max(temp ?? double.MinValue, v);
                                }
                            }
                            else if (t == SensorType.Load)
                            {
                                if (v >= 0 && v <= 100)
                                {
                                    // 倾向 Core/GPU Core 负载
                                    if (name.IndexOf("core", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                        name.IndexOf("gpu", StringComparison.OrdinalIgnoreCase) >= 0)
                                        load = Math.Max(load ?? 0.0, v);
                                }
                            }
                            else if (t == SensorType.Clock)
                            {
                                // MHz，倾向 Core/Graphics
                                if (name.IndexOf("core", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    name.IndexOf("graphics", StringComparison.OrdinalIgnoreCase) >= 0)
                                {
                                    if (v > 10 && v < 50000)
                                        coreMhz = Math.Max(coreMhz ?? 0.0, v);
                                }
                            }
                            else if (t == SensorType.Fan)
                            {
                                if (v > 0 && v < 20000)
                                {
                                    var rpm = (int)Math.Round(v);
                                    fanRpm = Math.Max(fanRpm ?? 0, rpm);
                                }
                            }
                            else if (t == SensorType.Power)
                            {
                                // GPU 板卡/总功耗（W）
                                if (v > 0 && v < 1000)
                                {
                                    // 优先包含 total/board 的命名，否则取最大值
                                    if (nameLc.Contains("total") || nameLc.Contains("board"))
                                        powerW = Math.Max(powerW ?? 0.0, v);
                                    else
                                        powerW = Math.Max(powerW ?? 0.0, v);
                                }
                            }
                            else if (t == SensorType.SmallData || t == SensorType.Data)
                            {
                                // VRAM 使用（MB/Bytes），匹配名称关键字
                                bool isVramName =
                                    (nameLc.Contains("vram") || nameLc.Contains("memory")) &&
                                    (nameLc.Contains("used") || nameLc.Contains("usage") || nameLc.Contains("util"));
                                if (isVramName)
                                {
                                    double mb;
                                    if (v > 8 * 1024 * 1024) // 以 Bytes 暴露
                                        mb = v / (1024.0 * 1024.0);
                                    else
                                        mb = v; // 直接按 MB 解释
                                    if (mb >= 0 && mb < 1024 * 1024) // 排除异常值
                                        vramMb = Math.Max(vramMb ?? 0.0, mb);
                                }
                            }
                        }
                        catch { }
                    }
                    foreach (var sh in h.SubHardware) Scan(sh);
                }
                Scan(hw);

                if (temp.HasValue || load.HasValue || coreMhz.HasValue || fanRpm.HasValue || vramMb.HasValue || powerW.HasValue)
                {
                    list.Add(new GpuInfo
                    {
                        Name = hw.Name,
                        TempC = temp.HasValue ? (float?)temp.Value : null,
                        LoadPct = load.HasValue ? (float?)load.Value : null,
                        CoreMhz = coreMhz,
                        FanRpm = fanRpm,
                        VramUsedMb = vramMb,
                        PowerW = powerW,
                    });
                }
            }
        }
        catch { }
        return list;
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

    // CPU 额外信息（第二梯队）
    class CpuExtra
    {
        public double? PkgPowerW { get; set; }
        public double? AvgCoreMhz { get; set; }
        public bool? ThrottleActive { get; set; }
        public List<string>? ThrottleReasons { get; set; }
    }

    static CpuExtra? CollectCpuExtra(IComputer computer)
    {
        try
        {
            // 允许通过环境变量将限频默认视为 false，以避免 UI 显示“—”。
            // BRIDGE_THROTTLE_DEFAULT_FALSE=1|true 时生效。
            var envDefaultFalse = Environment.GetEnvironmentVariable("BRIDGE_THROTTLE_DEFAULT_FALSE");
            bool defaultThrottleFalse = !string.IsNullOrEmpty(envDefaultFalse) &&
                (string.Equals(envDefaultFalse, "1", StringComparison.OrdinalIgnoreCase) ||
                 string.Equals(envDefaultFalse, "true", StringComparison.OrdinalIgnoreCase));

            double? pkgW = null;
            var coreClocks = new List<double>();
            bool? throttle = null;
            bool throttleSeen = false; // 是否见到限频相关传感器（即使当前未触发）
            var reasons = new List<string>();

            foreach (var hw in computer.Hardware)
            {
                bool isCpuRoot = hw.HardwareType == HardwareType.Cpu;
                // 遍历所有硬件，但仅对 CPU 硬件统计功耗/频率；限频标志在任意硬件上都可能出现，均需扫描
                void ScanHw(IHardware h, bool isCpu)
                {
                    foreach (var s in h.Sensors)
                    {
                        try
                        {
                            var t = s.SensorType;
                            var name = s.Name ?? string.Empty;
                            if (!s.Value.HasValue) continue;
                            var v = s.Value.Value;
                            if (t == SensorType.Power && isCpu)
                            {
                                // 优先选择名称包含 Package 的功耗；否则取最大值作为包功耗近似
                                if (name.IndexOf("package", StringComparison.OrdinalIgnoreCase) >= 0
                                    || name.IndexOf("cpu package", StringComparison.OrdinalIgnoreCase) >= 0
                                    || name.IndexOf("pkg", StringComparison.OrdinalIgnoreCase) >= 0)
                                {
                                    pkgW = v;
                                }
                                else
                                {
                                    pkgW = Math.Max(pkgW ?? 0.0, v);
                                }
                                // 电源限制/热限提示
                                if (name.IndexOf("limit", StringComparison.OrdinalIgnoreCase) >= 0)
                                {
                                    throttleSeen = true;
                                    if (v > 0)
                                    {
                                        throttle = true;
                                        reasons.Add(name);
                                    }
                                }
                            }
                            else if (t == SensorType.Clock && isCpu)
                            {
                                // 收集 Core/Efficient/E-core/P-core 频率（MHz）
                                if (name.IndexOf("core", StringComparison.OrdinalIgnoreCase) >= 0
                                    || name.IndexOf("effective", StringComparison.OrdinalIgnoreCase) >= 0
                                    || name.IndexOf("p-core", StringComparison.OrdinalIgnoreCase) >= 0
                                    || name.IndexOf("e-core", StringComparison.OrdinalIgnoreCase) >= 0)
                                {
                                    if (v > 10 && v < 10000)
                                        coreClocks.Add(v);
                                }
                            }
                            else if (t == SensorType.Load)
                            {
                                // 某些平台会以 Load 名称提示 thermal/power throttling（极少见），仅作为提示保留
                                if (name.IndexOf("thrott", StringComparison.OrdinalIgnoreCase) >= 0 && v > 0)
                                {
                                    throttle = true;
                                    reasons.Add(name);
                                }
                            }
                            else if (t == SensorType.Factor)
                            {
                                // 许多限频标志以 Factor(0/1) 暴露：Thermal Throttling, Power Limit Exceeded, PROCHOT, PL1/PL2/EDP Limit, Tau 等
                                bool maybeThrottleFlag =
                                    name.IndexOf("thrott", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    name.IndexOf("limit", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    name.IndexOf("prochot", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    name.IndexOf("pl1", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    name.IndexOf("pl2", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    name.IndexOf("edp", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    name.IndexOf("tau", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    name.IndexOf("thermal", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    name.IndexOf("vr", StringComparison.OrdinalIgnoreCase) >= 0;
                                if (maybeThrottleFlag)
                                {
                                    throttleSeen = true;
                                    if (v > 0.5)
                                    {
                                        throttle = true;
                                        reasons.Add(name);
                                    }
                                }
                            }
                        }
                        catch { }
                    }
                    foreach (var sh in h.SubHardware) ScanHw(sh, isCpu);
                }
                ScanHw(hw, isCpuRoot);
            }

            // 若扫描到限频相关传感器但未触发，或启用了默认 false 策略，则明确标记为 false（而不是 null）
            if (throttle == null && (throttleSeen || defaultThrottleFalse))
                throttle = false;

            var extra = new CpuExtra
            {
                PkgPowerW = pkgW,
                AvgCoreMhz = coreClocks.Count > 0 ? coreClocks.Average() : (double?)null,
                ThrottleActive = throttle,
                ThrottleReasons = reasons.Count > 0 ? reasons.Distinct().ToList() : null,
            };
            // 若完全无数据则返回 null，避免冗余字段
            if (extra.PkgPowerW == null && extra.AvgCoreMhz == null && extra.ThrottleActive == null && (extra.ThrottleReasons == null || extra.ThrottleReasons.Count == 0))
                return null;
            return extra;
        }
        catch { return null; }
    }
}
