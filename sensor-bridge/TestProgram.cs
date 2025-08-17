using System;
using System.Threading.Tasks;
using System.IO;
using System.Linq;
using SensorBridge.Tests;

namespace SensorBridge
{
    /// <summary>
    /// C# 桥接层测试程序入口
    /// 支持独立运行测试或作为便携版测试的一部分
    /// </summary>
    public class TestProgram
    {
        public static async Task<int> RunAsync(string[] args)
        {
            Console.WriteLine("=== sys-sensor C# 桥接层自动化测试 ===");
            Console.WriteLine($"启动时间: {DateTime.Now:yyyy-MM-dd HH:mm:ss}");
            Console.WriteLine();

            try
            {
                // 解析命令行参数
                var options = ParseArguments(args);
                
                if (options.ShowHelp)
                {
                    ShowHelp();
                    return 0;
                }

                // 运行测试
                if (options.RunMain)
                {
                    Console.WriteLine("[TestProgram] 进入 --test-main 模式，启动 MainTestRunner...");
                    // 运行综合85项指标测试
                    var mainRunner = new MainTestRunner();
                    var mainSummary = await mainRunner.RunAllTestsAsync();
                    Console.WriteLine("[TestProgram] MainTestRunner 完成，开始处理报告复制...");

                    // 将 comprehensive-* 报告复制为 main-test-* 到 test-reports 目录
                    var cwd = Directory.GetCurrentDirectory();
                    var testReportsDir = Path.GetFullPath(Path.Combine(cwd, "..", "test-reports"));
                    Directory.CreateDirectory(testReportsDir);

                    // JSON
                    var compJson = mainSummary.ReportPath;
                    if (string.IsNullOrEmpty(compJson) || !File.Exists(compJson))
                    {
                        var found = Directory.GetFiles(cwd, "comprehensive-test-report-*.json");
                        if (found.Length > 0) compJson = found.OrderBy(f => f).Last();
                    }
                    Console.WriteLine($"[TestProgram] 当前目录: {cwd}");
                    Console.WriteLine($"[TestProgram] 目标报告目录: {testReportsDir}");
                    Console.WriteLine($"[TestProgram] 发现的综合报告JSON路径: {compJson ?? "<null>"}");
                    if (!string.IsNullOrEmpty(compJson) && File.Exists(compJson))
                    {
                        var ts = Path.GetFileName(compJson).Replace("comprehensive-test-report-", string.Empty).Replace(".json", string.Empty);
                        var mainJson = Path.Combine(testReportsDir, $"main-test-report-{ts}.json");
                        File.Copy(compJson, mainJson, true);
                        Console.WriteLine($"[TestProgram] 综合测试报告(JSON)已复制到: {mainJson}");

                        // MD
                        var compMd = compJson.Replace(".json", ".md");
                        Console.WriteLine($"[TestProgram] 发现的综合报告MD路径: {compMd}");
                        if (File.Exists(compMd))
                        {
                            var mainMd = Path.Combine(testReportsDir, $"main-test-report-{ts}.md");
                            File.Copy(compMd, mainMd, true);
                            Console.WriteLine($"[TestProgram] 综合测试报告(MD)已复制到: {mainMd}");
                        }
                        else
                        {
                            Console.WriteLine("[TestProgram][WARN] 未找到Markdown报告文件，跳过MD复制。");
                        }
                    }
                    else
                    {
                        Console.WriteLine("[TestProgram][ERROR] 未找到综合测试JSON报告文件，无法复制到 test-reports。");
                    }

                    // 显示测试结果摘要
                    DisplayTestSummary(mainSummary);

                    // 如果指定了输出路径，复制报告（按综合测试模式）
                    if (!string.IsNullOrEmpty(options.OutputPath))
                    {
                        var reportFiles = Directory.GetFiles(
                            Directory.GetCurrentDirectory(),
                            "comprehensive-test-report-*.json");
                        if (reportFiles.Length > 0)
                        {
                            var latestReport = reportFiles[reportFiles.Length - 1];
                            var targetPath = Path.Combine(options.OutputPath, Path.GetFileName(latestReport));
                            File.Copy(latestReport, targetPath, true);
                            Console.WriteLine($"[TestProgram] 测试报告已复制到: {targetPath}");
                        }
                        else
                        {
                            Console.WriteLine("[TestProgram][WARN] 未在当前目录发现 comprehensive-test-report-*.json, 跳过按 --output-dir 复制。");
                        }
                    }

                    return mainSummary.FailedTests == 0 ? 0 : 1;
                }
                else
                {
                    Console.WriteLine("[TestProgram] 进入桥接层测试模式，启动 TestRunner...");
                    var testRunner = new TestRunner();
                    var bridgeSummary = await testRunner.RunAllTestsAsync();

                    // 显示测试结果摘要
                    DisplayTestSummary(bridgeSummary);

                    // 如果指定了输出路径，复制报告（按桥接层测试模式）
                    if (!string.IsNullOrEmpty(options.OutputPath))
                    {
                        var reportFiles = Directory.GetFiles(
                            Directory.GetCurrentDirectory(),
                            "bridge-test-report-*.json");
                        if (reportFiles.Length > 0)
                        {
                            var latestReport = reportFiles[reportFiles.Length - 1];
                            var targetPath = Path.Combine(options.OutputPath, Path.GetFileName(latestReport));
                            File.Copy(latestReport, targetPath, true);
                            Console.WriteLine($"[TestProgram] 测试报告已复制到: {targetPath}");
                        }
                        else
                        {
                            Console.WriteLine("[TestProgram][WARN] 未在当前目录发现 bridge-test-report-*.json, 跳过按 --output-dir 复制。");
                        }
                    }

                    return bridgeSummary.FailedTests == 0 ? 0 : 1;
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"测试运行失败: {ex.Message}");
                Console.WriteLine($"堆栈跟踪: {ex.StackTrace}");
                return -1;
            }
        }

