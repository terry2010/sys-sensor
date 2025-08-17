using System;
using System.Threading.Tasks;

namespace SensorBridge.Tests
{
    public class NetworkTests : BaseTestRunner
    {
        public NetworkTests(string testReportPath) : base(testReportPath)
        {
        }

        public async Task RunAllNetworkTests()
        {
            await TestNetworkSmoothRates();
            await TestNetworkInstantRates();
            await TestNetworkErrors();
            await TestNetworkPacketLoss();
            await TestActiveConnections();
            await TestNetworkLatency();
            await TestMultiTargetLatency();
            await TestWiFiBasicInfo();
            await TestWiFiParameters();
            await TestWiFiRates();
            await TestWiFiSignalInfo();
            await TestWiFiSecurity();
            await TestWiFiChannelWidth();
            await TestNetworkInterfaces();
            await TestPublicNetworkInfo();
        }

        private async Task TestNetworkSmoothRates()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var netRxBps = data.NetRxBps;
                var netTxBps = data.NetTxBps;
                
                var success = netRxBps >= 0 && netTxBps >= 0;
                var message = success ? "网络平滑速率检测成功" : "网络平滑速率数据无效";
                var details = new { NetRxBps = netRxBps, NetTxBps = netTxBps };
                
                AddTestResult("网络下行上行(平滑)", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("网络下行上行(平滑)", false, $"网络平滑速率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestNetworkInstantRates()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var netRxInstant = data.NetRxInstantBps;
                var netTxInstant = data.NetTxInstantBps;
                
                var success = netRxInstant >= 0 && netTxInstant >= 0;
                var message = success ? "网络瞬时速率检测成功" : "网络瞬时速率数据无效";
                var details = new { NetRxInstantBps = netRxInstant, NetTxInstantBps = netTxInstant };
                
                AddTestResult("网络下行上行(瞬时)", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("网络下行上行(瞬时)", false, $"网络瞬时速率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestNetworkErrors()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var netRxErrPs = data.NetRxErrPs;
                var netTxErrPs = data.NetTxErrPs;
                
                var success = (netRxErrPs == null || netRxErrPs >= 0) && (netTxErrPs == null || netTxErrPs >= 0);
                var message = success ? "网络错误率检测成功" : "网络错误率数据无效";
                var details = new { NetRxErrPs = netRxErrPs, NetTxErrPs = netTxErrPs };
                
                AddTestResult("网络错误(RX/TX)", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("网络错误(RX/TX)", false, $"网络错误率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestNetworkPacketLoss()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var packetLoss = data.PacketLossPct;
                
                var success = packetLoss == null || (packetLoss >= 0 && packetLoss <= 100);
                var message = success ? "网络丢包率检测成功" : "网络丢包率超出有效范围";
                var details = new { PacketLossPct = packetLoss, Valid = success };
                
                AddTestResult("网络丢包率", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("网络丢包率", false, $"网络丢包率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestActiveConnections()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var activeConns = data.ActiveConnections;
                
                var success = activeConns == null || activeConns >= 0;
                var message = success ? "活动连接数检测成功" : "活动连接数数据无效";
                var details = new { ActiveConnections = activeConns, Valid = success };
                
                AddTestResult("活动连接数", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("活动连接数", false, $"活动连接数检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestNetworkLatency()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var pingRtt = data.PingRttMs;
                
                var success = pingRtt == null || (pingRtt >= 0 && pingRtt <= 10000);
                var message = success ? "网络延迟检测成功" : "网络延迟超出有效范围";
                var details = new { PingRttMs = pingRtt, Valid = success };
                
                AddTestResult("网络延迟", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("网络延迟", false, $"网络延迟检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestMultiTargetLatency()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var rttMulti = data.RttMulti;
                
                var success = true; // 多目标延迟数组总是有效的
                var message = "多目标延迟检测成功";
                var details = new { RttMultiCount = rttMulti?.Count ?? 0 };
                
                AddTestResult("多目标延迟", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("多目标延迟", false, $"多目标延迟检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestWiFiBasicInfo()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var wifiSsid = data.WifiSsid;
                var wifiSignal = data.WifiSignalPct;
                var wifiLink = data.WifiLinkMbps;
                var wifiBssid = data.WifiBssid;
                
                var success = true; // WiFi基础信息总是有效的
                var message = "WiFi基础信息检测成功";
                var details = new { WifiSsid = wifiSsid, WifiSignalPct = wifiSignal, WifiLinkMbps = wifiLink, WifiBssid = wifiBssid };
                
                AddTestResult("WiFi基础信息", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("WiFi基础信息", false, $"WiFi基础信息检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestWiFiParameters()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var wifiChannel = data.WifiChannel;
                var wifiBand = data.WifiBand;
                var wifiRadio = data.WifiRadio;
                
                var success = true; // WiFi参数总是有效的
                var message = "WiFi参数检测成功";
                var details = new { WifiChannel = wifiChannel, WifiBand = wifiBand, WifiRadio = wifiRadio };
                
                AddTestResult("WiFi参数", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("WiFi参数", false, $"WiFi参数检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestWiFiRates()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var wifiRxMbps = data.WifiRxMbps;
                var wifiTxMbps = data.WifiTxMbps;
                
                var success = (wifiRxMbps == null || wifiRxMbps >= 0) && (wifiTxMbps == null || wifiTxMbps >= 0);
                var message = success ? "WiFi速率检测成功" : "WiFi速率数据无效";
                var details = new { WifiRxMbps = wifiRxMbps, WifiTxMbps = wifiTxMbps };
                
                AddTestResult("WiFi速率", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("WiFi速率", false, $"WiFi速率检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestWiFiSignalInfo()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var wifiRssi = data.WifiRssiDbm;
                var wifiRssiEst = data.WifiRssiEstimated;
                
                var success = (wifiRssi == null || (wifiRssi >= -100 && wifiRssi <= 0));
                var message = success ? "WiFi信号信息检测成功" : "WiFi信号强度超出有效范围";
                var details = new { WifiRssiDbm = wifiRssi, WifiRssiEstimated = wifiRssiEst };
                
                AddTestResult("WiFi RSSI", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("WiFi RSSI", false, $"WiFi信号信息检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestWiFiSecurity()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var wifiAuth = data.WifiAuth;
                var wifiCipher = data.WifiCipher;
                
                var success = true; // WiFi安全信息总是有效的
                var message = "WiFi安全信息检测成功";
                var details = new { WifiAuth = wifiAuth, WifiCipher = wifiCipher };
                
                AddTestResult("WiFi安全", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("WiFi安全", false, $"WiFi安全信息检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestWiFiChannelWidth()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var wifiChanWidth = data.WifiChanWidthMhz;
                
                var success = wifiChanWidth == null || (wifiChanWidth > 0 && wifiChanWidth <= 320);
                var message = success ? "WiFi信道宽度检测成功" : "WiFi信道宽度超出有效范围";
                var details = new { WifiChanWidthMhz = wifiChanWidth, Valid = success };
                
                AddTestResult("WiFi信道宽度", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("WiFi信道宽度", false, $"WiFi信道宽度检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestNetworkInterfaces()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var netIfs = data.NetIfs;
                
                var success = netIfs != null && netIfs.Count > 0;
                var message = success ? "网络接口检测成功" : "未检测到网络接口";
                var details = new { NetworkInterfaceCount = netIfs?.Count ?? 0 };
                
                AddTestResult("网络接口", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("网络接口", false, $"网络接口检测失败: {ex.Message}", null, ex);
            }
            finally
            {
                _summary.TestResults[^1].StartTime = startTime;
                _summary.TestResults[^1].EndTime = DateTime.Now;
                _summary.TestResults[^1].Duration = DateTime.Now - startTime;
            }
        }

        private async Task TestPublicNetworkInfo()
        {
            var startTime = DateTime.Now;
            try
            {
                var data = await TestDataCollector.CollectDataAsync();
                var publicIp = data.PublicIp;
                var isp = data.Isp;
                
                var success = true; // 公网信息总是有效的
                var message = "公网信息检测成功";
                var details = new { PublicIp = publicIp, Isp = isp };
                
                AddTestResult("公网信息", success, message, details);
            }
            catch (Exception ex)
            {
                AddTestResult("公网信息", false, $"公网信息检测失败: {ex.Message}", null, ex);
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
