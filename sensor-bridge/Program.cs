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
    static void Main()
    {
        Console.OutputEncoding = Encoding.UTF8;
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
    }

    // 创建并开启 Computer，统一初始化开关







}
