using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Threading;
#nullable enable

namespace SensorBridge
{
    /// <summary>
    /// 日志帮助类 - 提供统一的日志记录功能
    /// </summary>
    public static class LogHelper
    {
        // 日志级别
        public enum LogLevel
        {
            Debug,
            Info,
            Warning,
            Error,
            Fatal
        }
        
        // 日志配置
        private static bool _enableConsoleLog = true;
        private static bool _enableFileLog = false;
        private static string _logFilePath = "sensor-bridge.log";
        private static LogLevel _minLogLevel = LogLevel.Info;
        
        // 日志统计
        private static int _debugCount = 0;
        private static int _infoCount = 0;
        private static int _warningCount = 0;
        private static int _errorCount = 0;
        private static int _fatalCount = 0;
        
        // 错误计数器
        private static Dictionary<string, int> _errorCounters = new Dictionary<string, int>();
        
        // 最后一次日志时间
        private static Dictionary<string, DateTime> _lastLogTime = new Dictionary<string, DateTime>();
        
        // 日志限流间隔（毫秒）
        private static int _throttleIntervalMs = 5000;
        
        /// <summary>
        /// 配置日志系统
        /// </summary>
        public static void Configure(bool enableConsole = true, bool enableFile = false, 
                                    string logFilePath = "sensor-bridge.log", LogLevel minLevel = LogLevel.Info)
        {
            _enableConsoleLog = enableConsole;
            _enableFileLog = enableFile;
            _logFilePath = logFilePath ?? string.Empty;
            _minLogLevel = minLevel;
            
            // 初始化日志
            Log(LogLevel.Info, "System", "日志系统已初始化");
        }
        
        /// <summary>
        /// 记录日志
        /// </summary>
        public static void Log(LogLevel level, string module, string message, bool throttle = false, string? throttleKey = null)
        {
            // 检查日志级别
            if (level < _minLogLevel)
                return;
                
            // 日志限流
            if (throttle)
            {
                string key = throttleKey ?? $"{module}_{message}";
                if (_lastLogTime.TryGetValue(key, out DateTime lastLogTime))
                {
                    var elapsed = (DateTime.Now - lastLogTime).TotalMilliseconds;
                    if (elapsed < _throttleIntervalMs)
                        return;
                }
                _lastLogTime[key] = DateTime.Now;
            }
            
            // 更新统计
            switch (level)
            {
                case LogLevel.Debug: _debugCount++; break;
                case LogLevel.Info: _infoCount++; break;
                case LogLevel.Warning: _warningCount++; break;
                case LogLevel.Error: _errorCount++; break;
                case LogLevel.Fatal: _fatalCount++; break;
            }
            
            // 错误计数
            if (level >= LogLevel.Error)
            {
                string errorKey = $"{module}_{message}";
                if (!_errorCounters.ContainsKey(errorKey))
                    _errorCounters[errorKey] = 1;
                else
                    _errorCounters[errorKey]++;
            }
            
            // 构建日志消息
            string timestamp = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss.fff");
            string levelStr = level.ToString().ToUpper().PadRight(7);
            string logMessage = $"[{timestamp}] [{levelStr}] [{module}] {message}";
            
            // 输出到控制台
            if (_enableConsoleLog)
            {
                if (level >= LogLevel.Error)
                    Console.Error.WriteLine(logMessage);
                else
                    Console.WriteLine(logMessage);
            }
            
            // 输出到文件
            if (_enableFileLog)
            {
                try
                {
                    File.AppendAllText(_logFilePath, logMessage + Environment.NewLine);
                }
                catch { }
            }
        }
        
        /// <summary>
        /// 记录调试日志
        /// </summary>
        public static void Debug(string module, string message, bool throttle = false)
        {
            Log(LogLevel.Debug, module, message, throttle);
        }
        
        /// <summary>
        /// 记录信息日志
        /// </summary>
        public static void Info(string module, string message, bool throttle = false)
        {
            Log(LogLevel.Info, module, message, throttle);
        }
        
        /// <summary>
        /// 记录警告日志
        /// </summary>
        public static void Warning(string module, string message, bool throttle = false)
        {
            Log(LogLevel.Warning, module, message, throttle);
        }
        
        /// <summary>
        /// 记录错误日志
        /// </summary>
        public static void Error(string module, string message, Exception? ex = null, bool throttle = false)
        {
            string fullMessage = message;
            if (ex != null)
                fullMessage += $": {ex.Message}";
                
            Log(LogLevel.Error, module, fullMessage, throttle);
            
            // 记录详细堆栈信息（仅限严重错误）
            if (ex != null && _minLogLevel <= LogLevel.Debug)
            {
                Log(LogLevel.Debug, module, $"异常堆栈: {ex.StackTrace ?? "<无堆栈信息>"}");
            }
        }
        
        /// <summary>
        /// 记录致命错误日志
        /// </summary>
        public static void Fatal(string module, string message, Exception? ex = null)
        {
            string fullMessage = message;
            if (ex != null)
                fullMessage += $": {ex.Message}\n{ex.StackTrace ?? "<无堆栈信息>"}";
                
            Log(LogLevel.Fatal, module, fullMessage);
        }
        
        /// <summary>
        /// 输出日志统计信息
        /// </summary>
        public static void DumpStats()
        {
            StringBuilder sb = new StringBuilder();
            sb.AppendLine("[LogHelper] 日志统计信息:");
            sb.AppendLine($"- Debug: {_debugCount}");
            sb.AppendLine($"- Info: {_infoCount}");
            sb.AppendLine($"- Warning: {_warningCount}");
            sb.AppendLine($"- Error: {_errorCount}");
            sb.AppendLine($"- Fatal: {_fatalCount}");
            
            if (_errorCount > 0 || _fatalCount > 0)
            {
                sb.AppendLine("错误计数器:");
                foreach (var kvp in _errorCounters)
                {
                    sb.AppendLine($"- {kvp.Key}: {kvp.Value}次");
                }
            }
            
            Console.Error.WriteLine(sb.ToString());
        }
    }
}
