using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using LibreHardwareMonitor.Hardware;
using SensorBridge;

namespace SensorBridge
{
    /// <summary>
    /// 数据收集模块 - 负责从硬件传感器收集各种数据
    /// </summary>
    public static class DataCollector
    {
        /// <summary>
        /// 收集存储（NVMe/SSD）温度
        /// </summary>
        public static List<StorageTemp> CollectStorageTemps(IComputer computer)
        {
            var list = new List<StorageTemp>();
            try
            {
                void CollectFromHw(IHardware hw, string? deviceName)
                {
                    foreach (var s in hw.Sensors)
                    {
                        if (s.SensorType == SensorType.Temperature && s.Value.HasValue)
                        {
                            var v = s.Value.Value;
                            if (v > -50 && v < 150)
                            {
                                var loc = SensorUtils.MapStorageTempName(s.Name);
                                var dev = (deviceName ?? hw.Name ?? string.Empty).Trim();
                                var full = string.IsNullOrEmpty(dev) ? loc : ($"{dev} {loc}");
                                list.Add(new StorageTemp { Name = full, TempC = v });
                            }
                        }
                    }
                    foreach (var sh in hw.SubHardware)
                    {
                        CollectFromHw(sh, deviceName);
                    }
                }

                foreach (var hw in computer.Hardware)
                {
                    if (hw.HardwareType == HardwareType.Storage)
                    {
                        var dev = hw.Name;
                        CollectFromHw(hw, dev);
                    }
                }

                // 不再按名称去重：避免多盘或同盘多位置（复合/控制器/闪存）被合并丢失
            }
            catch { }
            return list;
        }

        /// <summary>
        /// 收集 CPU 每核心指标数据
        /// </summary>
        public static CpuPerCore? CollectCpuPerCore(IComputer computer)
        {
            try
            {
                var loadByIdx = new Dictionary<int, float?>();
                var clockByIdx = new Dictionary<int, double?>();
                var tempByIdx = new Dictionary<int, float?>();

                void Scan(IHardware h)
                {
                    foreach (var s in h.Sensors)
                    {
                        try
                        {
                            var name = s.Name ?? string.Empty;
                            if (!s.Value.HasValue) continue;
                            if (!SensorUtils.TryParseCoreIndex(name, out var idx1)) continue;
                            var v = s.Value.Value;
                            if (s.SensorType == SensorType.Load)
                            {
                                if (v >= 0 && v <= 100)
                                {
                                    if (!loadByIdx.TryGetValue(idx1, out var old) || (old ?? -1) < (float)v)
                                        loadByIdx[idx1] = (float)v;
                                }
                            }
                            else if (s.SensorType == SensorType.Clock)
                            {
                                if (v > 10 && v < 10000)
                                {
                                    if (!clockByIdx.TryGetValue(idx1, out var old) || (old ?? -1) < v)
                                        clockByIdx[idx1] = v;
                                }
                            }
                            else if (s.SensorType == SensorType.Temperature)
                            {
                                if (v > -50 && v < 150)
                                {
                                    if (!tempByIdx.TryGetValue(idx1, out var old) || (old ?? -999) < (float)v)
                                        tempByIdx[idx1] = (float)v;
                                }
                            }
                        }
                        catch { }
                    }
                    foreach (var sh in h.SubHardware) Scan(sh);
                }

                foreach (var hw in computer.Hardware)
                {
                    if (hw.HardwareType == HardwareType.Cpu)
                        Scan(hw);
                }

                int maxIdx = 0;
                if (loadByIdx.Count > 0) maxIdx = Math.Max(maxIdx, loadByIdx.Keys.Max());
                if (clockByIdx.Count > 0) maxIdx = Math.Max(maxIdx, clockByIdx.Keys.Max());
                if (tempByIdx.Count > 0) maxIdx = Math.Max(maxIdx, tempByIdx.Keys.Max());
                if (maxIdx <= 0)
                {
                    return new CpuPerCore();
                }

                var loads = Enumerable.Range(1, maxIdx).Select(i => loadByIdx.ContainsKey(i) ? loadByIdx[i] : (float?)null).ToList();
                var clocks = Enumerable.Range(1, maxIdx).Select(i => clockByIdx.ContainsKey(i) ? clockByIdx[i] : (double?)null).ToList();
                var temps = Enumerable.Range(1, maxIdx).Select(i => tempByIdx.ContainsKey(i) ? tempByIdx[i] : (float?)null).ToList();

                return new CpuPerCore
                {
                    Loads = loads,
                    ClocksMhz = clocks,
                    TempsC = temps,
                };
            }
            catch { return null; }
        }