        private static TestOptions ParseArguments(string[] args)
        {
            var options = new TestOptions();

            for (int i = 0; i < args.Length; i++)
            {
                switch (args[i].ToLower())
                {
                    case "-h":
                    case "--help":
                        options.ShowHelp = true;
                        break;
                    case "--test-main":
                        options.RunMain = true;
                        break;
                    case "-o":
                    case "--output":
                    case "--output-dir":
                        if (i + 1 < args.Length)
                        {
                            options.OutputPath = args[++i];
                        }
                        break;
                    case "-v":
                    case "--verbose":
                        options.Verbose = true;
                        break;
                }
            }

            return options;
        }

        private static void ShowHelp()
        {
            Console.WriteLine("用法: TestProgram.exe [选项]");
            Console.WriteLine();
            Console.WriteLine("选项:");
            Console.WriteLine("  -h, --help      显示此帮助信息");
            Console.WriteLine("  --test-main     运行综合85项主测试套件");
            Console.WriteLine("  -o, --output    指定测试报告输出目录");
            Console.WriteLine("      --output-dir  指定测试报告输出目录(同 --output)");
            Console.WriteLine("  -v, --verbose   详细输出模式");
            Console.WriteLine();
            Console.WriteLine("示例:");
            Console.WriteLine("  TestProgram.exe                    # 运行桥接层测试");
            Console.WriteLine("  TestProgram.exe --test-main        # 运行综合85项主测试");
            Console.WriteLine("  TestProgram.exe -o C:\\Reports      # 运行并将报告保存到指定目录");
            Console.WriteLine("  TestProgram.exe --test-main --output-dir C:\\Reports");
        }

