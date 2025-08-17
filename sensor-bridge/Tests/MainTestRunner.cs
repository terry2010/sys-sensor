using System;
using System.Threading.Tasks;
using System.IO;

namespace SensorBridge.Tests
{
    /// <summary>
    /// 主测试运行器 - 整合所有拆分的测试类
    /// 为85个监控指标提供完整的测试覆盖
    /// </summary>
    public class MainTestRunner
    {
        private readonly string _testReportPath;
        public MainTestRunner()
        {
            _testReportPath = Path.Combine(Directory.GetCurrentDirectory(), 
                $"comprehensive-test-report-{DateTime.Now:yyyy-MM-dd-HH-mm-ss}.json");
        }

        /// <summary>
        /// 运行所有85个指标的完整测试
        /// </summary>
        public async Task<TestSummary> RunAllTestsAsync()
        {
            Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 开始运行完整的85指标测试套件...");
            Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 测试报告将保存至: {_testReportPath}");

            var overallSummary = new TestSummary
            {
                TestStartTime = DateTime.Now,
                TestResults = new System.Collections.Generic.List<TestResult>(),
                ReportPath = _testReportPath
            };

            try
            {
                // 1. CPU测试 (12个指标)
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 开始CPU测试...");
                var cpuTests = new CpuTests(_testReportPath);
                await cpuTests.RunAllCpuTests();
                overallSummary.TestResults.AddRange(cpuTests.GetTestResults());

                // 2. 内存测试 (12个指标)
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 开始内存测试...");
                var memoryTests = new MemoryTests(_testReportPath);
                await memoryTests.RunAllMemoryTests();
                overallSummary.TestResults.AddRange(memoryTests.GetTestResults());

                // 3. 网络测试 (15个指标)
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 开始网络测试...");
                var networkTests = new NetworkTests(_testReportPath);
                await networkTests.RunAllNetworkTests();
                overallSummary.TestResults.AddRange(networkTests.GetTestResults());

                // 4. 存储测试 (8个指标)
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 开始存储测试...");
                var storageTests = new StorageTests(_testReportPath);
                await storageTests.RunAllStorageTests();
                overallSummary.TestResults.AddRange(storageTests.GetTestResults());

                // 5. GPU测试 (8个指标)
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 开始GPU测试...");
                var gpuTests = new GpuTests(_testReportPath);
                await gpuTests.RunAllGpuTests();
                overallSummary.TestResults.AddRange(gpuTests.GetTestResults());

                // 6. 系统其他测试 (30个指标)
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 开始系统其他测试...");
                var systemTests = new SystemTests(_testReportPath);
                await systemTests.RunAllSystemTests();
                overallSummary.TestResults.AddRange(systemTests.GetTestResults());

                // 生成最终汇总
                overallSummary.TestEndTime = DateTime.Now;
                overallSummary.TotalDuration = overallSummary.TestEndTime - overallSummary.TestStartTime;
                overallSummary.TotalTests = overallSummary.TestResults.Count;
                overallSummary.PassedTests = 0;
                overallSummary.FailedTests = 0;

                foreach (var result in overallSummary.TestResults)
                {
                    if (result.Success)
                        overallSummary.PassedTests++;
                    else
                        overallSummary.FailedTests++;
                }

                overallSummary.SuccessRate = overallSummary.TotalTests > 0 
                    ? (double)overallSummary.PassedTests / overallSummary.TotalTests * 100 
                    : 0;

                overallSummary.IsAdministrator = SensorUtils.IsAdministrator();

                // 保存测试报告
                await SaveTestReportAsync(overallSummary);

                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 完整测试套件执行完成!");
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 总测试数: {overallSummary.TotalTests}");
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 通过: {overallSummary.PassedTests}");
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 失败: {overallSummary.FailedTests}");
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 成功率: {overallSummary.SuccessRate:F1}%");
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 报告已保存至: {_testReportPath}");

                return overallSummary;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] 测试执行过程中发生异常: {ex.Message}");
                throw;
            }
        }