        /// <summary>
        /// 选择 CPU 温度（取最高值作为包/核心温度）
        /// </summary>
        public static float? PickCpuTemperature(IComputer computer)
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

        /// <summary>
        /// 选择主板温度（优先选择环境温度传感器）
        /// </summary>
        public static float? PickMotherboardTemperature(IComputer computer)
        {
            var preferred = new List<float>();
            var all = new List<float>();
            bool NamePreferred(string? n)
            {
                var name = n ?? string.Empty;
                if (name.Length == 0) return false;
                // 排除 CPU/GPU/SSD/VRM/内存等非主板环境温度
                string[] deny = { "cpu", "core", "package", "gpu", "ssd", "nvme", "hdd", "vrm", "dimm", "memory" };
                foreach (var d in deny)
                    if (name.IndexOf(d, StringComparison.OrdinalIgnoreCase) >= 0) return false;
                // 倾向匹配主板/系统/EC/PCH/机箱等环境温度
                string[] allow = { "motherboard", "mainboard", "system", "pch", "board", "ambient", "chassis", "ec" };
                foreach (var a in allow)
                    if (name.IndexOf(a, StringComparison.OrdinalIgnoreCase) >= 0) return true;
                return false;
            }

            void CollectFromHardware(IHardware hw)
            {
                foreach (var s in hw.Sensors)
                {
                    if (s.SensorType == SensorType.Temperature && s.Value.HasValue)
                    {
                        var v = s.Value.Value;
                        if (v > -50 && v < 150)
                        {
                            all.Add(v);
                            if (NamePreferred(s.Name)) preferred.Add(v);
                        }
                    }
                }
                foreach (var sh in hw.SubHardware)
                    CollectFromHardware(sh);
            }

            foreach (var hw in computer.Hardware)
            {
                // 扩大范围：Motherboard / SuperIO / EmbeddedController 下的温度
                if (hw.HardwareType == HardwareType.Motherboard || hw.HardwareType == HardwareType.SuperIO || hw.HardwareType == HardwareType.EmbeddedController)
                {
                    CollectFromHardware(hw);
                }
            }
            var src = preferred.Count > 0 ? preferred : all;
            if (src.Count == 0) return null;
            var avg = src.Average();
            if (avg < -50 || avg > 150) return null;
            return avg;
        }

