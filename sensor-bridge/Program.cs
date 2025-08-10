using System.Text.Json;
using System.Text;
using LibreHardwareMonitor.Hardware;
using System.Linq;
using System.Security.Principal;

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

        var computer = new Computer
        {
            IsCpuEnabled = true,
            IsMotherboardEnabled = true,
            IsControllerEnabled = true,
            IsMemoryEnabled = false,
            IsStorageEnabled = false,
            IsNetworkEnabled = false,
            IsGpuEnabled = false,
        };
        computer.Open();

        int tick = 0;
        while (true)
        {
            try
            {
                // 使用访问者统一刷新全树
                computer.Accept(new UpdateVisitor());
                if (tick % 10 == 0) DumpSensors(computer);

                float? cpuTemp = PickCpuTemperature(computer);
                float? moboTemp = PickMotherboardTemperature(computer);
                var fans = CollectFans(computer);

                // Flags
                bool anyTempSensor = HasSensor(computer, SensorType.Temperature);
                bool anyTempValue = HasSensorValue(computer, SensorType.Temperature);
                bool anyFanSensor = HasSensor(computer, SensorType.Fan) || HasFanLikeControl(computer);
                bool anyFanValue = HasSensorValue(computer, SensorType.Fan) || HasFanLikeControlWithValue(computer);
                bool isAdmin = new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator);

                var payload = new
                {
                    cpuTempC = cpuTemp,
                    moboTempC = moboTemp,
                    fans = fans.Count > 0 ? fans : null,
                    isAdmin = isAdmin,
                    hasTemp = anyTempSensor,
                    hasTempValue = anyTempValue,
                    hasFan = anyFanSensor,
                    hasFanValue = anyFanValue,
                };

                Console.WriteLine(JsonSerializer.Serialize(payload, jsonOptions));
                Console.Out.Flush();
            }
            catch
            {
                // Swallow and continue next tick
            }
            tick++;
            Thread.Sleep(1000);
        }
    }

    static bool HasSensor(IComputer computer, SensorType type)
    {
        foreach (var hw in computer.Hardware)
        {
            if (hw.Sensors.Any(s => s.SensorType == type)) return true;
            foreach (var sh in hw.SubHardware)
                if (sh.Sensors.Any(s => s.SensorType == type)) return true;
        }
        return false;
    }

    static bool HasSensorValue(IComputer computer, SensorType type)
    {
        foreach (var hw in computer.Hardware)
        {
            if (hw.Sensors.Any(s => s.SensorType == type && s.Value.HasValue)) return true;
            foreach (var sh in hw.SubHardware)
                if (sh.Sensors.Any(s => s.SensorType == type && s.Value.HasValue)) return true;
        }
        return false;
    }

    static bool HasFanLikeControl(IComputer computer)
    {
        foreach (var hw in computer.Hardware)
        {
            if (hw.Sensors.Any(IsFanLikeControl)) return true;
            foreach (var sh in hw.SubHardware)
                if (sh.Sensors.Any(IsFanLikeControl)) return true;
        }
        return false;
    }

    static bool HasFanLikeControlWithValue(IComputer computer)
    {
        foreach (var hw in computer.Hardware)
        {
            if (hw.Sensors.Any(s => IsFanLikeControl(s) && s.Value.HasValue)) return true;
            foreach (var sh in hw.SubHardware)
                if (sh.Sensors.Any(s => IsFanLikeControl(s) && s.Value.HasValue)) return true;
        }
        return false;
    }

    static bool IsFanLikeControl(ISensor s)
    {
        if (s.SensorType != SensorType.Control) return false;
        var name = s.Name ?? string.Empty;
        return name.IndexOf("fan", StringComparison.OrdinalIgnoreCase) >= 0
            || name.IndexOf("pwm", StringComparison.OrdinalIgnoreCase) >= 0
            || name.IndexOf("duty", StringComparison.OrdinalIgnoreCase) >= 0
            || name.IndexOf("cool", StringComparison.OrdinalIgnoreCase) >= 0;
    }
    // 访问者：递归刷新所有硬件与子硬件
    class UpdateVisitor : IVisitor
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

    static void DumpSensors(IComputer computer)
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

    static float? PickCpuTemperature(IComputer computer)
    {
        var temps = new List<float>();
        foreach (var hw in computer.Hardware)
        {
            if (hw.HardwareType != HardwareType.Cpu) continue;
            foreach (var s in hw.Sensors)
            {
                if (s.SensorType == SensorType.Temperature && s.Value.HasValue)
                {
                    temps.Add(s.Value.Value);
                }
            }
            foreach (var sh in hw.SubHardware)
            {
                foreach (var s in sh.Sensors)
                {
                    if (s.SensorType == SensorType.Temperature && s.Value.HasValue)
                    {
                        temps.Add(s.Value.Value);
                    }
                }
            }
        }
        if (temps.Count == 0) return null;
        // Prefer higher temps as package/core; cap to plausible range
        var v = temps.Max();
        if (v < -50 || v > 150) return null;
        return v;
    }

    static float? PickMotherboardTemperature(IComputer computer)
    {
        var temps = new List<float>();
        foreach (var hw in computer.Hardware)
        {
            if (hw.HardwareType != HardwareType.Motherboard && hw.HardwareType != HardwareType.SuperIO) continue;
            foreach (var s in hw.Sensors)
            {
                if (s.SensorType == SensorType.Temperature && s.Value.HasValue)
                {
                    temps.Add(s.Value.Value);
                }
            }
            foreach (var sh in hw.SubHardware)
            {
                foreach (var s in sh.Sensors)
                {
                    if (s.SensorType == SensorType.Temperature && s.Value.HasValue)
                    {
                        temps.Add(s.Value.Value);
                    }
                }
            }
        }
        if (temps.Count == 0) return null;
        var v = temps.Average();
        if (v < -50 || v > 120) return null;
        return v;
    }

    class FanInfo
    {
        public string? Name { get; set; }
        public int? Rpm { get; set; }
        public int? Pct { get; set; }
    }

    static List<FanInfo> CollectFans(IComputer computer)
    {
        var fans = new List<FanInfo>();
        foreach (var hw in computer.Hardware)
        {
            // 直接遍历所有硬件与子硬件
            foreach (var s in hw.Sensors)
            {
                if (s.SensorType == SensorType.Fan)
                {
                    int? rpm = s.Value.HasValue ? (int?)Math.Round(s.Value.Value) : null;
                    fans.Add(new FanInfo { Name = s.Name, Rpm = rpm });
                }
                else if (s.SensorType == SensorType.Control)
                {
                    // 仅抓取名称含 fan 的控制量作为占空比
                    var name = s.Name ?? string.Empty;
                    if (name.IndexOf("fan", StringComparison.OrdinalIgnoreCase) >= 0)
                    {
                        int? pct = s.Value.HasValue ? (int?)Math.Round(s.Value.Value) : null; // 0~100
                        fans.Add(new FanInfo { Name = s.Name, Pct = pct });
                    }
                }
            }
            foreach (var sh in hw.SubHardware)
            {
                foreach (var s in sh.Sensors)
                {
                    if (s.SensorType == SensorType.Fan)
                    {
                        int? rpm = s.Value.HasValue ? (int?)Math.Round(s.Value.Value) : null;
                        fans.Add(new FanInfo { Name = s.Name, Rpm = rpm });
                    }
                    else if (s.SensorType == SensorType.Control)
                    {
                        var name = s.Name ?? string.Empty;
                        if (name.IndexOf("fan", StringComparison.OrdinalIgnoreCase) >= 0)
                        {
                            int? pct = s.Value.HasValue ? (int?)Math.Round(s.Value.Value) : null;
                            fans.Add(new FanInfo { Name = s.Name, Pct = pct });
                        }
                    }
                }
            }
        }
        // 按名称去重：保留最大 RPM 和最大占空比
        var dedup = fans
            .GroupBy(f => f.Name ?? "")
            .Select(g => new FanInfo {
                Name = string.IsNullOrEmpty(g.Key) ? null : g.Key,
                Rpm = g.Max(x => x.Rpm),
                Pct = g.Max(x => x.Pct)
            })
            .Where(f => f.Rpm.HasValue || f.Pct.HasValue)
            .ToList();
        return dedup;
    }
}
