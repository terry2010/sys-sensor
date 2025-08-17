using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using System.Linq;
using System.Management;
using LibreHardwareMonitor.Hardware;

namespace SensorBridge.Tests
{
    /// <summary>
    /// 测试用数据收集器 - 模拟实际数据收集过程
    /// </summary>
    public static class TestDataCollector
    {
        public static async Task<TestDataSnapshot> CollectDataAsync()
        {
            return await Task.Run(() =>
            {
                var computer = HardwareManager.MakeComputer();
                
                try
                {
                    // 更新硬件数据
                    foreach (var hardware in computer.Hardware)
                    {
                        hardware.Update();
                        foreach (var subHardware in hardware.SubHardware)
                        {
                            subHardware.Update();
                        }
                    }

                    var snapshot = new TestDataSnapshot();

                    // CPU相关数据
                    CollectCpuData(computer, snapshot);
                    
                    // 内存相关数据
                    CollectMemoryData(computer, snapshot);
                    
                    // GPU相关数据
                    CollectGpuData(computer, snapshot);
                    
                    // 存储相关数据
                    CollectStorageData(computer, snapshot);
                    
                    // 网络相关数据
                    CollectNetworkData(computer, snapshot);
                    
                    // 系统其他数据
                    CollectSystemData(computer, snapshot);

                    return snapshot;
                }
                finally
                {
                    computer.Close();
                }
            });
        }

        private static void CollectCpuData(IComputer computer, TestDataSnapshot snapshot)
        {
            try
            {
                // CPU使用率
                snapshot.CpuUsage = GetCpuUsage(computer);
                
                // CPU温度
                snapshot.CpuTempC = DataCollector.PickCpuTemperature(computer);
                
                // CPU功耗
                var cpuExtra = DataCollector.CollectCpuExtra(computer);
                snapshot.CpuPkgPowerW = (float?)cpuExtra?.PkgPowerW;
                
                // CPU频率
                snapshot.CpuAvgFreqMhz = (float?)cpuExtra?.AvgCoreMhz;
                
                // CPU限频信息
                snapshot.CpuThrottleActive = cpuExtra?.ThrottleActive ?? false;
                snapshot.CpuThrottleReasons = cpuExtra?.ThrottleReasons ?? new List<string>();
                
                // CPU每核心数据
                var perCore = DataCollector.CollectCpuPerCore(computer);
                snapshot.CpuCoreLoadsPct = perCore?.Loads?.Where(x => x.HasValue).Select(x => x.Value).ToList() ?? new List<float>();
                snapshot.CpuCoreClocksMhz = perCore?.ClocksMhz?.Where(x => x.HasValue).Select(x => (float)x.Value).ToList() ?? new List<float>();
                snapshot.CpuCoreTempsc = perCore?.TempsC?.Where(x => x.HasValue).Select(x => x.Value).ToList() ?? new List<float>();
                
                // 系统运行时间
                snapshot.UptimeSec = Environment.TickCount64 / 1000.0;
                snapshot.UptimeMs = Environment.TickCount64;
                
                // 桥接健康数据
                snapshot.HbTick = Environment.TickCount64;
                snapshot.IdleSec = 0;
                snapshot.ExcCount = 0;
                snapshot.SinceReopenSec = 0;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"CPU数据收集错误: {ex.Message}");
            }
        }

