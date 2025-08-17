using System;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;
using System.Text.Encodings.Web;
using System.Text.Unicode;

namespace SensorBridge.Tests
{
    public abstract class BaseTestRunner
    {
        protected readonly string _testReportPath;
        protected TestSummary _summary;

        protected BaseTestRunner(string testReportPath)
        {
            _testReportPath = testReportPath;
            _summary = new TestSummary
            {
                TestStartTime = DateTime.Now,
                IsAdministrator = IsRunningAsAdministrator()
            };
        }

        protected static bool IsRunningAsAdministrator()
        {
            try
            {
                var identity = System.Security.Principal.WindowsIdentity.GetCurrent();
                var principal = new System.Security.Principal.WindowsPrincipal(identity);
                return principal.IsInRole(System.Security.Principal.WindowsBuiltInRole.Administrator);
            }
            catch
            {
                return false;
            }
        }

        protected void AddTestResult(string testName, bool success, string message, object? details = null, Exception? exception = null)
        {
            var result = new TestResult
            {
                TestName = testName,
                Success = success,
                Message = message,
                StartTime = DateTime.Now,
                EndTime = DateTime.Now,
                Duration = TimeSpan.Zero,
                Details = details,
                ErrorDetails = exception?.ToString()
            };

            _summary.TestResults.Add(result);
            _summary.TotalTests++;
            if (success)
                _summary.PassedTests++;
            else
                _summary.FailedTests++;
        }

        /// <summary>
        /// 获取所有测试结果
        /// </summary>
        public List<TestResult> GetTestResults()
        {
            return _summary.TestResults;
        }

        protected async Task SaveTestReportAsync()
        {
            try
            {
                _summary.TestEndTime = DateTime.Now;
                _summary.TotalDuration = _summary.TestEndTime - _summary.TestStartTime;
                _summary.SuccessRate = _summary.TotalTests > 0 ? (_summary.PassedTests * 100.0 / _summary.TotalTests) : 0;
                _summary.ReportPath = _testReportPath;

                var options = new JsonSerializerOptions
                {
                    WriteIndented = true,
                    PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
                    Encoder = JavaScriptEncoder.Create(UnicodeRanges.All)
                };

                var json = JsonSerializer.Serialize(_summary, options);
                await File.WriteAllTextAsync(_testReportPath, json, System.Text.Encoding.UTF8);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"[ERROR] 保存测试报告失败: {ex.Message}");
            }
        }

        protected void PrintTestSummary()
        {
            Console.WriteLine("\n=== 测试结果摘要 ===");
            Console.WriteLine($"测试开始时间: {_summary.TestStartTime:yyyy-MM-dd HH:mm:ss}");
            Console.WriteLine($"测试结束时间: {_summary.TestEndTime:yyyy-MM-dd HH:mm:ss}");
            Console.WriteLine($"总耗时: {_summary.TotalDuration.TotalSeconds:F2}秒");
            Console.WriteLine($"管理员权限: {(_summary.IsAdministrator ? "是" : "否")}");
            Console.WriteLine();
            Console.WriteLine($"总测试数: {_summary.TotalTests}");
            Console.WriteLine($"通过: {_summary.PassedTests}");
            Console.WriteLine($"失败: {_summary.FailedTests}");
            Console.WriteLine($"成功率: {_summary.SuccessRate:F1}%");
            Console.WriteLine();

            if (_summary.TestResults.Count > 0)
            {
                Console.WriteLine("【测试详情】");
                foreach (var result in _summary.TestResults)
                {
                    var status = result.Success ? "✓" : "✗";
                    var duration = result.Duration.TotalMilliseconds;
                    Console.WriteLine($"  {status} {result.TestName} ({duration:F0}ms)");
                    if (result.Details != null)
                    {
                        Console.WriteLine($"    详情: {JsonSerializer.Serialize(result.Details, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })}");
                    }
                    if (!result.Success && !string.IsNullOrEmpty(result.ErrorDetails))
                    {
                        Console.WriteLine($"    错误: {result.ErrorDetails}");
                    }
                }
            }

            Console.WriteLine();
            Console.WriteLine("【环境信息】");
            Console.WriteLine($"  操作系统: {Environment.OSVersion}");
            Console.WriteLine($"  .NET版本: {Environment.Version}");
            Console.WriteLine($"  报告路径: {_testReportPath}");
        }
    }
}
