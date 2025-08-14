using System.Collections.Generic;

namespace SensorBridge
{
    /// <summary>
    /// 存储温度数据模型
    /// </summary>
    public class StorageTemp
    {
        public string? Name { get; set; }
        public float? TempC { get; set; }
    }

    /// <summary>
    /// CPU 每核心指标数据模型
    /// </summary>
    public class CpuPerCore
    {
        public List<float?> Loads { get; set; } = new List<float?>();
        public List<double?> ClocksMhz { get; set; } = new List<double?>();
        public List<float?> TempsC { get; set; } = new List<float?>();
    }

    /// <summary>
    /// 风扇信息数据模型
    /// </summary>
    public class FanInfo
    {
        public string? Name { get; set; }
        public int? Rpm { get; set; }
        public int? Pct { get; set; }
    }

    /// <summary>
    /// 主板电压信息数据模型
    /// </summary>
    public class VoltageInfo
    {
        public string? Name { get; set; }
        public double? Volts { get; set; }
    }

    /// <summary>
    /// GPU 信息数据模型
    /// </summary>
    public class GpuInfo
    {
        public string? Name { get; set; }
        public float? TempC { get; set; }
        public float? LoadPct { get; set; }
        public double? CoreMhz { get; set; }
        public double? MemoryMhz { get; set; }
        public int? FanRpm { get; set; }
        public int? FanDutyPct { get; set; }
        public double? VramUsedMb { get; set; }
        public double? PowerW { get; set; }
        public double? PowerLimitW { get; set; }
        public double? VoltageV { get; set; }
        public float? HotspotTempC { get; set; }
        public float? VramTempC { get; set; }
    }

    /// <summary>
    /// CPU 额外信息数据模型（第二梯队）
    /// </summary>
    public class CpuExtra
    {
        public double? PkgPowerW { get; set; }
        public double? AvgCoreMhz { get; set; }
        public bool? ThrottleActive { get; set; }
        public List<string>? ThrottleReasons { get; set; }
    }
}
