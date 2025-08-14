using System;
using System.Text;
using LibreHardwareMonitor.Hardware;

namespace SensorBridge
{
    /// <summary>
    /// 硬件管理模块 - 负责硬件初始化、更新和调试输出
    /// </summary>
    public static class HardwareManager
    {
        /// <summary>
        /// 创建并初始化硬件监控计算机实例
        /// </summary>
        /// <returns>已初始化的 Computer 实例</returns>
        public static Computer MakeComputer()
        {
            var c = new Computer
            {
                IsCpuEnabled = true,
                IsMotherboardEnabled = true,
                IsControllerEnabled = true,
                IsMemoryEnabled = false,
                IsStorageEnabled = true,
                IsNetworkEnabled = false,
                IsGpuEnabled = true,
            };
            c.Open();
            return c;
        }

        /// <summary>
        /// 转储所有传感器信息到错误输出流（用于调试）
        /// </summary>
        /// <param name="computer">要转储的计算机实例</param>
        public static void DumpSensors(IComputer computer)
        {
            try
            {
                var sb = new StringBuilder();
                sb.AppendLine("[bridge][dump] sensors:");
                foreach (var hw in computer.Hardware)
                {
                    sb.AppendLine($"- HW {hw.HardwareType} | {hw.Name}");
                    foreach (var s in hw.Sensors)
                    {
                        sb.AppendLine($"  * {s.SensorType} | {s.Name} = {s.Value}");
                    }
                    foreach (var sh in hw.SubHardware)
                    {
                        sb.AppendLine($"  - Sub {sh.HardwareType} | {sh.Name}");
                        foreach (var s in sh.Sensors)
                        {
                            sb.AppendLine($"    * {s.SensorType} | {s.Name} = {s.Value}");
                        }
                    }
                }
                Console.Error.WriteLine(sb.ToString());
                Console.Error.Flush();
            }
            catch { }
        }
    }

    /// <summary>
    /// 硬件更新访问者 - 递归刷新所有硬件与子硬件
    /// </summary>
    public class UpdateVisitor : IVisitor
    {
        public void VisitComputer(IComputer computer) => computer.Traverse(this);
        
        public void VisitHardware(IHardware hardware)
        {
            hardware.Update();
            foreach (var sh in hardware.SubHardware)
                sh.Accept(this);
        }
        
        public void VisitSensor(ISensor sensor) { }
        
        public void VisitParameter(IParameter parameter) { }
    }
}
