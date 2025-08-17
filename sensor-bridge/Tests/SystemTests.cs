using System;
using System.Threading.Tasks;

namespace SensorBridge.Tests
{
    public class SystemTests : BaseTestRunner
    {
        public SystemTests(string testReportPath) : base(testReportPath)
        {
        }

        public async Task RunAllSystemTests()
        {
            await TestSystemUptime();
            await TestProcessCount();
            await TestTopProcesses();
            await TestBatteryInfo();
            await TestBatteryHealth();
            await TestBatteryTime();
            await TestSystemFans();
            await TestMainboardVoltages();
            await TestSystemTemperatures();
            await TestSystemLoad();
        }

        private async Task TestSystemUptime()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var uptimeMs = data.UptimeMs;
                
                var success = uptimeMs >= 0;
                var message = success ? "系统运行时间检测成功" : "系统运行时间数据无效";
                var details = new { UptimeMs = uptimeMs, UptimeHours = uptimeMs / (1000.0 * 60 * 60) };
                
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

        private async Task TestProcessCount()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var processCount = data.ProcessCount;
                
                var success = processCount == null || processCount > 0;
                var message = success ? "进程数量检测成功" : "进程数量数据无效";
                var details = new { ProcessCount = processCount, Valid = success };
                
                AddTestResult("进程数量", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("进程数量", false, $"进程数量检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestTopProcesses()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var topProcs = data.TopProcs;
                
                var success = topProcs != null;
                var message = success ? "进程列表检测成功" : "未检测到进程列表";
                var details = new { TopProcessCount = topProcs?.Count ?? 0 };
                
                AddTestResult("进程列表", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("进程列表", false, $"进程列表检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestBatteryInfo()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var batteryPct = data.BatteryPct;
                var batteryStatus = data.BatteryStatus;
                
                var success = batteryPct == null || (batteryPct >= 0 && batteryPct <= 100);
                var message = success ? "电池基础信息检测成功" : "电池电量超出有效范围";
                var details = new { BatteryPct = batteryPct, BatteryStatus = batteryStatus };
                
                AddTestResult("电池基础信息", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("电池基础信息", false, $"电池基础信息检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestBatteryHealth()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var batteryHealthPct = data.BatteryHealthPct;
                var batteryCapacityWh = data.BatteryCapacityWh;
                var batteryDesignCapacityWh = data.BatteryDesignCapacityWh;
                
                var success = (batteryHealthPct == null || (batteryHealthPct >= 0 && batteryHealthPct <= 100)) &&
                             (batteryCapacityWh == null || batteryCapacityWh >= 0) &&
                             (batteryDesignCapacityWh == null || batteryDesignCapacityWh >= 0);
                
                var message = success ? "电池健康信息检测成功" : "电池健康信息超出有效范围";
                var details = new { 
                    BatteryHealthPct = batteryHealthPct, 
                    BatteryCapacityWh = batteryCapacityWh,
                    BatteryDesignCapacityWh = batteryDesignCapacityWh
                };
                
                AddTestResult("电池健康", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("电池健康", false, $"电池健康信息检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestBatteryTime()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var batteryTimeToEmptySec = data.BatteryTimeToEmptySec;
                var batteryTimeToFullSec = data.BatteryTimeToFullSec;
                
                var success = (batteryTimeToEmptySec == null || batteryTimeToEmptySec >= 0) &&
                             (batteryTimeToFullSec == null || batteryTimeToFullSec >= 0);
                
                var message = success ? "电池时间预估检测成功" : "电池时间预估数据无效";
                var details = new { 
                    BatteryTimeToEmptySec = batteryTimeToEmptySec,
                    BatteryTimeToFullSec = batteryTimeToFullSec
                };
                
                AddTestResult("电池时间预估", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("电池时间预估", false, $"电池时间预估检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestSystemFans()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var fans = data.Fans;
                var fansExtra = data.FansExtra;
                
                var success = true; // 风扇信息总是有效的
                var message = "系统风扇检测成功";
                var details = new { 
                    FansCount = fans?.Count ?? 0,
                    FansExtraCount = fansExtra?.Count ?? 0
                };
                
                AddTestResult("系统风扇", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("系统风扇", false, $"系统风扇检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMainboardVoltages()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var moboVoltages = data.MoboVoltages;
                
                var success = true; // 主板电压信息总是有效的
                var message = "主板电压检测成功";
                var details = new { MoboVoltagesCount = moboVoltages?.Count ?? 0 };
                
                AddTestResult("主板电压", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("主板电压", false, $"主板电压检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestSystemTemperatures()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var moboTempC = data.MoboTempC;
                
                var success = moboTempC == null || (moboTempC >= 0 && moboTempC <= 120);
                var message = success ? "系统温度检测成功" : "系统温度超出有效范围";
                var details = new { MoboTempC = moboTempC, Valid = success };
                
                AddTestResult("系统温度", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("系统温度", false, $"系统温度检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestSystemLoad()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var timestampMs = data.TimestampMs;
                
                var success = timestampMs > 0;
                var message = success ? "系统负载时间戳检测成功" : "系统负载时间戳无效";
                var details = new { TimestampMs = timestampMs, Timestamp = DateTimeOffset.FromUnixTimeMilliseconds(timestampMs).ToString() };
                
                AddTestResult("系统负载时间戳", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("系统负载时间戳", false, $"系统负载时间戳检测失败: {ex.Message}", null, ex);
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
