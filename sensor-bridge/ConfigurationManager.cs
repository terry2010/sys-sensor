using System;
using System.IO;
using System.Text;

namespace SensorBridge
{
    /// <summary>
    /// 配置管理器：负责环境变量读取、日志配置和记录
    /// </summary>
    public static class ConfigurationManager
    {
        private static string? s_logFilePath;
        private static readonly object s_logLock = new object();

        /// <summary>
        /// 从环境变量读取 int 值，并限定范围
        /// </summary>
        /// <param name="name">环境变量名</param>
        /// <param name="defaultValue">默认值</param>
        /// <param name="min">最小值</param>
        /// <param name="max">最大值</param>
        /// <returns>限定范围内的整数值</returns>
        public static int ReadEnvInt(string name, int defaultValue, int min, int max)
        {
            try
            {
                var s = Environment.GetEnvironmentVariable(name);
                if (!string.IsNullOrWhiteSpace(s) && int.TryParse(s, out var v))
                {
                    if (v < min) return min;
                    if (v > max) return max;
                    return v;
                }
            }
            catch { }
            return defaultValue;
        }

        /// <summary>
        /// 初始化日志文件路径
        /// </summary>
        /// <param name="logFilePath">日志文件路径</param>
        public static void InitializeLogFile(string logFilePath)
        {
            try
            {
                if (!string.IsNullOrWhiteSpace(logFilePath))
                {
                    s_logFilePath = logFilePath;
                    TryEnsureLogDir(logFilePath);
                }
            }
            catch { }
        }

        /// <summary>
        /// 记录日志：同时写入 stderr 与可选文件
        /// </summary>
        /// <param name="msg">日志消息</param>
        public static void Log(string msg)
        {
            var line = $"[bridge]{DateTime.UtcNow:O} {msg}";
            try { Console.Error.WriteLine(line); Console.Error.Flush(); } catch { }
            var path = s_logFilePath;
            if (!string.IsNullOrWhiteSpace(path))
            {
                try { lock (s_logLock) { File.AppendAllText(path, line + Environment.NewLine, Encoding.UTF8); } } catch { }
            }
        }

        /// <summary>
        /// 确保日志目录存在
        /// </summary>
        /// <param name="path">日志文件路径</param>
        private static void TryEnsureLogDir(string path)
        {
            try
            {
                var dir = Path.GetDirectoryName(path);
                if (!string.IsNullOrEmpty(dir)) Directory.CreateDirectory(dir);
            }
            catch { }
        }

        /// <summary>
        /// 格式化浮点数值（有值显示1位小数，无值显示"—"）
        /// </summary>
        /// <param name="v">浮点数值</param>
        /// <returns>格式化后的字符串</returns>
        public static string Fmt(float? v) => v.HasValue ? v.Value.ToString("0.0") : "—";
    }
}