        /// <summary>
        /// 收集 GPU 信息
        /// </summary>
        public static List<GpuInfo> CollectGpus(IComputer computer)
        {
            var list = new List<GpuInfo>();
            foreach (var hw in computer.Hardware)
            {
                if (hw.HardwareType != HardwareType.GpuNvidia && hw.HardwareType != HardwareType.GpuAmd && hw.HardwareType != HardwareType.GpuIntel)
                    continue;

                string? gpuName = hw.Name;
                double? tempC = null;
                double? loadPct = null;
                double? coreMhz = null;
                double? fanRpm = null;
                double? powerW = null;
                double? voltageV = null;
                double? vramUsedMB = null;

                foreach (var s in hw.Sensors)
                {
                    if (!s.Value.HasValue) continue;
                    var v = s.Value.Value;
                    var nameLc = (s.Name ?? "").ToLowerInvariant();

                    if (s.SensorType == SensorType.Temperature)
                    {
                        // GPU 温度（°C）：优先选择 GPU Core/Hot Spot/Graphics，范围 -50~150°C
                        if (v > -50 && v < 150)
                        {
                            // 倾向 Core/HotSpot/Graphics，更高者优先
                            var isHotspot = nameLc.Contains("hotspot") || nameLc.Contains("hot spot") || nameLc.Contains("hot");
                            var isCore = nameLc.Contains("core") || nameLc.Contains("gpu core") || nameLc.Contains("graphics");
                            if (isHotspot)
                            {
                                tempC = Math.Max(tempC ?? double.MinValue, v);
                            }
                            else if (isCore)
                            {
                                tempC = Math.Max(tempC ?? double.MinValue, v);
                            }
                            else
                            {
                                tempC = Math.Max(tempC ?? double.MinValue, v);
                            }
                        }
                    }
                    else if (s.SensorType == SensorType.Load)
                    {
                        // GPU 负载（%）：收集所有有效负载传感器，范围 0-100%
                        if (v >= 0 && v <= 100)
                        {
                            // 优先选择 Core/GPU Core 负载，但也收集 D3D 3D 等其他负载
                            if (nameLc.Contains("core") || nameLc.Contains("gpu"))
                                loadPct = Math.Max(loadPct ?? 0.0, v);
                            else if (nameLc.Contains("d3d") && nameLc.Contains("3d"))
                                loadPct = Math.Max(loadPct ?? 0.0, v); // Intel 集成显卡的主要负载指标
                            else
                                loadPct = Math.Max(loadPct ?? 0.0, v);
                        }
                    }
                    else if (s.SensorType == SensorType.Clock)
                    {
                        // MHz，倾向 Core/Graphics
                        if (nameLc.Contains("core") || nameLc.Contains("graphics"))
                        {
                            if (v > 10 && v < 50000)
                                coreMhz = Math.Max(coreMhz ?? 0.0, v);
                        }
                    }
                    else if (s.SensorType == SensorType.Fan)
                    {
                        // GPU 风扇转速（RPM）：范围 0-10000 RPM
                        if (v >= 0 && v <= 10000)
                            fanRpm = Math.Max(fanRpm ?? 0.0, v);
                    }
                    else if (s.SensorType == SensorType.Power)
                    {
                        // GPU 板卡/总功耗（W）
                        if (v > 0 && v < 1000)
                        {
                            // 优先包含 total/board 的命名，否则取最大值
                            if (nameLc.Contains("total") || nameLc.Contains("board"))
                                powerW = Math.Max(powerW ?? 0.0, v);
                            else
                                powerW = Math.Max(powerW ?? 0.0, v);
                        }
                    }
                    else if (s.SensorType == SensorType.Voltage)
                    {
                        // GPU 核心电压（V）：常见命名含 core/vddc/gfx，排除 12V 等
                        // 采用保守范围：0.2 ~ 2.5 V
                        var prefer = nameLc.Contains("core") || nameLc.Contains("vddc") || nameLc.Contains("gfx");
                        if (v >= 0.2 && v <= 2.5)
                        {
                            if (prefer)
                                voltageV = Math.Max(voltageV ?? 0.0, v);
                            else
                                voltageV = Math.Max(voltageV ?? 0.0, v);
                        }
                    }
                    else if (s.SensorType == SensorType.SmallData)
                    {
                        // VRAM 使用量（MB）：常见命名含 memory/vram/used，范围 0-100000 MB
                        if (nameLc.Contains("memory") || nameLc.Contains("vram") || nameLc.Contains("used"))
                        {
                            if (v >= 0 && v <= 100000)
                                vramUsedMB = Math.Max(vramUsedMB ?? 0.0, v);
                        }
                    }
                }

                // 仅当有有效数据时才添加 GPU 信息
                if (tempC.HasValue || loadPct.HasValue || coreMhz.HasValue || fanRpm.HasValue || powerW.HasValue || voltageV.HasValue || vramUsedMB.HasValue)
                {
                    list.Add(new GpuInfo
                    {
                        Name = gpuName,
                        TempC = (float?)tempC,
                        LoadPct = (float?)loadPct,
                        CoreMhz = coreMhz,
                        FanRpm = (int?)fanRpm,
                        PowerW = powerW,
                        VoltageV = voltageV,
                        VramUsedMb = vramUsedMB
                    });
                }
            }
            return list;
        }

