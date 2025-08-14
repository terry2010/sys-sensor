using System.Text.Json;
using System.Text;
using LibreHardwareMonitor.Hardware;
using System.Linq;
using System.Security.Principal;
using System.IO;
using System.Collections.Generic;
using System.Text.RegularExpressions;
using SensorBridge;

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
        int idleSecThreshold = ConfigurationManager.ReadEnvInt("BRIDGE_SELFHEAL_IDLE_SEC", 300, 30, 3600);
        int excThreshold = ConfigurationManager.ReadEnvInt("BRIDGE_SELFHEAL_EXC_MAX", 5, 1, 100);
        int periodicReopenSec = ConfigurationManager.ReadEnvInt("BRIDGE_PERIODIC_REOPEN_SEC", 0, 0, 86400);

        // 日志控制：
        // BRIDGE_SUMMARY_EVERY_TICKS: 每 N 次循环输出状态摘要到 stderr/日志文件（默认 60，0 表示关闭）
        // BRIDGE_DUMP_EVERY_TICKS: 每 N 次循环转储完整传感器树到 stderr/日志文件（默认 0 关闭）
        // BRIDGE_LOG_FILE: 若设置，则将日志追加写入到此文件（自动创建目录）
        int summaryEvery = ConfigurationManager.ReadEnvInt("BRIDGE_SUMMARY_EVERY_TICKS", 60, 0, 360000);
        int dumpEvery = ConfigurationManager.ReadEnvInt("BRIDGE_DUMP_EVERY_TICKS", 0, 0, 360000);
        try
        {
            var lf = Environment.GetEnvironmentVariable("BRIDGE_LOG_FILE");
            if (!string.IsNullOrWhiteSpace(lf))
            {
                ConfigurationManager.InitializeLogFile(lf);
            }
        }
        catch { }

        bool isAdminStart = false;
        try { isAdminStart = new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator); } catch { }
        ConfigurationManager.Log($"[start] idleSec={idleSecThreshold} excMax={excThreshold} periodicReopenSec={periodicReopenSec} summaryEvery={summaryEvery} dumpEvery={dumpEvery} isAdmin={isAdminStart}");

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

                float? cpuTemp = DataCollector.PickCpuTemperature(computer);
                float? moboTemp = DataCollector.PickMotherboardTemperature(computer);
                var fans = DataCollector.CollectFans(computer);
                var fansRaw = DataCollector.CollectFansRaw(computer);
                var moboVoltages = DataCollector.CollectMoboVoltages(computer);
                var storageTemps = DataCollector.CollectStorageTemps(computer);
                var gpus = DataCollector.CollectGpus(computer);

                // Flags
                bool anyTempSensor = SensorUtils.HasSensor(computer, SensorType.Temperature);
                bool anyTempValue = SensorUtils.HasSensorValue(computer, SensorType.Temperature);
                bool anyFanSensor = SensorUtils.HasSensor(computer, SensorType.Fan) || SensorUtils.HasFanLikeControl(computer);
                bool anyFanValue = SensorUtils.HasSensorValue(computer, SensorType.Fan) || SensorUtils.HasFanLikeControlWithValue(computer);
                bool isAdmin = new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator);

                // 记录是否有有效读数（温度/风扇任一有值即为“好”）
                if (anyTempValue || anyFanValue || cpuTemp.HasValue || moboTemp.HasValue)
                {
                    lastGood = DateTime.UtcNow;
                }

                // 状态变更日志（有无有效温度/风扇值）
                if (lastHasTempValue == null || lastHasTempValue != anyTempValue)
                {
                    ConfigurationManager.Log($"[state] hasTempValue {lastHasTempValue}->{anyTempValue}");
                    lastHasTempValue = anyTempValue;
                }
                if (lastHasFanValue == null || lastHasFanValue != anyFanValue)
                {
                    ConfigurationManager.Log($"[state] hasFanValue {lastHasFanValue}->{anyFanValue}");
                    lastHasFanValue = anyFanValue;
                }

                // 周期摘要
                if (summaryEvery > 0 && tick % summaryEvery == 0)
                {
                    int idleSec = (int)(DateTime.UtcNow - lastGood).TotalSeconds;
                    ConfigurationManager.Log($"[summary] tick={tick} cpuTemp={ConfigurationManager.Fmt(cpuTemp)} moboTemp={ConfigurationManager.Fmt(moboTemp)} fansCount={(fans?.Count ?? 0)} hasTemp={anyTempSensor}/{anyTempValue} hasFan={anyFanSensor}/{anyFanValue} idleSec={idleSec}");
                }

                var nowUtc = DateTime.UtcNow;
                int idleSecNow = (int)(nowUtc - lastGood).TotalSeconds;
                int uptimeSec = (int)(nowUtc - startUtc).TotalSeconds;

                // 收集 CPU 包功耗/频率/限频标志
                var cpuExtra = DataCollector.CollectCpuExtra(computer);
                // 收集 CPU 每核心 负载/频率/温度
                var cpuPerCore = DataCollector.CollectCpuPerCore(computer);

                var payload = new
                {
                    cpuTempC = cpuTemp,
                    moboTempC = moboTemp,
                    fans = (fans != null && fans.Count > 0) ? fans : null,
                    fansExtra = (fansRaw != null && fansRaw.Count > 0) ? fansRaw : null,
                    moboVoltages = (moboVoltages != null && moboVoltages.Count > 0) ? moboVoltages : null,
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
                ConfigurationManager.Log($"[error] exception #{consecutiveExceptions}: {ex}");
                if (consecutiveExceptions >= excThreshold)
                {
                    ConfigurationManager.Log($"[selfheal] consecutive exceptions >= {excThreshold}, reopening Computer...");
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
                    ConfigurationManager.Log($"[selfheal] idle={idleSec}s, sinceReopen={sinceReopenSec}s -> reopening Computer...");
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






}
