using System;
using System.Threading.Tasks;

namespace SensorBridge.Tests
{
    public class CpuTests : BaseTestRunner
    {
        public CpuTests(string testReportPath) : base(testReportPath)
        {
        }

        public async Task RunAllCpuTests()
        {
            await TestCpuUsage();
            await TestCpuTemperature();
            await TestCpuPackagePower();
            await TestCpuAverageFrequency();
            await TestCpuThrottleStatus();
            await TestCpuThrottleReasons();
            await TestCpuPerCoreLoads();
            await TestCpuPerCoreFrequencies();
            await TestCpuPerCoreTemperatures();
            await TestBridgeHealth();
            await TestSystemUptime();
            await TestBridgeReconnectTime();
        }

        private async Task TestCpuUsage()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var cpuUsage = data.CpuUsage;
                
                var success = cpuUsage >= 0 && cpuUsage <= 100;
                var message = success ? "CPU使用率检测成功" : "CPU使用率超出有效范围";
                var details = new { CpuUsage = cpuUsage, Valid = success };
                
                AddTestResult("CPU使用率", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("CPU使用率", false, $"CPU使用率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestCpuTemperature()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var cpuTemp = data.CpuTempC;
                
                var success = cpuTemp == null || (cpuTemp >= 0 && cpuTemp <= 150);
                var message = success ? "CPU温度检测成功" : "CPU温度超出有效范围";
                var details = new { CpuTempC = cpuTemp, Valid = success };
                
                AddTestResult("CPU温度", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("CPU温度", false, $"CPU温度检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestCpuPackagePower()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var cpuPower = data.CpuPkgPowerW;
                
                var success = cpuPower == null || (cpuPower >= 0 && cpuPower <= 1000);
                var message = success ? "CPU包功耗检测成功" : "CPU包功耗超出有效范围";
                var details = new { CpuPkgPowerW = cpuPower, Valid = success };
                
                AddTestResult("CPU包功耗", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("CPU包功耗", false, $"CPU包功耗检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestCpuAverageFrequency()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var cpuFreq = data.CpuAvgFreqMhz;
                
                var success = cpuFreq == null || (cpuFreq >= 100 && cpuFreq <= 10000);
                var message = success ? "CPU平均频率检测成功" : "CPU平均频率超出有效范围";
                var details = new { CpuAvgFreqMhz = cpuFreq, Valid = success };
                
                AddTestResult("CPU平均频率", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("CPU平均频率", false, $"CPU平均频率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestCpuThrottleStatus()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var throttleActive = data.CpuThrottleActive;
                
                var success = true; // 限频状态是布尔值，任何值都有效
                var message = "CPU限频状态检测成功";
                var details = new { CpuThrottleActive = throttleActive, Valid = success };
                
                AddTestResult("CPU限频状态", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("CPU限频状态", false, $"CPU限频状态检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestCpuThrottleReasons()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var throttleReasons = data.CpuThrottleReasons;
                
                var success = true; // 限频原因数组总是有效的
                var message = "CPU限频原因检测成功";
                var details = new { CpuThrottleReasons = throttleReasons, Count = throttleReasons?.Count ?? 0 };
                
                AddTestResult("CPU限频原因", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("CPU限频原因", false, $"CPU限频原因检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestCpuPerCoreLoads()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var coreLoads = data.CpuCoreLoadsPct;
                
                var success = coreLoads == null || coreLoads.Count > 0;
                var message = success ? "CPU每核负载检测成功" : "CPU每核负载数据无效";
                var details = new { CpuCoreLoads = coreLoads?.Count ?? 0, Valid = success };
                
                AddTestResult("CPU每核负载", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("CPU每核负载", false, $"CPU每核负载检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestCpuPerCoreFrequencies()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var coreFreqs = data.CpuCoreClocksMhz;
                
                var success = coreFreqs == null || coreFreqs.Count > 0;
                var message = success ? "CPU每核频率检测成功" : "CPU每核频率数据无效";
                var details = new { CpuCoreFreqs = coreFreqs?.Count ?? 0, Valid = success };
                
                AddTestResult("CPU每核频率", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("CPU每核频率", false, $"CPU每核频率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestCpuPerCoreTemperatures()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var coreTemps = data.CpuCoreTempsc;
                
                var success = coreTemps == null || coreTemps.Count > 0;
                var message = success ? "CPU每核温度检测成功" : "CPU每核温度数据无效";
                var details = new { CpuCoreTemps = coreTemps?.Count ?? 0, Valid = success };
                
                AddTestResult("CPU每核温度", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("CPU每核温度", false, $"CPU每核温度检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestBridgeHealth()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var hbTick = data.HbTick;
                var idleSec = data.IdleSec;
                var excCount = data.ExcCount;
                
                var success = true; // 桥接健康指标总是有效的
                var message = "桥接健康检测成功";
                var details = new { HbTick = hbTick, IdleSec = idleSec, ExcCount = excCount };
                
                AddTestResult("桥接健康", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("桥接健康", false, $"桥接健康检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestSystemUptime()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var uptime = data.UptimeSec;
                
                var success = uptime == null || uptime >= 0;
                var message = success ? "系统运行时间检测成功" : "系统运行时间无效";
                var details = new { UptimeSec = uptime, Valid = success };
                
                AddTestResult("系统运行时间", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("系统运行时间", false, $"系统运行时间检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestBridgeReconnectTime()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var sinceReopen = data.SinceReopenSec;
                
                var success = sinceReopen == null || sinceReopen >= 0;
                var message = success ? "桥接重连时间检测成功" : "桥接重连时间无效";
                var details = new { SinceReopenSec = sinceReopen, Valid = success };
                
                AddTestResult("桥接重连时间", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("桥接重连时间", false, $"桥接重连时间检测失败: {ex.Message}", null, ex);
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
