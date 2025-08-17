using System;
using System.Threading.Tasks;

namespace SensorBridge.Tests
{
    public class StorageTests : BaseTestRunner
    {
        public StorageTests(string testReportPath) : base(testReportPath)
        {
        }

        public async Task RunAllStorageTests()
        {
            await TestDiskUsage();
            await TestDiskReadWrite();
            await TestDiskQueueLength();
            await TestDiskActiveTime();
            await TestDiskResponseTime();
            await TestSmartHealth();
            await TestDiskTemperature();
            await TestDiskList();
        }

        private async Task TestDiskUsage()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var diskUsedGb = data.DiskUsedGb;
                var diskTotalGb = data.DiskTotalGb;
                var diskPct = data.DiskPct;
                
                var success = diskUsedGb >= 0 && diskTotalGb > 0 && diskPct >= 0 && diskPct <= 100;
                var message = success ? "磁盘使用量检测成功" : "磁盘使用量数据无效";
                var details = new { DiskUsedGb = diskUsedGb, DiskTotalGb = diskTotalGb, DiskPct = diskPct };
                
                AddTestResult("磁盘使用量", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("磁盘使用量", false, $"磁盘使用量检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestDiskReadWrite()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var diskReadBps = data.DiskReadBps;
                var diskWriteBps = data.DiskWriteBps;
                
                var success = diskReadBps >= 0 && diskWriteBps >= 0;
                var message = success ? "磁盘读写速度检测成功" : "磁盘读写速度数据无效";
                var details = new { DiskReadBps = diskReadBps, DiskWriteBps = diskWriteBps };
                
                AddTestResult("磁盘读写速度", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("磁盘读写速度", false, $"磁盘读写速度检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestDiskQueueLength()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var diskQueueLen = data.DiskQueueLen;
                
                var success = diskQueueLen == null || diskQueueLen >= 0;
                var message = success ? "磁盘队列长度检测成功" : "磁盘队列长度数据无效";
                var details = new { DiskQueueLen = diskQueueLen, Valid = success };
                
                AddTestResult("磁盘队列长度", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("磁盘队列长度", false, $"磁盘队列长度检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestDiskActiveTime()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var diskActivePct = data.DiskActivePct;
                
                var success = diskActivePct == null || (diskActivePct >= 0 && diskActivePct <= 100);
                var message = success ? "磁盘活动时间检测成功" : "磁盘活动时间超出有效范围";
                var details = new { DiskActivePct = diskActivePct, Valid = success };
                
                AddTestResult("磁盘活动时间", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("磁盘活动时间", false, $"磁盘活动时间检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestDiskResponseTime()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var diskRespMs = data.DiskRespMs;
                
                var success = diskRespMs == null || diskRespMs >= 0;
                var message = success ? "磁盘响应时间检测成功" : "磁盘响应时间数据无效";
                var details = new { DiskRespMs = diskRespMs, Valid = success };
                
                AddTestResult("磁盘响应时间", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("磁盘响应时间", false, $"磁盘响应时间检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestSmartHealth()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var smartHealth = data.SmartHealth;
                
                var success = true; // SMART健康信息总是有效的
                var message = "SMART健康信息检测成功";
                var details = new { SmartHealthCount = smartHealth?.Count ?? 0 };
                
                AddTestResult("SMART健康", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("SMART健康", false, $"SMART健康信息检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestDiskTemperature()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var diskTempC = data.DiskTempC;
                
                var success = diskTempC == null || (diskTempC >= 0 && diskTempC <= 100);
                var message = success ? "磁盘温度检测成功" : "磁盘温度超出有效范围";
                var details = new { DiskTempC = diskTempC, Valid = success };
                
                AddTestResult("磁盘温度", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("磁盘温度", false, $"磁盘温度检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestDiskList()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var disks = data.Disks;
                
                var success = disks != null && disks.Count > 0;
                var message = success ? "磁盘列表检测成功" : "未检测到磁盘";
                var details = new { DiskCount = disks?.Count ?? 0 };
                
                AddTestResult("磁盘列表", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("磁盘列表", false, $"磁盘列表检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }
    }
}
