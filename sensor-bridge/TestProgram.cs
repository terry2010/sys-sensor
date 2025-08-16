using System;
using System.Threading.Tasks;

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
                var testRunner = new TestRunner();
                var summary = await testRunner.RunAllTestsAsync();

                // 显示测试结果摘要
                DisplayTestSummary(summary);

                // 如果指定了输出路径，复制报告
                if (!string.IsNullOrEmpty(options.OutputPath))
                {
                    var reportFiles = System.IO.Directory.GetFiles(
                        System.IO.Directory.GetCurrentDirectory(), 
                        "bridge-test-report-*.json");
                    
                    if (reportFiles.Length > 0)
                    {
                        var latestReport = reportFiles[reportFiles.Length - 1];
                        var targetPath = System.IO.Path.Combine(options.OutputPath, 
                            System.IO.Path.GetFileName(latestReport));
                        System.IO.File.Copy(latestReport, targetPath, true);
                        Console.WriteLine($"测试报告已复制到: {targetPath}");
                    }
                }

                return summary.FailedTests == 0 ? 0 : 1;
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
                    case "-o":
                    case "--output":
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
            Console.WriteLine("  -o, --output    指定测试报告输出目录");
            Console.WriteLine("  -v, --verbose   详细输出模式");
            Console.WriteLine();
            Console.WriteLine("示例:");
            Console.WriteLine("  TestProgram.exe                    # 运行所有测试");
            Console.WriteLine("  TestProgram.exe -o C:\\Reports      # 运行测试并将报告保存到指定目录");
            Console.WriteLine("  TestProgram.exe -v                 # 详细模式运行测试");
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

        private class TestOptions
        {
            public bool ShowHelp { get; set; }
            public string OutputPath { get; set; } = "";
            public bool Verbose { get; set; }
        }
    }
}