        /// <summary>
        /// 收集 CPU 额外信息（功耗、限频等）
        /// </summary>
        public static CpuExtra? CollectCpuExtra(IComputer computer)
        {
            try
            {
                // BRIDGE_THROTTLE_DEFAULT_FALSE=1|true 时生效。
                var envDefaultFalse = Environment.GetEnvironmentVariable("BRIDGE_THROTTLE_DEFAULT_FALSE");
                bool defaultThrottleFalse = !string.IsNullOrEmpty(envDefaultFalse) &&
                    (string.Equals(envDefaultFalse, "1", StringComparison.OrdinalIgnoreCase) ||
                     string.Equals(envDefaultFalse, "true", StringComparison.OrdinalIgnoreCase));

                double? pkgW = null;
                var coreClocks = new List<double>();
                bool? throttle = null;
                bool throttleSeen = false; // 是否见到限频相关传感器（即使当前未触发）
                var reasons = new List<string>();

                foreach (var hw in computer.Hardware)
                {
                    bool isCpuRoot = hw.HardwareType == HardwareType.Cpu;
                    // 遍历所有硬件，但仅对 CPU 硬件统计功耗/频率；限频标志在任意硬件上都可能出现，均需扫描
                    void ScanHw(IHardware h, bool isCpu)
                    {
                        foreach (var s in h.Sensors)
                        {
                            try
                            {
                                var t = s.SensorType;
                                var name = s.Name ?? string.Empty;
                                if (!s.Value.HasValue) continue;
                                var v = s.Value.Value;
                                if (t == SensorType.Power && isCpu)
                                {
                                    // 优先选择名称包含 Package 的功耗；否则取最大值作为包功耗近似
                                    if (name.IndexOf("package", StringComparison.OrdinalIgnoreCase) >= 0
                                        || name.IndexOf("cpu package", StringComparison.OrdinalIgnoreCase) >= 0
                                        || name.IndexOf("pkg", StringComparison.OrdinalIgnoreCase) >= 0)
                                    {
                                        pkgW = v;
                                    }
                                    else
                                    {
                                        pkgW = Math.Max(pkgW ?? 0.0, v);
                                    }
                                    // 电源限制/热限提示
                                    if (name.IndexOf("limit", StringComparison.OrdinalIgnoreCase) >= 0)
                                    {
                                        throttleSeen = true;
                                        if (v > 0)
                                        {
                                            throttle = true;
                                            reasons.Add(name);
                                        }
                                    }
                                }
                                else if (t == SensorType.Clock && isCpu)
                                {
                                    // 收集 Core/Efficient/E-core/P-core 频率（MHz）
                                    if (name.IndexOf("core", StringComparison.OrdinalIgnoreCase) >= 0
                                        || name.IndexOf("effective", StringComparison.OrdinalIgnoreCase) >= 0
                                        || name.IndexOf("p-core", StringComparison.OrdinalIgnoreCase) >= 0
                                        || name.IndexOf("e-core", StringComparison.OrdinalIgnoreCase) >= 0)
                                    {
                                        if (v > 10 && v < 10000)
                                            coreClocks.Add(v);
                                    }
                                }
                                else if (t == SensorType.Load)
                                {
                                    // 某些平台会以 Load 名称提示 thermal/power throttling（极少见），仅作为提示保留
                                    if (name.IndexOf("thrott", StringComparison.OrdinalIgnoreCase) >= 0 && v > 0)
                                    {
                                        throttle = true;
                                        reasons.Add(name);
                                    }
                                }
                                else if (t == SensorType.Factor)
                                {
                                    // 许多限频标志以 Factor(0/1) 暴露：Thermal Throttling, Power Limit Exceeded, PROCHOT, PL1/PL2/EDP Limit, Tau 等
                                    bool maybeThrottleFlag =
                                        name.IndexOf("thrott", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                        name.IndexOf("limit", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                        name.IndexOf("prochot", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                        name.IndexOf("pl1", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                        name.IndexOf("pl2", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                        name.IndexOf("edp", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                        name.IndexOf("tau", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                        name.IndexOf("thermal", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                        name.IndexOf("vr", StringComparison.OrdinalIgnoreCase) >= 0;
                                    if (maybeThrottleFlag)
                                    {
                                        throttleSeen = true;
                                        if (v > 0.5)
                                        {
                                            throttle = true;
                                            reasons.Add(name);
                                        }
                                    }
                                }
                            }
                            catch { }
                        }
                        foreach (var sh in h.SubHardware) ScanHw(sh, isCpu);
                    }
                    ScanHw(hw, isCpuRoot);
                }

                // 若扫描到限频相关传感器但未触发，或启用了默认 false 策略，则明确标记为 false（而不是 null）
                if (throttle == null && (throttleSeen || defaultThrottleFalse))
                    throttle = false;

                var extra = new CpuExtra
                {
                    PkgPowerW = pkgW,
                    AvgCoreMhz = coreClocks.Count > 0 ? coreClocks.Average() : (double?)null,
                    ThrottleActive = throttle,
                    ThrottleReasons = reasons.Count > 0 ? reasons.Distinct().ToList() : null,
                };
                // 若完全无数据则返回 null，避免冗余字段
                if (extra.PkgPowerW == null && extra.AvgCoreMhz == null && extra.ThrottleActive == null && (extra.ThrottleReasons == null || extra.ThrottleReasons.Count == 0))
                    return null;
                return extra;
            }
            catch { return null; }
        }

        /// <summary>
        /// 收集风扇信息（基础版本）
        /// </summary>
        public static List<FanInfo> CollectFans(IComputer computer)
        {
            var list = new List<FanInfo>();
            try
            {
                foreach (var hw in computer.Hardware)
                {
                    void ScanHardware(IHardware h)
                    {
                        foreach (var s in h.Sensors)
                        {
                            if (s.SensorType == SensorType.Fan && s.Value.HasValue)
                            {
                                var v = s.Value.Value;
                                if (v >= 0 && v <= 10000) // 合理的风扇转速范围
                                {
                                    list.Add(new FanInfo
                                    {
                                        Name = s.Name,
                                        Rpm = (int?)v,
                                        Pct = null // 基础版本不收集百分比
                                    });
                                }
                            }
                        }
                        foreach (var sh in h.SubHardware)
                            ScanHardware(sh);
                    }
                    ScanHardware(hw);
                }
            }
            catch { }
            return list;
        }

        /// <summary>
        /// 收集风扇信息（扩展版本，包含控制百分比）
        /// </summary>
        public static List<FanInfo> CollectFansRaw(IComputer computer)
        {
            var list = new List<FanInfo>();
            try
            {
                foreach (var hw in computer.Hardware)
                {
                    void ScanHardware(IHardware h)
                    {
                        foreach (var s in h.Sensors)
                        {
                            if (s.Value.HasValue)
                            {
                                var v = s.Value.Value;
                                var name = s.Name ?? "";
                                
                                if (s.SensorType == SensorType.Fan && v >= 0 && v <= 10000)
                                {
                                    list.Add(new FanInfo
                                    {
                                        Name = name,
                                        Rpm = (int?)v,
                                        Pct = null
                                    });
                                }
                                else if (s.SensorType == SensorType.Control && SensorUtils.IsFanLikeControl(s) && v >= 0 && v <= 100)
                                {
                                    list.Add(new FanInfo
                                    {
                                        Name = name,
                                        Rpm = null,
                                        Pct = (int?)v
                                    });
                                }
                            }
                        }
                        foreach (var sh in h.SubHardware)
                            ScanHardware(sh);
                    }
                    ScanHardware(hw);
                }
            }
            catch { }
            return list;
        }

        /// <summary>
        /// 收集主板电压信息
        /// </summary>
        public static List<VoltageInfo> CollectMoboVoltages(IComputer computer)
        {
            var list = new List<VoltageInfo>();
            try
            {
                foreach (var hw in computer.Hardware)
                {
                    // 主要从主板、SuperIO、嵌入式控制器收集电压
                    if (hw.HardwareType == HardwareType.Motherboard || 
                        hw.HardwareType == HardwareType.SuperIO || 
                        hw.HardwareType == HardwareType.EmbeddedController)
                    {
                        void ScanHardware(IHardware h)
                        {
                            foreach (var s in h.Sensors)
                            {
                                if (s.SensorType == SensorType.Voltage && s.Value.HasValue)
                                {
                                    var v = s.Value.Value;
                                    var name = s.Name ?? "";
                                    var nameLc = name.ToLowerInvariant();
                                    
                                    // 过滤合理的电压范围：0.1V ~ 20V
                                    if (v >= 0.1 && v <= 20.0)
                                    {
                                        list.Add(new VoltageInfo
                                        {
                                            Name = name,
                                            Volts = v
                                        });
                                    }
                                }
                            }
                            foreach (var sh in h.SubHardware)
                                ScanHardware(sh);
                        }
                        ScanHardware(hw);
                    }
                }

                // 同名传感器保留最大值
                var dedup = list
                    .GroupBy(x => x.Name ?? "")
                    .Select(g => new VoltageInfo
                    {
                        Name = string.IsNullOrEmpty(g.Key) ? null : g.Key,
                        Volts = g.Max(x => x.Volts)
                    })
                    .Where(x => x.Volts.HasValue)
                    .ToList();
                return dedup;
            }
            catch { }
            return new List<VoltageInfo>();
        }
    }
}
