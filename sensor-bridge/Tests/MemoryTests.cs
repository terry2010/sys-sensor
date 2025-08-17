using System;
using System.Threading.Tasks;

namespace SensorBridge.Tests
{
    public class MemoryTests : BaseTestRunner
    {
        public MemoryTests(string testReportPath) : base(testReportPath)
        {
        }

        public async Task RunAllMemoryTests()
        {
            await TestMemoryUsage();
            await TestMemoryAvailable();
            await TestSwapMemory();
            await TestMemoryCache();
            await TestMemoryCommitted();
            await TestMemoryPagedPool();
            await TestMemoryNonPagedPool();
            await TestMemoryPagingRate();
            await TestMemoryPageReads();
            await TestMemoryPageWrites();
            await TestMemoryPageFaults();
            await TestMotherboardTemperature();
        }

        private async Task TestMemoryUsage()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var memUsed = data.MemUsedGb;
                var memTotal = data.MemTotalGb;
                var memPct = data.MemPct;
                
                var success = memUsed >= 0 && memTotal > 0 && memPct >= 0 && memPct <= 100;
                var message = success ? "内存使用检测成功" : "内存使用数据无效";
                var details = new { MemUsedGb = memUsed, MemTotalGb = memTotal, MemPct = memPct };
                
                AddTestResult("内存使用", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("内存使用", false, $"内存使用检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMemoryAvailable()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var memAvail = data.MemAvailGb;
                
                var success = memAvail == null || memAvail >= 0;
                var message = success ? "内存可用量检测成功" : "内存可用量数据无效";
                var details = new { MemAvailGb = memAvail, Valid = success };
                
                AddTestResult("内存可用", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("内存可用", false, $"内存可用量检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestSwapMemory()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var swapUsed = data.SwapUsedGb;
                var swapTotal = data.SwapTotalGb;
                
                var success = (swapUsed == null || swapUsed >= 0) && (swapTotal == null || swapTotal >= 0);
                var message = success ? "交换区检测成功" : "交换区数据无效";
                var details = new { SwapUsedGb = swapUsed, SwapTotalGb = swapTotal };
                
                AddTestResult("交换区", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("交换区", false, $"交换区检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMemoryCache()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var memCache = data.MemCacheGb;
                
                var success = memCache == null || memCache >= 0;
                var message = success ? "内存缓存检测成功" : "内存缓存数据无效";
                var details = new { MemCacheGb = memCache, Valid = success };
                
                AddTestResult("内存缓存", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("内存缓存", false, $"内存缓存检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMemoryCommitted()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var memCommitted = data.MemCommittedGb;
                var memCommitLimit = data.MemCommitLimitGb;
                
                var success = (memCommitted == null || memCommitted >= 0) && (memCommitLimit == null || memCommitLimit >= 0);
                var message = success ? "内存提交检测成功" : "内存提交数据无效";
                var details = new { MemCommittedGb = memCommitted, MemCommitLimitGb = memCommitLimit };
                
                AddTestResult("内存提交", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("内存提交", false, $"内存提交检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMemoryPagedPool()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var memPoolPaged = data.MemPoolPagedGb;
                
                var success = memPoolPaged == null || memPoolPaged >= 0;
                var message = success ? "分页池检测成功" : "分页池数据无效";
                var details = new { MemPoolPagedGb = memPoolPaged, Valid = success };
                
                AddTestResult("分页池", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("分页池", false, $"分页池检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMemoryNonPagedPool()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var memPoolNonPaged = data.MemPoolNonpagedGb;
                
                var success = memPoolNonPaged == null || memPoolNonPaged >= 0;
                var message = success ? "非分页池检测成功" : "非分页池数据无效";
                var details = new { MemPoolNonpagedGb = memPoolNonPaged, Valid = success };
                
                AddTestResult("非分页池", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("非分页池", false, $"非分页池检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMemoryPagingRate()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var memPagesPerSec = data.MemPagesPerSec;
                
                var success = memPagesPerSec == null || memPagesPerSec >= 0;
                var message = success ? "分页速率检测成功" : "分页速率数据无效";
                var details = new { MemPagesPerSec = memPagesPerSec, Valid = success };
                
                AddTestResult("分页速率", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("分页速率", false, $"分页速率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMemoryPageReads()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var memPageReads = data.MemPageReadsPerSec;
                
                var success = memPageReads == null || memPageReads >= 0;
                var message = success ? "页面读取检测成功" : "页面读取数据无效";
                var details = new { MemPageReadsPerSec = memPageReads, Valid = success };
                
                AddTestResult("页面读取", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("页面读取", false, $"页面读取检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMemoryPageWrites()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var memPageWrites = data.MemPageWritesPerSec;
                
                var success = memPageWrites == null || memPageWrites >= 0;
                var message = success ? "页面写入检测成功" : "页面写入数据无效";
                var details = new { MemPageWritesPerSec = memPageWrites, Valid = success };
                
                AddTestResult("页面写入", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("页面写入", false, $"页面写入检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMemoryPageFaults()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var memPageFaults = data.MemPageFaultsPerSec;
                
                var success = memPageFaults == null || memPageFaults >= 0;
                var message = success ? "页面错误检测成功" : "页面错误数据无效";
                var details = new { MemPageFaultsPerSec = memPageFaults, Valid = success };
                
                AddTestResult("页面错误", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("页面错误", false, $"页面错误检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMotherboardTemperature()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var moboTemp = data.MoboTempC;
                
                var success = moboTemp == null || (moboTemp >= 0 && moboTemp <= 150);
                var message = success ? "主板温度检测成功" : "主板温度超出有效范围";
                var details = new { MoboTempC = moboTemp, Valid = success };
                
                AddTestResult("主板温度", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("主板温度", false, $"主板温度检测失败: {ex.Message}", null, ex);
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
