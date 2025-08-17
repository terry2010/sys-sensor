using System;
using System.Threading.Tasks;

namespace SensorBridge.Tests
{
    public class GpuTests : BaseTestRunner
    {
        public GpuTests(string testReportPath) : base(testReportPath)
        {
        }

        public async Task RunAllGpuTests()
        {
            await TestGpuBasicInfo();
            await TestGpuUsage();
            await TestGpuTemperature();
            await TestGpuMemory();
            await TestGpuClockSpeeds();
            await TestGpuFanSpeed();
            await TestGpuPower();
            await TestGpuVoltage();
        }

        private async Task TestGpuBasicInfo()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var gpus = data.Gpus;
                
                var success = gpus != null;
                var message = success ? "GPU基础信息检测成功" : "未检测到GPU信息";
                var details = new { GpuCount = gpus?.Count ?? 0 };
                
                AddTestResult("GPU基础信息", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("GPU基础信息", false, $"GPU基础信息检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestGpuUsage()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var gpus = data.Gpus;
                
                var success = true;
                var validGpus = 0;
                
                if (gpus != null)
                {
                    foreach (var gpu in gpus)
                    {
                        if (gpu.LoadPct >= 0 && gpu.LoadPct <= 100)
                        {
                            validGpus++;
                        }
                        else
                        {
                            success = false;
                        }
                    }
                }
                
                var message = success ? "GPU使用率检测成功" : "GPU使用率数据无效";
                var details = new { GpuCount = gpus?.Count ?? 0, ValidGpus = validGpus };
                
                AddTestResult("GPU使用率", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("GPU使用率", false, $"GPU使用率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestGpuTemperature()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var gpus = data.Gpus;
                
                var success = true;
                var validTemps = 0;
                
                if (gpus != null)
                {
                    foreach (var gpu in gpus)
                    {
                        if (gpu.TempC == null || (gpu.TempC >= 0 && gpu.TempC <= 120))
                        {
                            validTemps++;
                        }
                        else
                        {
                            success = false;
                        }
                    }
                }
                
                var message = success ? "GPU温度检测成功" : "GPU温度超出有效范围";
                var details = new { GpuCount = gpus?.Count ?? 0, ValidTemperatures = validTemps };
                
                AddTestResult("GPU温度", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("GPU温度", false, $"GPU温度检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestGpuMemory()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var gpus = data.Gpus;
                
                var success = true;
                var validMemory = 0;
                
                if (gpus != null)
                {
                    foreach (var gpu in gpus)
                    {
                        if (gpu.VramUsedMb == null || gpu.VramUsedMb >= 0)
                        {
                            validMemory++;
                        }
                        else
                        {
                            success = false;
                        }
                    }
                }
                
                var message = success ? "GPU显存检测成功" : "GPU显存数据无效";
                var details = new { GpuCount = gpus?.Count ?? 0, ValidMemory = validMemory };
                
                AddTestResult("GPU显存", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("GPU显存", false, $"GPU显存检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestGpuClockSpeeds()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var gpus = data.Gpus;
                
                var success = true;
                var validClocks = 0;
                
                if (gpus != null)
                {
                    foreach (var gpu in gpus)
                    {
                        if (gpu.CoreMhz == null || gpu.CoreMhz >= 0)
                        {
                            validClocks++;
                        }
                        else
                        {
                            success = false;
                        }
                    }
                }
                
                var message = success ? "GPU时钟频率检测成功" : "GPU时钟频率数据无效";
                var details = new { GpuCount = gpus?.Count ?? 0, ValidClocks = validClocks };
                
                AddTestResult("GPU时钟频率", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("GPU时钟频率", false, $"GPU时钟频率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestGpuFanSpeed()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var gpus = data.Gpus;
                
                var success = true;
                var validFans = 0;
                
                if (gpus != null)
                {
                    foreach (var gpu in gpus)
                    {
                        if (gpu.FanRpm == null || gpu.FanRpm >= 0)
                        {
                            validFans++;
                        }
                        else
                        {
                            success = false;
                        }
                    }
                }
                
                var message = success ? "GPU风扇转速检测成功" : "GPU风扇转速数据无效";
                var details = new { GpuCount = gpus?.Count ?? 0, ValidFans = validFans };
                
                AddTestResult("GPU风扇转速", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("GPU风扇转速", false, $"GPU风扇转速检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestGpuPower()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var gpus = data.Gpus;
                
                var success = true;
                var validPower = 0;
                
                if (gpus != null)
                {
                    foreach (var gpu in gpus)
                    {
                        if (gpu.PowerW == null || gpu.PowerW >= 0)
                        {
                            validPower++;
                        }
                        else
                        {
                            success = false;
                        }
                    }
                }
                
                var message = success ? "GPU功耗检测成功" : "GPU功耗数据无效";
                var details = new { GpuCount = gpus?.Count ?? 0, ValidPower = validPower };
                
                AddTestResult("GPU功耗", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("GPU功耗", false, $"GPU功耗检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestGpuVoltage()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var gpus = data.Gpus;
                
                var success = true;
                var validVoltage = 0;
                
                if (gpus != null)
                {
                    foreach (var gpu in gpus)
                    {
                        if (gpu.VoltageV == null || gpu.VoltageV >= 0)
                        {
                            validVoltage++;
                        }
                        else
                        {
                            success = false;
                        }
                    }
                }
                
                var message = success ? "GPU电压检测成功" : "GPU电压数据无效";
                var details = new { GpuCount = gpus?.Count ?? 0, ValidVoltage = validVoltage };
                
                AddTestResult("GPU电压", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("GPU电压", false, $"GPU电压检测失败: {ex.Message}", null, ex);
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
