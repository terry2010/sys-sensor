using System;
using System.Text.Json;
using System.Threading;
using LibreHardwareMonitor.Hardware;
using System.Security.Principal;

namespace SensorBridge
{
    /// <summary>
    /// 传感器监控主循环模块 - 负责传感器数据收集、自愈机制和状态管理
    /// </summary>
    public static class SensorMonitor
    {
        /// <summary>
        /// 运行传感器监控主循环
        /// </summary>
        /// <param name="jsonOptions">JSON 序列化选项</param>
        public static void RunMonitoringLoop(JsonSerializerOptions jsonOptions)
        {
            // 初始化硬件枚举
            Computer computer = HardwareManager.MakeComputer();
            var startUtc = DateTime.UtcNow; // 进程启动时间（用于 uptimeSec）
            
            // 自愈相关状态
            DateTime lastGood = DateTime.UtcNow; // 最近一次有有效读数的时间
            DateTime lastReopen = DateTime.UtcNow; // 最近一次重建 Computer 的时间
            int consecutiveExceptions = 0;
            
            // 环境变量阈值
            int idleSecThreshold = ConfigurationManager.ReadEnvInt("BRIDGE_SELFHEAL_IDLE_SEC", 300, 30, 3600);
            int excThreshold = ConfigurationManager.ReadEnvInt("BRIDGE_SELFHEAL_EXC_MAX", 5, 1, 100);
            int periodicReopenSec = ConfigurationManager.ReadEnvInt("BRIDGE_PERIODIC_REOPEN_SEC", 0, 0, 86400);
            
            // 日志控制
            int summaryEvery = ConfigurationManager.ReadEnvInt("BRIDGE_SUMMARY_EVERY_TICKS", 60, 0, 360000);
            int dumpEvery = ConfigurationManager.ReadEnvInt("BRIDGE_DUMP_EVERY_TICKS", 0, 0, 360000);
            
            ConfigurationManager.Log($"[start] idleSec={idleSecThreshold} excMax={excThreshold} periodicReopenSec={periodicReopenSec} summaryEvery={summaryEvery} dumpEvery={dumpEvery} isAdmin={IsAdmin()}");
            
            int tick = 0;
            int? maxTicks = GetMaxTicks();
            bool? lastHasTempValue = null;
            bool? lastHasFanValue = null;
            
            while (true)
            {
                try
                {
                    // 使用访问者统一刷新全树
                    computer.Accept(new UpdateVisitor());
                    
                    if (dumpEvery > 0 && tick % dumpEvery == 0)
                    {
                        HardwareManager.DumpSensors(computer);
                    }
                    
                    // 收集传感器数据
                    var sensorData = CollectSensorData(computer);
                    
                    // 记录是否有有效读数（温度/风扇任一有值即为"好"）
                    if (sensorData.AnyTempValue || sensorData.AnyFanValue || 
                        sensorData.CpuTemp.HasValue || sensorData.MoboTemp.HasValue)
                    {
                        lastGood = DateTime.UtcNow;
                    }
                    
                    // 状态变更日志
                    LogStateChanges(ref lastHasTempValue, ref lastHasFanValue, 
                                   sensorData.AnyTempValue, sensorData.AnyFanValue);
                    
                    // 周期摘要
                    if (summaryEvery > 0 && tick % summaryEvery == 0)
                    {
                        LogSummary(tick, sensorData, lastGood);
                    }
                    
                    // 构建并输出 JSON 数据
                    var payload = BuildPayload(sensorData, startUtc, tick, consecutiveExceptions, lastReopen, lastGood, computer);
                    Console.WriteLine(JsonSerializer.Serialize(payload, jsonOptions));
                    Console.Out.Flush();
                    
                    // 正常一轮结束，异常计数清零
                    consecutiveExceptions = 0;
                }
                catch (Exception ex)
                {
                    // 异常处理和自愈
                    consecutiveExceptions++;
                    ConfigurationManager.Log($"[error] exception #{consecutiveExceptions}: {ex}");
                    
                    if (consecutiveExceptions >= excThreshold)
                    {
                        computer = HandleExceptionSelfHeal(computer, excThreshold, ref lastReopen, ref consecutiveExceptions);
                    }
                }
                
                tick++;
                if (maxTicks.HasValue && tick >= maxTicks.Value)
                {
                    break;
                }
                
                // 闲置自愈与周期重建
                computer = HandleIdleAndPeriodicSelfHeal(computer, lastGood, lastReopen, 
                                                       idleSecThreshold, periodicReopenSec, 
                                                       ref lastReopen, ref lastGood);
                
                Thread.Sleep(1000);
            }
        }
        
        /// <summary>
        /// 收集所有传感器数据
        /// </summary>
        private static SensorData CollectSensorData(IComputer computer)
        {
            return new SensorData
            {
                CpuTemp = DataCollector.PickCpuTemperature(computer),
                MoboTemp = DataCollector.PickMotherboardTemperature(computer),
                Fans = DataCollector.CollectFans(computer),
                FansRaw = DataCollector.CollectFansRaw(computer),
                MoboVoltages = DataCollector.CollectMoboVoltages(computer),
                StorageTemps = DataCollector.CollectStorageTemps(computer),
                Gpus = DataCollector.CollectGpus(computer),
                AnyTempSensor = SensorUtils.HasSensor(computer, SensorType.Temperature),
                AnyTempValue = SensorUtils.HasSensorValue(computer, SensorType.Temperature),
                AnyFanSensor = SensorUtils.HasSensor(computer, SensorType.Fan) || SensorUtils.HasFanLikeControl(computer),
                AnyFanValue = SensorUtils.HasSensorValue(computer, SensorType.Fan) || SensorUtils.HasFanLikeControlWithValue(computer)
            };
        }
        
