using System;
using System.Linq;
using System.Text.RegularExpressions;
using LibreHardwareMonitor.Hardware;

namespace SensorBridge
{
    /// <summary>
    /// 传感器工具类：提供传感器检查、风扇控制判断、存储温度映射等工具函数
    /// </summary>
    public static class SensorUtils
    {
        /// <summary>
        /// 检查计算机是否有指定类型的传感器
        /// </summary>
        /// <param name="computer">计算机对象</param>
        /// <param name="type">传感器类型</param>
        /// <returns>是否存在该类型传感器</returns>
        public static bool HasSensor(IComputer computer, SensorType type)
        {
            foreach (var hw in computer.Hardware)
            {
                if (hw.Sensors.Any(s => s.SensorType == type)) return true;
                foreach (var sh in hw.SubHardware)
                    if (sh.Sensors.Any(s => s.SensorType == type)) return true;
            }
            return false;
        }

        /// <summary>
        /// 检查计算机是否有指定类型且有值的传感器
        /// </summary>
        /// <param name="computer">计算机对象</param>
        /// <param name="type">传感器类型</param>
        /// <returns>是否存在该类型且有值的传感器</returns>
        public static bool HasSensorValue(IComputer computer, SensorType type)
        {
            foreach (var hw in computer.Hardware)
            {
                if (hw.Sensors.Any(s => s.SensorType == type && s.Value.HasValue)) return true;
                foreach (var sh in hw.SubHardware)
                    if (sh.Sensors.Any(s => s.SensorType == type && s.Value.HasValue)) return true;
            }
            return false;
        }

        /// <summary>
        /// 检查当前进程是否以管理员权限运行
        /// </summary>
        /// <returns>是否为管理员权限</returns>
        public static bool IsAdministrator()
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

        /// <summary>
        /// 检查计算机是否有风扇类控制传感器
        /// </summary>
        /// <param name="computer">计算机对象</param>
        /// <returns>是否存在风扇类控制传感器</returns>
        public static bool HasFanLikeControl(IComputer computer)
        {
            foreach (var hw in computer.Hardware)
            {
                if (hw.Sensors.Any(IsFanLikeControl)) return true;
                foreach (var sh in hw.SubHardware)
                    if (sh.Sensors.Any(IsFanLikeControl)) return true;
            }
            return false;
        }

        /// <summary>
        /// 检查计算机是否有风扇类控制传感器且有值
        /// </summary>
        /// <param name="computer">计算机对象</param>
        /// <returns>是否存在有值的风扇类控制传感器</returns>
        public static bool HasFanLikeControlWithValue(IComputer computer)
        {
            foreach (var hw in computer.Hardware)
            {
                if (hw.Sensors.Any(s => IsFanLikeControl(s) && s.Value.HasValue)) return true;
                foreach (var sh in hw.SubHardware)
                    if (sh.Sensors.Any(s => IsFanLikeControl(s) && s.Value.HasValue)) return true;
            }
            return false;
        }

        /// <summary>
        /// 判断传感器是否为风扇类控制传感器
        /// </summary>
        /// <param name="s">传感器对象</param>
        /// <returns>是否为风扇类控制传感器</returns>
        public static bool IsFanLikeControl(ISensor s)
        {
            if (s.SensorType != SensorType.Control) return false;
            var name = s.Name ?? string.Empty;
            // 常规命名匹配
            if (name.IndexOf("fan", StringComparison.OrdinalIgnoreCase) >= 0
                || name.IndexOf("pwm", StringComparison.OrdinalIgnoreCase) >= 0
                || name.IndexOf("duty", StringComparison.OrdinalIgnoreCase) >= 0
                || name.IndexOf("cool", StringComparison.OrdinalIgnoreCase) >= 0)
                return true;

            // 兼容部分 NUC/EC：在 EC / Motherboard / SuperIO 下，Control 传感器若数值在 [0,100]，也视为风扇占空比
            try
            {
                var hwType = s.Hardware?.HardwareType;
                if (hwType == HardwareType.EmbeddedController || hwType == HardwareType.Motherboard || hwType == HardwareType.SuperIO)
                {
                    if (s.Value.HasValue)
                    {
                        var v = s.Value.Value;
                        if (v >= 0 && v <= 100) return true;
                    }
                }
            }
            catch { }
            return false;
        }

        /// <summary>
        /// 存储温度名称映射：将通用英文名转换为更具体的位置名称
        /// </summary>
        /// <param name="sensorName">传感器名称</param>
        /// <returns>映射后的名称</returns>
        public static string MapStorageTempName(string? sensorName)
        {
            var n = sensorName?.Trim() ?? string.Empty;
            if (string.IsNullOrEmpty(n)) return "温度";

            // 标准别名优先
            if (n.Equals("Temperature", StringComparison.OrdinalIgnoreCase)
                || n.IndexOf("Composite", StringComparison.OrdinalIgnoreCase) >= 0
                || n.IndexOf("Drive Temperature", StringComparison.OrdinalIgnoreCase) >= 0)
                return "复合"; // NVMe Composite/盘体综合温度

            if (n.Equals("Temperature 1", StringComparison.OrdinalIgnoreCase)
                || n.IndexOf("Controller", StringComparison.OrdinalIgnoreCase) >= 0)
                return "控制器";

            if (n.Equals("Temperature 2", StringComparison.OrdinalIgnoreCase)
                || n.IndexOf("NAND", StringComparison.OrdinalIgnoreCase) >= 0
                || n.IndexOf("Memory", StringComparison.OrdinalIgnoreCase) >= 0
                || n.IndexOf("Flash", StringComparison.OrdinalIgnoreCase) >= 0)
                return "闪存";

            if (n.IndexOf("Drive", StringComparison.OrdinalIgnoreCase) >= 0)
                return "盘体";

            // 未知则原样返回
            return n;
        }

        /// <summary>
        /// 尝试从传感器名称解析核心索引（1-based）
        /// </summary>
        /// <param name="name">传感器名称</param>
        /// <param name="index1Based">解析出的1-based索引</param>
        /// <returns>是否成功解析</returns>
        public static bool TryParseCoreIndex(string? name, out int index1Based)
        {
            index1Based = -1;
            var n = name ?? string.Empty;
            if (n.Length == 0) return false;
            try
            {
                // 优先匹配 "#<num>"
                var m = Regex.Match(n, @"#\s*(?<idx>\d+)", RegexOptions.IgnoreCase);
                if (m.Success && int.TryParse(m.Groups["idx"].Value, out var idx1) && idx1 > 0)
                {
                    index1Based = idx1;
                    return true;
                }
                // 兼容 "Core 1", "CPU Core 2", "Core #3", "P-Core 4", "E-Core 5"
                m = Regex.Match(n, @"(?:cpu\s*)?(?:p-?core|e-?core|core)\s*#?\s*(?<idx>\d+)", RegexOptions.IgnoreCase);
                if (m.Success && int.TryParse(m.Groups["idx"].Value, out idx1) && idx1 > 0)
                {
                    index1Based = idx1;
                    return true;
                }
                return false;
            }
            catch { return false; }
        }
    }
}
