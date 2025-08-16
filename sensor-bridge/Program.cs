using System.Text.Json;
using System.Text;
using LibreHardwareMonitor.Hardware;
using System.Linq;
using System.Security.Principal;
using System.IO;
using System.Collections.Generic;
using System.Text.RegularExpressions;
using SensorBridge;

class Program
{
    static async Task<int> Main(string[] args)
    {
        Console.OutputEncoding = Encoding.UTF8;
        
        // 检查是否为测试模式
        if (args.Length > 0 && (args.Contains("--test") || args.Contains("--output-dir")))
        {
            // 运行测试程序
            return await TestProgram.RunAsync(args);
        }
        
        var jsonOptions = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull,
            WriteIndented = false
        };

        // 初始化日志文件
        try
        {
            var lf = Environment.GetEnvironmentVariable("BRIDGE_LOG_FILE");
            if (!string.IsNullOrWhiteSpace(lf))
            {
                ConfigurationManager.InitializeLogFile(lf);
            }
        }
        catch { }

        // 运行传感器监控主循环
        SensorMonitor.RunMonitoringLoop(jsonOptions);
        return 0;
    }

    // 创建并开启 Computer，统一初始化开关







}