        private static void CollectMemoryData(IComputer computer, TestDataSnapshot snapshot)
        {
            try
            {
                // 获取系统内存信息
                var memInfo = new System.Management.ManagementObjectSearcher("SELECT * FROM Win32_OperatingSystem");
                var memObjects = memInfo.Get();
                var totalMem = 0UL;
                var availMem = 0UL;
                
                foreach (var obj in memObjects)
                {
                    totalMem = Convert.ToUInt64(obj["TotalVisibleMemorySize"]) * 1024;
                    availMem = Convert.ToUInt64(obj["FreePhysicalMemory"]) * 1024;
                    break;
                }
                
                var usedMem = totalMem - availMem;

                snapshot.MemTotalGb = totalMem / (1024.0 * 1024 * 1024);
                snapshot.MemUsedGb = usedMem / (1024.0 * 1024 * 1024);
                snapshot.MemAvailGb = availMem / (1024.0 * 1024 * 1024);
                snapshot.MemPct = (usedMem * 100.0) / totalMem;

                // 其他内存指标（模拟数据）
                snapshot.SwapUsedGb = 0;
                snapshot.SwapTotalGb = 0;
                snapshot.MemCacheGb = snapshot.MemUsedGb * 0.2; // 估算缓存
                snapshot.MemCommittedGb = snapshot.MemUsedGb * 1.1;
                snapshot.MemCommitLimitGb = snapshot.MemTotalGb * 1.5;
                snapshot.MemPoolPagedGb = 0.1;
                snapshot.MemPoolNonpagedGb = 0.05;
                snapshot.MemPagesPerSec = 100;
                snapshot.MemPageReadsPerSec = 50;
                snapshot.MemPageWritesPerSec = 30;
                snapshot.MemPageFaultsPerSec = 1000;

                // 主板温度
                snapshot.MoboTempC = DataCollector.PickMotherboardTemperature(computer);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"内存数据收集错误: {ex.Message}");
            }
        }

        private static void CollectGpuData(IComputer computer, TestDataSnapshot snapshot)
        {
            try
            {
                snapshot.Gpus = DataCollector.CollectGpus(computer);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"GPU数据收集错误: {ex.Message}");
            }
        }

        private static void CollectStorageData(IComputer computer, TestDataSnapshot snapshot)
        {
            try
            {
                // 磁盘使用量（模拟数据）
                var drives = System.IO.DriveInfo.GetDrives();
                var totalSize = 0L;
                var usedSize = 0L;
                
                foreach (var drive in drives)
                {
                    if (drive.IsReady)
                    {
                        totalSize += drive.TotalSize;
                        usedSize += drive.TotalSize - drive.AvailableFreeSpace;
                    }
                }

                snapshot.DiskTotalGb = totalSize / (1024.0 * 1024 * 1024);
                snapshot.DiskUsedGb = usedSize / (1024.0 * 1024 * 1024);
                snapshot.DiskPct = totalSize > 0 ? (usedSize * 100.0) / totalSize : 0;

                // 磁盘性能指标（模拟数据）
                snapshot.DiskReadBps = 1000000; // 1MB/s
                snapshot.DiskWriteBps = 500000;  // 0.5MB/s
                snapshot.DiskQueueLen = 1.0;
                snapshot.DiskActivePct = 10.0;
                snapshot.DiskRespMs = 5.0;

                // 存储温度和SMART
                snapshot.SmartHealth = new List<TestSmartDisk>();
                snapshot.DiskTempC = 35.0f;
                snapshot.Disks = new List<TestDiskInfo>();
            }
            catch (Exception ex)
            {
                Console.WriteLine($"存储数据收集错误: {ex.Message}");
            }
        }

        private static void CollectNetworkData(IComputer computer, TestDataSnapshot snapshot)
        {
            try
            {
                // 网络速率（模拟数据）
                snapshot.NetRxBps = 1000000;
                snapshot.NetTxBps = 500000;
                snapshot.NetRxInstantBps = 1200000;
                snapshot.NetTxInstantBps = 600000;
                snapshot.NetRxErrPs = 0;
                snapshot.NetTxErrPs = 0;
                snapshot.PacketLossPct = 0.1;
                snapshot.ActiveConnections = 50;
                snapshot.PingRttMs = 20.0;
                snapshot.RttMulti = new List<double> { 20.0, 25.0, 18.0 };

                // WiFi信息（模拟数据）
                snapshot.WifiSsid = "TestNetwork";
                snapshot.WifiSignalPct = 80;
                snapshot.WifiLinkMbps = 100;
                snapshot.WifiBssid = "00:11:22:33:44:55";
                snapshot.WifiChannel = 6;
                snapshot.WifiBand = "2.4GHz";
                snapshot.WifiRadio = "802.11n";
                snapshot.WifiRxMbps = 50;
                snapshot.WifiTxMbps = 30;
                snapshot.WifiRssiDbm = -45;
                snapshot.WifiRssiEstimated = false;
                snapshot.WifiAuth = "WPA2";
                snapshot.WifiCipher = "AES";
                snapshot.WifiChanWidthMhz = 40;

                // 网络接口
                snapshot.NetIfs = new List<TestNetInterface>();
                
                // 公网信息
                snapshot.PublicIp = "192.168.1.100";
                snapshot.Isp = "Test ISP";
            }
            catch (Exception ex)
            {
                Console.WriteLine($"网络数据收集错误: {ex.Message}");
            }
        }

        private static void CollectSystemData(IComputer computer, TestDataSnapshot snapshot)
        {
            try
            {
                // 进程信息
                var processes = System.Diagnostics.Process.GetProcesses();
                snapshot.ProcessCount = processes.Length;
                snapshot.TopProcs = new List<TestProcessInfo>();

                // 电池信息（模拟数据）
                snapshot.BatteryPct = 85;
                snapshot.BatteryStatus = "Discharging";
                snapshot.BatteryHealthPct = 90;
                snapshot.BatteryCapacityWh = 50.0;
                snapshot.BatteryDesignCapacityWh = 55.0;
                snapshot.BatteryTimeToEmptySec = 7200;
                snapshot.BatteryTimeToFullSec = null;

                // 风扇信息
                snapshot.Fans = DataCollector.CollectFans(computer);
                snapshot.FansExtra = DataCollector.CollectFansRaw(computer);

                // 主板电压
                snapshot.MoboVoltages = DataCollector.CollectMoboVoltages(computer);

                // 时间戳
                snapshot.TimestampMs = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            }
            catch (Exception ex)
            {
                Console.WriteLine($"系统数据收集错误: {ex.Message}");
            }
        }

        private static double GetCpuUsage(IComputer computer)
        {
            try
            {
                foreach (var hardware in computer.Hardware)
                {
                    if (hardware.HardwareType == HardwareType.Cpu)
                    {
                        foreach (var sensor in hardware.Sensors)
                        {
                            if (sensor.SensorType == SensorType.Load && 
                                sensor.Name.Contains("Total") && 
                                sensor.Value.HasValue)
                            {
                                return sensor.Value.Value;
                            }
                        }
                    }
                }
                return 15.0; // 默认值
            }
            catch
            {
                return 15.0; // 默认值
            }
        }
    }
}