        /// <summary>
        /// 生成Markdown格式的测试报告
        /// </summary>
        public async Task GenerateMarkdownReportAsync(TestSummary summary)
        {
            var mdReportPath = _testReportPath.Replace(".json", ".md");
            
            var mdContent = $@"# 系统监控指标测试报告

## 测试概览
- **测试开始时间**: {summary.TestStartTime:yyyy-MM-dd HH:mm:ss}
- **测试结束时间**: {summary.TestEndTime:yyyy-MM-dd HH:mm:ss}
- **测试总时长**: {summary.TotalDuration.TotalSeconds:F1} 秒
- **总测试数**: {summary.TotalTests}
- **通过测试数**: {summary.PassedTests}
- **失败测试数**: {summary.FailedTests}
- **成功率**: {summary.SuccessRate:F1}%
- **管理员权限**: {(summary.IsAdministrator ? "是" : "否")}

## 详细测试结果

";

            // 按类别分组显示测试结果
            var categories = new[]
            {
                ("CPU相关", new[] { "CPU使用率", "CPU温度", "CPU频率", "CPU功耗", "CPU每核心使用率", "CPU每核心温度", "CPU每核心频率", "CPU每核心功耗", "CPU基础频率", "CPU最大频率", "CPU电压", "CPU缓存" }),
                ("内存相关", new[] { "内存使用量", "内存使用率", "内存总量", "可用内存", "已用内存", "内存缓冲区", "内存缓存", "内存交换", "内存提交", "内存池", "内存句柄", "内存分页" }),
                ("网络相关", new[] { "网络下行上行(平滑)", "网络下行上行(瞬时)", "网络错误(RX/TX)", "网络丢包率", "活动连接数", "网络延迟", "多目标延迟", "WiFi基础信息", "WiFi参数", "WiFi速率", "WiFi RSSI", "WiFi安全", "WiFi信道宽度", "网络接口", "公网信息" }),
                ("存储相关", new[] { "磁盘使用量", "磁盘读写速度", "磁盘队列长度", "磁盘活动时间", "磁盘响应时间", "SMART健康", "磁盘温度", "磁盘列表" }),
                ("GPU相关", new[] { "GPU基础信息", "GPU使用率", "GPU温度", "GPU显存", "GPU时钟频率", "GPU风扇转速", "GPU功耗", "GPU电压" }),
                ("系统其他", new[] { "系统运行时间", "进程数量", "进程列表", "电池基础信息", "电池健康", "电池时间预估", "系统风扇", "主板电压", "系统温度", "系统负载时间戳" })
            };

            foreach (var (categoryName, testNames) in categories)
            {
                mdContent += $"### {categoryName}\n\n";
                mdContent += "| 测试项目 | 状态 | 耗时(ms) | 详细信息 |\n";
                mdContent += "|---------|------|---------|----------|\n";

                foreach (var testName in testNames)
                {
                    var testResult = summary.TestResults.Find(r => r.TestName == testName);
                    if (testResult != null)
                    {
                        var status = testResult.Success ? "✅ 通过" : "❌ 失败";
                        var duration = testResult.Duration.TotalMilliseconds.ToString("F0");
                        var details = testResult.Success ? testResult.Message : $"{testResult.Message} ({testResult.ErrorDetails?.Substring(0, Math.Min(50, testResult.ErrorDetails?.Length ?? 0))})";
                        mdContent += $"| {testName} | {status} | {duration} | {details} |\n";
                    }
                    else
                    {
                        mdContent += $"| {testName} | ⚠️ 未执行 | - | 测试未运行 |\n";
                    }
                }
                mdContent += "\n";
            }

            mdContent += $@"
## 测试环境信息
- **操作系统**: Windows 10
- **测试框架**: C# + LibreHardwareMonitor
- **管理员权限**: {(summary.IsAdministrator ? "已获取" : "未获取")}
- **报告生成时间**: {DateTime.Now:yyyy-MM-dd HH:mm:ss}

## 总结
本次测试共覆盖了系统监控的85个关键指标，测试成功率为 {summary.SuccessRate:F1}%。
{(summary.SuccessRate >= 90 ? "测试结果良好，系统监控功能正常。" : "部分测试失败，建议检查相关硬件或权限配置。")}
";

            await File.WriteAllTextAsync(mdReportPath, mdContent);
            Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] Markdown报告已生成: {mdReportPath}");
        }

        private async Task SaveTestReportAsync(TestSummary summary)
        {
            try
            {
                var options = new System.Text.Json.JsonSerializerOptions
                {
                    WriteIndented = true,
                    PropertyNamingPolicy = System.Text.Json.JsonNamingPolicy.CamelCase,
                    Encoder = System.Text.Encodings.Web.JavaScriptEncoder.Create(System.Text.Unicode.UnicodeRanges.All)
                };

                var json = System.Text.Json.JsonSerializer.Serialize(summary, options);
                await File.WriteAllTextAsync(_testReportPath, json);

                // 同时生成Markdown报告
                await GenerateMarkdownReportAsync(summary);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"保存测试报告失败: {ex.Message}");
                throw;
            }
        }
    }
}