        private static void DisplayTestSummary(TestSummary summary)
        {
            Console.WriteLine();
            Console.WriteLine("=== 测试结果摘要 ===");
            Console.WriteLine($"测试开始时间: {summary.TestStartTime:yyyy-MM-dd HH:mm:ss}");
            Console.WriteLine($"测试结束时间: {summary.TestEndTime:yyyy-MM-dd HH:mm:ss}");
            Console.WriteLine($"总耗时: {summary.TotalDuration.TotalSeconds:F2}秒");
            Console.WriteLine($"管理员权限: {(summary.IsAdministrator ? "是" : "否")}");
            Console.WriteLine();
            Console.WriteLine($"总测试数: {summary.TotalTests}");
            Console.WriteLine($"通过: {summary.PassedTests}");
            Console.WriteLine($"失败: {summary.FailedTests}");
            Console.WriteLine($"成功率: {summary.SuccessRate:F1}%");
            Console.WriteLine();

            // 显示测试结果详情
            Console.WriteLine("【测试详情】");
            foreach (var test in summary.TestResults)
            {
                var status = test.Success ? "✓" : "✗";
                var duration = test.Duration.TotalMilliseconds;
                Console.WriteLine($"  {status} {test.TestName} ({duration:F0}ms)");
                
                if (test.Details != null)
                {
                    Console.WriteLine($"    详情: {test.Details}");
                }
                
                if (!test.Success && !string.IsNullOrEmpty(test.ErrorDetails))
                {
                    Console.WriteLine($"    错误: {test.ErrorDetails}");
                }
            }
            Console.WriteLine();

            // 显示环境信息
            Console.WriteLine("【环境信息】");
            Console.WriteLine($"  操作系统: {Environment.OSVersion}");
            Console.WriteLine($"  .NET版本: {Environment.Version}");
            Console.WriteLine($"  报告路径: {summary.ReportPath}");
        }

        private static void DisplayTestSummary(SensorBridge.Tests.TestSummary summary)
        {
            Console.WriteLine();
            Console.WriteLine("=== 测试结果摘要 ===");
            Console.WriteLine($"测试开始时间: {summary.TestStartTime:yyyy-MM-dd HH:mm:ss}");
            Console.WriteLine($"测试结束时间: {summary.TestEndTime:yyyy-MM-dd HH:mm:ss}");
            Console.WriteLine($"总耗时: {summary.TotalDuration.TotalSeconds:F2}秒");
            Console.WriteLine($"管理员权限: {(summary.IsAdministrator ? "是" : "否")}");
            Console.WriteLine();
            Console.WriteLine($"总测试数: {summary.TotalTests}");
            Console.WriteLine($"通过: {summary.PassedTests}");
            Console.WriteLine($"失败: {summary.FailedTests}");
            Console.WriteLine($"成功率: {summary.SuccessRate:F1}%");
            Console.WriteLine();

            // 显示测试结果详情
            Console.WriteLine("【测试详情】");
            foreach (var test in summary.TestResults)
            {
                var status = test.Success ? "✓" : "✗";
                var duration = test.Duration.TotalMilliseconds;
                Console.WriteLine($"  {status} {test.TestName} ({duration:F0}ms)");
                
                if (test.Details != null)
                {
                    Console.WriteLine($"    详情: {test.Details}");
                }
                
                if (!test.Success && !string.IsNullOrEmpty(test.ErrorDetails))
                {
                    Console.WriteLine($"    错误: {test.ErrorDetails}");
                }
            }
            Console.WriteLine();

            // 显示环境信息
            Console.WriteLine("【环境信息】");
            Console.WriteLine($"  操作系统: {Environment.OSVersion}");
            Console.WriteLine($"  .NET版本: {Environment.Version}");
            Console.WriteLine($"  报告路径: {summary.ReportPath}");
        }

        private class TestOptions
        {
            public bool ShowHelp { get; set; }
            public string OutputPath { get; set; } = "";
            public bool Verbose { get; set; }
            public bool RunMain { get; set; }
        }
    }
}
