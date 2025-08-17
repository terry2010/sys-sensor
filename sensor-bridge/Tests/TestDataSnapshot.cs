using System;
using System.Collections.Generic;

namespace SensorBridge.Tests
{
    /// <summary>
    /// 测试数据快照 - 包含所有85个监控指标的数据结构
    /// </summary>
    public class TestDataSnapshot
    {
        // CPU相关指标 (12个)
        public double CpuUsage { get; set; }
        public float? CpuTempC { get; set; }
        public float? CpuPkgPowerW { get; set; }
        public float? CpuAvgFreqMhz { get; set; }
        public bool CpuThrottleActive { get; set; }
        public List<string> CpuThrottleReasons { get; set; } = new();
        public List<float> CpuCoreLoadsPct { get; set; } = new();
        public List<float> CpuCoreClocksMhz { get; set; } = new();
        public List<float> CpuCoreTempsc { get; set; } = new();
        public long HbTick { get; set; }
        public double IdleSec { get; set; }
        public int ExcCount { get; set; }

        // 内存相关指标 (12个)
        public double MemUsedGb { get; set; }
        public double MemTotalGb { get; set; }
        public double MemPct { get; set; }
        public double? MemAvailGb { get; set; }
        public double? SwapUsedGb { get; set; }
        public double? SwapTotalGb { get; set; }
        public double? MemCacheGb { get; set; }
        public double? MemCommittedGb { get; set; }
        public double? MemCommitLimitGb { get; set; }
        public double? MemPoolPagedGb { get; set; }
        public double? MemPoolNonpagedGb { get; set; }
        public double? MemPagesPerSec { get; set; }

        // 网络相关指标 (15个)
        public long NetRxBps { get; set; }
        public long NetTxBps { get; set; }
        public long NetRxInstantBps { get; set; }
        public long NetTxInstantBps { get; set; }
        public double? NetRxErrPs { get; set; }
        public double? NetTxErrPs { get; set; }
        public double? PacketLossPct { get; set; }
        public int? ActiveConnections { get; set; }
        public double? PingRttMs { get; set; }
        public List<double> RttMulti { get; set; } = new();
        public string? WifiSsid { get; set; }
        public int? WifiSignalPct { get; set; }
        public int? WifiLinkMbps { get; set; }
        public string? WifiBssid { get; set; }
        public int? WifiChannel { get; set; }

        // 存储相关指标 (8个)
        public double DiskUsedGb { get; set; }
        public double DiskTotalGb { get; set; }
        public double DiskPct { get; set; }
        public long DiskReadBps { get; set; }
        public long DiskWriteBps { get; set; }
        public double? DiskQueueLen { get; set; }
        public double? DiskActivePct { get; set; }
        public double? DiskRespMs { get; set; }

        // GPU相关指标 (8个)
        public List<GpuInfo> Gpus { get; set; } = new();

        // 系统其他指标 (30个)
        public double? UptimeSec { get; set; }
        public long UptimeMs { get; set; }
        public int? ProcessCount { get; set; }
        public List<TestProcessInfo> TopProcs { get; set; } = new();
        public int? BatteryPct { get; set; }
        public string? BatteryStatus { get; set; }
        public double? BatteryHealthPct { get; set; }
        public double? BatteryCapacityWh { get; set; }
        public double? BatteryDesignCapacityWh { get; set; }
        public long? BatteryTimeToEmptySec { get; set; }
        public long? BatteryTimeToFullSec { get; set; }
        public List<FanInfo> Fans { get; set; } = new();
        public List<FanInfo> FansExtra { get; set; } = new();
        public List<VoltageInfo> MoboVoltages { get; set; } = new();
        public float? MoboTempC { get; set; }
        public long TimestampMs { get; set; }

        // 附加指标
        public double? SinceReopenSec { get; set; }
        public double? MemPageReadsPerSec { get; set; }
        public double? MemPageWritesPerSec { get; set; }
        public double? MemPageFaultsPerSec { get; set; }
        public string? WifiBand { get; set; }
        public string? WifiRadio { get; set; }
        public double? WifiRxMbps { get; set; }
        public double? WifiTxMbps { get; set; }
        public int? WifiRssiDbm { get; set; }
        public bool? WifiRssiEstimated { get; set; }
        public string? WifiAuth { get; set; }
        public string? WifiCipher { get; set; }
        public int? WifiChanWidthMhz { get; set; }
        public List<TestNetInterface> NetIfs { get; set; } = new();
        public string? PublicIp { get; set; }
        public string? Isp { get; set; }
        public List<TestSmartDisk> SmartHealth { get; set; } = new();
        public float? DiskTempC { get; set; }
        public List<TestDiskInfo> Disks { get; set; } = new();
    }

    /// <summary>
    /// 测试进程信息
    /// </summary>
    public class TestProcessInfo
    {
        public string? Name { get; set; }
        public double? CpuPct { get; set; }
        public double? MemMb { get; set; }
        public int? Pid { get; set; }
    }

    /// <summary>
    /// 测试网络接口信息
    /// </summary>
    public class TestNetInterface
    {
        public string? Name { get; set; }
        public string? Type { get; set; }
        public bool? IsUp { get; set; }
        public long? RxBps { get; set; }
        public long? TxBps { get; set; }
    }

    /// <summary>
    /// 测试SMART磁盘信息
    /// </summary>
    public class TestSmartDisk
    {
        public string? Name { get; set; }
        public string? Health { get; set; }
        public int? TempC { get; set; }
        public long? PowerOnHours { get; set; }
    }

    /// <summary>
    /// 测试磁盘信息
    /// </summary>
    public class TestDiskInfo
    {
        public string? Name { get; set; }
        public string? Type { get; set; }
        public double? SizeGb { get; set; }
        public double? UsedGb { get; set; }
        public double? UsagePct { get; set; }
    }
}