        /// <summary>
        /// 记录状态变更日志
        /// </summary>
        private static void LogStateChanges(ref bool? lastHasTempValue, ref bool? lastHasFanValue,
                                           bool anyTempValue, bool anyFanValue)
        {
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
        }
        
        /// <summary>
        /// 记录周期摘要日志
        /// </summary>
        private static void LogSummary(int tick, SensorData sensorData, DateTime lastGood)
        {
            int idleSec = (int)(DateTime.UtcNow - lastGood).TotalSeconds;
            int fansCount = 0;
            if (sensorData.Fans is System.Collections.ICollection fansCollection)
            {
                fansCount = fansCollection.Count;
            }
            ConfigurationManager.Log($"[summary] tick={tick} cpuTemp={ConfigurationManager.Fmt(sensorData.CpuTemp)} moboTemp={ConfigurationManager.Fmt(sensorData.MoboTemp)} fansCount={fansCount} hasTemp={sensorData.AnyTempSensor}/{sensorData.AnyTempValue} hasFan={sensorData.AnyFanSensor}/{sensorData.AnyFanValue} idleSec={idleSec}");
        }
        
        /// <summary>
        /// 构建输出数据载荷
        /// </summary>
        private static object BuildPayload(SensorData sensorData, DateTime startUtc, int tick, 
                                         int consecutiveExceptions, DateTime lastReopen, DateTime lastGood,
                                         Computer computer)
        {
            // 收集 CPU 包功耗/频率/限频标志
            var cpuExtra = DataCollector.CollectCpuExtra(computer);
            // 收集 CPU 每核心 负载/频率/温度
            var cpuPerCore = DataCollector.CollectCpuPerCore(computer);

            return new
            {
                cpuTempC = sensorData.CpuTemp,
                moboTempC = sensorData.MoboTemp,
                fans = (sensorData.Fans != null) ? sensorData.Fans : null,
                fansExtra = (sensorData.FansRaw != null) ? sensorData.FansRaw : null,
                moboVoltages = (sensorData.MoboVoltages != null) ? sensorData.MoboVoltages : null,
                storageTemps = (sensorData.StorageTemps != null) ? sensorData.StorageTemps : null,
                gpus = (sensorData.Gpus != null) ? sensorData.Gpus : null,
                isAdmin = IsAdmin(),
                hasTemp = sensorData.AnyTempSensor,
                hasTempValue = sensorData.AnyTempValue,
                hasFan = sensorData.AnyFanSensor,
                hasFanValue = sensorData.AnyFanValue,
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
                idleSec = (int)(DateTime.UtcNow - lastGood).TotalSeconds,
                excCount = consecutiveExceptions,
                uptimeSec = (int)(DateTime.UtcNow - startUtc).TotalSeconds,
                sinceReopenSec = (int)(DateTime.UtcNow - lastReopen).TotalSeconds
            };
        }
        
        /// <summary>
        /// 处理异常自愈
        /// </summary>
        private static Computer HandleExceptionSelfHeal(Computer computer, int excThreshold,
                                                        ref DateTime lastReopen, ref int consecutiveExceptions)
        {
            ConfigurationManager.Log($"[selfheal] consecutive exceptions >= {excThreshold}, reopening Computer...");
            try { computer.Close(); } catch { }
            computer = HardwareManager.MakeComputer();
            lastReopen = DateTime.UtcNow;
            consecutiveExceptions = 0;
            return computer;
        }
        
        /// <summary>
        /// 处理闲置和周期性自愈
        /// </summary>
        private static Computer HandleIdleAndPeriodicSelfHeal(Computer computer, DateTime lastGood, 
                                                              DateTime lastReopen, int idleSecThreshold, 
                                                              int periodicReopenSec, ref DateTime refLastReopen, 
                                                              ref DateTime refLastGood)
        {
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
                    computer = HardwareManager.MakeComputer();
                    refLastReopen = now;
                    refLastGood = now; // 避免立即再次触发
                }
            }
            catch { }
            return computer;
        }
        
        /// <summary>
        /// 获取最大循环次数（用于测试）
        /// </summary>
        private static int? GetMaxTicks()
        {
            try
            {
                var envTicks = Environment.GetEnvironmentVariable("BRIDGE_TICKS");
                if (!string.IsNullOrWhiteSpace(envTicks) && int.TryParse(envTicks, out var t) && t > 0)
                {
                    return t;
                }
            }
            catch { }
            return null;
        }
        
        /// <summary>
        /// 检查是否为管理员权限
        /// </summary>
        private static bool IsAdmin()
        {
            return new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator);
        }
    }
    
    /// <summary>
    /// 传感器数据容器
    /// </summary>
    internal class SensorData
    {
        public float? CpuTemp { get; set; }
        public float? MoboTemp { get; set; }
        public object? Fans { get; set; }
        public object? FansRaw { get; set; }
        public object? MoboVoltages { get; set; }
        public object? StorageTemps { get; set; }
        public object? Gpus { get; set; }
        public bool AnyTempSensor { get; set; }
        public bool AnyTempValue { get; set; }
        public bool AnyFanSensor { get; set; }
        public bool AnyFanValue { get; set; }
    }
}
