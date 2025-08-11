// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// 汇总磁盘 IOPS 与队列长度（排除 _Total）
fn wmi_perf_disk(conn: &wmi::WMIConnection) -> (Option<f64>, Option<f64>, Option<f64>) {
    let res: Result<Vec<PerfDiskPhysical>, _> = conn.query();
    if let Ok(list) = res {
        let mut r = 0.0f64; let mut w = 0.0f64; let mut q = 0.0f64; let mut n = 0u32;
        for it in list.into_iter() {
            let name = it.name.as_deref().unwrap_or("");
            if name == "_Total" { continue; }
            if let Some(v) = it.disk_reads_per_sec { if v.is_finite() { r += v; } }
            if let Some(v) = it.disk_writes_per_sec { if v.is_finite() { w += v; } }
            if let Some(v) = it.current_disk_queue_length { if v.is_finite() { q += v; n += 1; } }
        }
        let q_avg = if n > 0 { Some(q / (n as f64)) } else { None };
        let r_o = if r > 0.0 { Some(r) } else { Some(0.0) };
        let w_o = if w > 0.0 { Some(w) } else { Some(0.0) };
        return (r_o, w_o, q_avg);
    }
    (None, None, None)
}

// 汇总网络错误率（每秒，排除 _Total）
fn wmi_perf_net_err(conn: &wmi::WMIConnection) -> (Option<f64>, Option<f64>) {
    let res: Result<Vec<PerfTcpipNic>, _> = conn.query();
    if let Ok(list) = res {
        let mut rx = 0.0f64; let mut tx = 0.0f64;
        for it in list.into_iter() {
            let name = it.name.as_deref().unwrap_or("");
            if name == "_Total" { continue; }
            if let Some(v) = it.packets_received_errors { rx += v as f64; }
            if let Some(v) = it.packets_outbound_errors { tx += v as f64; }
        }
        return (Some(rx), Some(tx));
    }
    (None, None)
}

fn tcp_rtt_ms(addr: &str, timeout_ms: u64) -> Option<f64> {
    use std::net::ToSocketAddrs;
    use std::time::Instant;
    let mut addrs_iter = match addr.to_socket_addrs() { Ok(it) => it, Err(_) => return None };
    if let Some(sa) = addrs_iter.next() {
        let dur = std::time::Duration::from_millis(timeout_ms);
        let start = Instant::now();
        if std::net::TcpStream::connect_timeout(&sa, dur).is_ok() {
            let rtt = start.elapsed().as_secs_f64() * 1000.0;
            return Some(rtt);
        }
    }
    None
}

// 控制台输出解码助手：优先 UTF-8，失败则回退 GBK（中文 Windows 常见），最后退回损失性 UTF-8
fn decode_console_bytes(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let (cow, _, had_errors) = encoding_rs::GBK.decode(bytes);
    if !had_errors {
        return cow.to_string();
    }
    // 仍有错误则使用损失性转换
    String::from_utf8_lossy(bytes).to_string()
}

// ---- Wi-Fi helpers (Windows: parse `netsh wlan show interfaces`) ----
#[derive(Clone, Debug, Default)]
struct WifiInfoExt {
    ssid: Option<String>,
    signal_pct: Option<i32>,
    link_mbps: Option<i32>,
    bssid: Option<String>,
    channel: Option<i32>,
    radio: Option<String>,
    band: Option<String>,
    rx_mbps: Option<i32>,
    tx_mbps: Option<i32>,
    rssi_dbm: Option<i32>,
    // 新增：标记 rssi_dbm 是否为根据 Signal% 估算
    rssi_estimated: bool,
}

#[allow(dead_code)]
fn read_wifi_info() -> (Option<String>, Option<i32>, Option<i32>) {
    #[cfg(windows)]
    {
        let output = std::process::Command::new("netsh")
            .args(["wlan", "show", "interfaces"])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let text = decode_console_bytes(&out.stdout);
                let mut ssid: Option<String> = None;
                let mut signal_pct: Option<i32> = None;
                let mut rx_mbps: Option<i32> = None;
                let mut tx_mbps: Option<i32> = None;

                for line in text.lines() {
                    let t = line.trim();
                    let tl = t.to_lowercase();
                    // SSID（避免匹配到 BSSID）
                    if tl.starts_with("ssid") && !tl.starts_with("bssid") {
                        if let Some(pos) = t.find(':') {
                            let v = t[pos + 1..].trim();
                            if !v.is_empty() { ssid = Some(v.to_string()); }
                        }
                        continue;
                    }
                    // 信号强度："Signal" 或 中文 "信号"
                    if tl.contains("signal") || t.contains("信号") {
                        if let Some(pos) = t.find(':') {
                            let v = t[pos + 1..].trim();
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { signal_pct = Some(n.clamp(0, 100)); }
                        }
                        continue;
                    }
                    // 速率：接收/发送（英/中文）
                    if tl.contains("receive rate (mbps)") || t.contains("接收速率 (Mbps)") {
                        if let Some(pos) = t.find(':') {
                            let v = t[pos + 1..].trim();
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { rx_mbps = Some(n.max(0)); }
                        }
                        continue;
                    }
                    if tl.contains("transmit rate (mbps)") || t.contains("传输速率 (Mbps)") {
                        if let Some(pos) = t.find(':') {
                            let v = t[pos + 1..].trim();
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { tx_mbps = Some(n.max(0)); }
                        }
                        continue;
                    }
                }

                let link = rx_mbps.or(tx_mbps);
                return (ssid, signal_pct, link);
            }
        }
        (None, None, None)
    }
    #[cfg(not(windows))]
    {
        (None, None, None)
    }
}

fn read_wifi_info_ext() -> WifiInfoExt {
    #[cfg(windows)]
    {
        let mut out = WifiInfoExt::default();
        let output = std::process::Command::new("netsh")
            .args(["wlan", "show", "interfaces"]) 
            .output();
        if let Ok(outp) = output {
            if outp.status.success() {
                let text = decode_console_bytes(&outp.stdout);
                let raw_text_for_dbg = if cfg!(debug_assertions) { Some(text.clone()) } else { None };
                let mut rx_mbps: Option<i32> = None;
                let mut tx_mbps: Option<i32> = None;
                // 兼容中文冒号，返回所有权字符串以避免生命周期问题
                let get_after_colon = |s: &str| -> Option<String> {
                    if let Some(pos) = s.find(':') { return Some(s[pos + 1..].trim().to_string()); }
                    if let Some(pos2) = s.find('：') { return Some(s[pos2 + 1..].trim().to_string()); }
                    None
                };
                for line in text.lines() {
                    let t = line.trim();
                    let tl = t.to_lowercase();
                    // SSID（避免匹配到 BSSID）
                    if tl.starts_with("ssid") && !tl.starts_with("bssid") {
                        if let Some(v) = get_after_colon(t) { if !v.is_empty() { out.ssid = Some(v); } }
                        continue;
                    }
                    // BSSID
                    if tl.starts_with("bssid") {
                        if let Some(v) = get_after_colon(t) { if !v.is_empty() { out.bssid = Some(v); } }
                        continue;
                    }
                    // 信号强度（含“信号质量”）
                    if tl.contains("signal") || t.contains("信号") || t.contains("信号质量") {
                        if let Some(v) = get_after_colon(t) {
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { out.signal_pct = Some(n.clamp(0, 100)); }
                        }
                        continue;
                    }
                    // 信道（放宽匹配：channel/信道/通道/频道）
                    if tl.contains("channel") || t.contains("信道") || t.contains("通道") || t.contains("频道") {
                        if let Some(v) = get_after_colon(t) {
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { out.channel = Some(n.max(0)); }
                        }
                        continue;
                    }
                    // 无线制式（放宽匹配：Radio type/无线电类型/无线类型/物理类型）
                    if tl.contains("radio type") || t.contains("无线电类型") || t.contains("无线类型") || t.contains("物理类型") {
                        if let Some(v) = get_after_colon(t) { if !v.is_empty() { out.radio = Some(v); } }
                        continue;
                    }
                    // RSSI（部分系统会展示为 RSSI 或 信号质量）
                    if tl.starts_with("rssi") {
                        if let Some(v) = get_after_colon(t) {
                            // 可能为 "-45 dBm"
                            let mut s = String::new();
                            for ch in v.chars() { if ch == '-' || ch.is_ascii_digit() { s.push(ch); } }
                            if let Ok(n) = s.parse::<i32>() { out.rssi_dbm = Some(n); out.rssi_estimated = false; }
                        }
                        continue;
                    }
                    // 速率：接收（英/中文，放宽空格/大小写/括号）
                    if (tl.contains("receive rate") && tl.contains("mbps")) || t.contains("接收速率") {
                        if let Some(v) = get_after_colon(t) {
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { rx_mbps = Some(n.max(0)); }
                        }
                        continue;
                    }
                    // 速率：发送（英/中文，放宽空格/大小写/括号），兼容“传输速率/发送速率”
                    if (tl.contains("transmit rate") && tl.contains("mbps")) || t.contains("传输速率") || t.contains("发送速率") {
                        if let Some(v) = get_after_colon(t) {
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { tx_mbps = Some(n.max(0)); }
                        }
                        continue;
                    }
                }

                out.rx_mbps = rx_mbps;
                out.tx_mbps = tx_mbps;
                // 若能从信道推断频段
                if out.band.is_none() {
                    if let Some(ch) = out.channel {
                        out.band = Some(
                            if (1..=14).contains(&ch) { "2.4GHz".to_string() }
                            else if (32..=177).contains(&ch) { "5GHz".to_string() }
                            else { "".to_string() }
                        ).filter(|s| !s.is_empty());
                    }
                }
                // 若无原生 RSSI，则基于 Signal% 估算：RSSI ~= round(signal/2 - 100)
                if out.rssi_dbm.is_none() {
                    if let Some(q) = out.signal_pct { // 0..100
                        let est = (q as f64 / 2.0 - 100.0).round() as i32;
                        out.rssi_dbm = Some(est);
                        out.rssi_estimated = true;
                    }
                }
                // Debug 构建下输出解析摘要，便于现场排错
                if cfg!(debug_assertions) {
                    if out.signal_pct.is_none() && out.channel.is_none() && out.radio.is_none() && out.rx_mbps.is_none() && out.tx_mbps.is_none() && out.rssi_dbm.is_none() {
                        if let Some(raw) = raw_text_for_dbg.as_ref() {
                            println!("[wifi][raw]\n{}", raw);
                        }
                    }
                    println!(
                        "[wifi][parsed] ssid={:?} signal%={:?} ch={:?} radio={:?} band={:?} rx={:?} tx={:?} bssid={:?} rssi={:?} rssi_est={}",
                        out.ssid, out.signal_pct, out.channel, out.radio, out.band, out.rx_mbps, out.tx_mbps, out.bssid, out.rssi_dbm, out.rssi_estimated
                    );
                }
                return out;
            }
        }
        out
    }
    #[cfg(not(windows))]
    {
        WifiInfoExt::default()
    }
}
// ---- WMI helpers: temperature & fan ----
#[derive(serde::Deserialize, Debug)]
struct MSAcpiThermalZoneTemperature {
    #[serde(rename = "CurrentTemperature")] 
    current_temperature: Option<i64>,
}

#[derive(serde::Deserialize, Debug)]
struct Win32Fan {
    #[serde(rename = "DesiredSpeed")]
    desired_speed: Option<u64>,
}

// ---- WMI Perf counters (disk & network) ----
#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_PerfFormattedData_PerfDisk_PhysicalDisk")]
struct PerfDiskPhysical {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "DiskReadsPerSec")]
    disk_reads_per_sec: Option<f64>,
    #[serde(rename = "DiskWritesPerSec")]
    disk_writes_per_sec: Option<f64>,
    #[serde(rename = "CurrentDiskQueueLength")]
    current_disk_queue_length: Option<f64>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_PerfFormattedData_Tcpip_NetworkInterface")]
struct PerfTcpipNic {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "PacketsReceivedErrors")]
    packets_received_errors: Option<u64>,
    #[serde(rename = "PacketsOutboundErrors")]
    packets_outbound_errors: Option<u64>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_NetworkAdapter")]
struct Win32NetworkAdapter {
    #[serde(rename = "Index")]
    index: Option<i32>,
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "MACAddress")]
    mac_address: Option<String>,
    #[serde(rename = "Speed")]
    speed: Option<u64>,
    #[serde(rename = "NetEnabled")]
    net_enabled: Option<bool>,
    #[serde(rename = "PhysicalAdapter")]
    physical_adapter: Option<bool>,
    #[serde(rename = "AdapterType")]
    adapter_type: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_NetworkAdapterConfiguration")]
struct Win32NetworkAdapterConfiguration {
    #[serde(rename = "Index")]
    index: Option<i32>,
    #[serde(rename = "IPAddress")]
    ip_address: Option<Vec<String>>, // IPv4/IPv6 列表
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_LogicalDisk")]
struct Win32LogicalDisk {
    #[serde(rename = "DeviceID")]
    device_id: Option<String>,
    #[serde(rename = "Size")]
    size: Option<u64>,
    #[serde(rename = "FreeSpace")]
    free_space: Option<u64>,
    #[serde(rename = "DriveType")]
    drive_type: Option<u32>, // 3 表示本地磁盘
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "MSStorageDriver_FailurePredictStatus")]
struct MsStorageDriverFailurePredictStatus {
    #[serde(rename = "InstanceName")]
    instance_name: Option<String>,
    #[serde(rename = "PredictFailure")]
    predict_failure: Option<bool>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_DiskDrive")]
struct Win32DiskDrive {
    #[serde(rename = "Model")]
    model: Option<String>,
    #[serde(rename = "Status")]
    status: Option<String>,
}

fn wmi_read_cpu_temp_c(conn: &wmi::WMIConnection) -> Option<f32> {
    let res: Result<Vec<MSAcpiThermalZoneTemperature>, _> = conn.query();
    let mut vals: Vec<f32> = Vec::new();
    if let Ok(list) = res {
        for item in list.into_iter() {
            if let Some(kx10) = item.current_temperature {
                // Kelvin x10 -> Celsius
                if kx10 > 0 {
                    let c = (kx10 as f32 / 10.0) - 273.15;
                    // 过滤异常值
                    if c > -50.0 && c < 150.0 {
                        vals.push(c);
                    }
                }
            }
        }
    }
    if vals.is_empty() { None } else { Some(vals.iter().copied().sum::<f32>() / vals.len() as f32) }
}

fn wmi_read_fan_rpm(conn: &wmi::WMIConnection) -> Option<u32> {
    // Win32_Fan 通常不提供实时转速，这里尽力读取 DesiredSpeed 作为近似；若无则返回 None
    let res: Result<Vec<Win32Fan>, _> = conn.query();
    if let Ok(list) = res {
        let mut best: u64 = 0;
        for item in list.into_iter() {
            if let Some(v) = item.desired_speed {
                if v > best { best = v; }
            }
        }
        if best > 0 { return Some(best.min(u32::MAX as u64) as u32); }
    }
    None
}

// ---- WMI helpers: network interfaces, logical disks, SMART status ----
fn wmi_list_net_ifs(conn: &wmi::WMIConnection) -> Option<Vec<NetIfPayload>> {
    let cfgs: Result<Vec<Win32NetworkAdapterConfiguration>, _> = conn.query();
    let ads: Result<Vec<Win32NetworkAdapter>, _> = conn.query();
    if let (Ok(cfgs), Ok(ads)) = (cfgs, ads) {
        use std::collections::HashMap;
        let mut by_index: HashMap<i32, Vec<String>> = HashMap::new();
        for c in cfgs.into_iter() {
            if let Some(idx) = c.index {
                if let Some(ips) = c.ip_address { by_index.insert(idx, ips); }
            }
        }
        let mut out: Vec<NetIfPayload> = Vec::new();
        for a in ads.into_iter() {
            let enabled = a.net_enabled.unwrap_or(true);
            let physical = a.physical_adapter.unwrap_or(true);
            if !enabled || !physical { continue; }
            if a.mac_address.is_none() { continue; }
            let link_mbps = a.speed.map(|bps| (bps / 1_000_000) as u64);
            let ips = a.index.and_then(|idx| by_index.remove(&idx));
            out.push(NetIfPayload {
                name: a.name,
                mac: a.mac_address,
                ips: ips,
                link_mbps,
                media_type: a.adapter_type,
            });
        }
        if out.is_empty() { None } else { Some(out) }
    } else {
        None
    }
}

fn wmi_list_logical_disks(conn: &wmi::WMIConnection) -> Option<Vec<LogicalDiskPayload>> {
    let res: Result<Vec<Win32LogicalDisk>, _> = conn.query();
    if let Ok(list) = res {
        let mut out: Vec<LogicalDiskPayload> = Vec::new();
        for d in list.into_iter() {
            // 3 = 本地磁盘；过滤掉光驱、网络驱动器等
            if d.drive_type != Some(3) { continue; }
            out.push(LogicalDiskPayload {
                drive: d.device_id,
                size_bytes: d.size,
                free_bytes: d.free_space,
            });
        }
        if out.is_empty() { None } else { Some(out) }
    } else { None }
}

fn wmi_list_smart_status(conn: &wmi::WMIConnection) -> Option<Vec<SmartHealthPayload>> {
    let res: Result<Vec<MsStorageDriverFailurePredictStatus>, _> = conn.query();
    if let Ok(list) = res {
        let mut out: Vec<SmartHealthPayload> = Vec::new();
        for it in list.into_iter() {
            out.push(SmartHealthPayload {
                device: it.instance_name,
                predict_fail: it.predict_failure,
            });
        }
        if out.is_empty() { None } else { Some(out) }
    } else { None }
}

 fn wmi_fallback_disk_status(conn: &wmi::WMIConnection) -> Option<Vec<SmartHealthPayload>> {
     // 回退：使用 Win32_DiskDrive.Status（ROOT\\CIMV2）作为健康近似。
     // Status 常见值为 "OK"/"Error"/"Degraded"/"Unknown" 等。
     let res: Result<Vec<Win32DiskDrive>, _> = conn.query();
     if let Ok(list) = res {
         let mut out: Vec<SmartHealthPayload> = Vec::new();
         for d in list.into_iter() {
             // 将非 OK 视为预警；未知则 None
             let predict = d.status.as_deref().map(|s| s.to_ascii_uppercase() != "OK");
             out.push(SmartHealthPayload {
                 device: d.model,
                 predict_fail: predict,
             });
         }
         if out.is_empty() { None } else { Some(out) }
     } else { None }
 }

// ---- Realtime snapshot payload for frontend ----
#[derive(Clone, serde::Serialize)]
struct SensorSnapshot {
    cpu_usage: f32,
    mem_used_gb: f32,
    mem_total_gb: f32,
    mem_pct: f32,
    net_rx_bps: f64,
    net_tx_bps: f64,
    // 新增：Wi‑Fi 指标（若无连接则为 None）
    wifi_ssid: Option<String>,
    wifi_signal_pct: Option<i32>,
    wifi_link_mbps: Option<i32>,
    // Wi‑Fi 扩展
    wifi_bssid: Option<String>,
    wifi_channel: Option<i32>,
    wifi_radio: Option<String>,
    wifi_band: Option<String>,
    wifi_rx_mbps: Option<i32>,
    wifi_tx_mbps: Option<i32>,
    wifi_rssi_dbm: Option<i32>,
    wifi_rssi_estimated: Option<bool>,
    // 新增：网络接口（IP/MAC/速率/介质）
    net_ifs: Option<Vec<NetIfPayload>>,
    disk_r_bps: f64,
    disk_w_bps: f64,
    // 新增：温度（摄氏度）与风扇转速（RPM），可能不可用
    cpu_temp_c: Option<f32>,
    mobo_temp_c: Option<f32>,
    fan_rpm: Option<u32>,
    // 新增：存储温度（NVMe/SSD），与桥接字段 storageTemps 对应
    storage_temps: Option<Vec<StorageTempPayload>>,
    // 新增：逻辑磁盘容量（每盘总容量/可用空间）
    logical_disks: Option<Vec<LogicalDiskPayload>>,
    // 新增：SMART 健康（是否预测失败）
    smart_health: Option<Vec<SmartHealthPayload>>,
    // 新增：桥接健康指标
    hb_tick: Option<i64>,
    idle_sec: Option<i32>,
    exc_count: Option<i32>,
    uptime_sec: Option<i32>,
    // 第二梯队：CPU 扩展与桥接重建秒数
    cpu_pkg_power_w: Option<f64>,
    cpu_avg_freq_mhz: Option<f64>,
    cpu_throttle_active: Option<bool>,
    cpu_throttle_reasons: Option<Vec<String>>,
    since_reopen_sec: Option<i32>,
    // 每核心：负载/频率/温度（与桥接输出对应）。数组元素可为 null。
    cpu_core_loads_pct: Option<Vec<Option<f32>>>,
    cpu_core_clocks_mhz: Option<Vec<Option<f64>>>,
    cpu_core_temps_c: Option<Vec<Option<f32>>>,
    // 第二梯队：磁盘 IOPS/队列长度
    disk_r_iops: Option<f64>,
    disk_w_iops: Option<f64>,
    disk_queue_len: Option<f64>,
    // 第二梯队：网络错误率（每秒）与近似延迟（ms）
    net_rx_err_ps: Option<f64>,
    net_tx_err_ps: Option<f64>,
    ping_rtt_ms: Option<f64>,
    // 新增：GPU 列表
    gpus: Option<Vec<GpuPayload>>,
    timestamp_ms: i64,
}

#[derive(Clone, serde::Serialize)]
struct StorageTempPayload {
    name: Option<String>,
    temp_c: Option<f32>,
}

#[derive(Clone, serde::Serialize)]
struct GpuPayload {
    name: Option<String>,
    temp_c: Option<f32>,
    load_pct: Option<f32>,
    core_mhz: Option<f64>,
    fan_rpm: Option<i32>,
    vram_used_mb: Option<f64>,
    power_w: Option<f64>,
}

#[derive(Clone, serde::Serialize)]
struct NetIfPayload {
    name: Option<String>,
    mac: Option<String>,
    ips: Option<Vec<String>>,
    link_mbps: Option<u64>,
    media_type: Option<String>,
}

#[derive(Clone, serde::Serialize)]
struct LogicalDiskPayload {
    drive: Option<String>,
    size_bytes: Option<u64>,
    free_bytes: Option<u64>,
}

#[derive(Clone, serde::Serialize)]
struct SmartHealthPayload {
    device: Option<String>,
    predict_fail: Option<bool>,
}

// ---- Bridge (.NET LibreHardwareMonitor) JSON payload ----
#[derive(Clone, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BridgeFan {
    name: Option<String>,
    rpm: Option<i32>,
    pct: Option<i32>,
}

#[derive(Clone, serde::Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct BridgeOut {
    cpu_temp_c: Option<f32>,
    mobo_temp_c: Option<f32>,
    fans: Option<Vec<BridgeFan>>,
    storage_temps: Option<Vec<BridgeStorageTemp>>,
    gpus: Option<Vec<BridgeGpu>>,
    is_admin: Option<bool>,
    has_temp: Option<bool>,
    has_temp_value: Option<bool>,
    has_fan: Option<bool>,
    has_fan_value: Option<bool>,
    // 第二梯队：CPU 指标
    cpu_pkg_power_w: Option<f64>,
    cpu_avg_freq_mhz: Option<f64>,
    cpu_throttle_active: Option<bool>,
    cpu_throttle_reasons: Option<Vec<String>>,
    since_reopen_sec: Option<i32>,
    // 每核心：负载/频率/温度（桥接输出：cpuCoreLoadsPct/cpuCoreClocksMhz/cpuCoreTempsC）
    cpu_core_loads_pct: Option<Vec<Option<f32>>>,
    cpu_core_clocks_mhz: Option<Vec<Option<f64>>>,
    cpu_core_temps_c: Option<Vec<Option<f32>>>,
    // 健康指标
    hb_tick: Option<i64>,
    idle_sec: Option<i32>,
    exc_count: Option<i32>,
    uptime_sec: Option<i32>,
}

#[derive(Clone, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BridgeStorageTemp {
    name: Option<String>,
    temp_c: Option<f32>,
}

#[derive(Clone, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BridgeGpu {
    name: Option<String>,
    temp_c: Option<f32>,
    load_pct: Option<f32>,
    core_mhz: Option<f64>,
    fan_rpm: Option<i32>,
    vram_used_mb: Option<f64>,
    power_w: Option<f64>,
}

// ---- Minimal 5x7 bitmap font (digits and a few symbols) ----
const FONT_W: usize = 5;
const FONT_H: usize = 7;

fn glyph_rows(ch: char) -> [u8; FONT_H] {
    match ch {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '%' => [0b10001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b11111],
        'C' => [0b00110, 0b01001, 0b10000, 0b10000, 0b10000, 0b01001, 0b00110],
        'M' => [0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001],
        '-' => [0b00000, 0b00000, 0b00000, 0b01110, 0b00000, 0b00000, 0b00000],
        _ => [0; FONT_H],
    }
}

fn draw_text_rgba(buf: &mut [u8], w: usize, h: usize, x: usize, y: usize, scale: usize, gap: usize, text: &str) {
    // simple shadow
    draw_text_rgba_no_shadow(buf, w, h, x + 1, y + 1, scale, gap, text, [0, 0, 0, 180]);
    draw_text_rgba_no_shadow(buf, w, h, x, y, scale, gap, text, [255, 255, 255, 255]);
}

fn draw_text_rgba_no_shadow(buf: &mut [u8], w: usize, h: usize, x: usize, y: usize, scale: usize, gap: usize, text: &str, color: [u8;4]) {
    let mut pen_x = x;
    for ch in text.chars() {
        let rows = glyph_rows(ch);
        for (ry, row_bits) in rows.iter().enumerate() {
            for rx in 0..FONT_W {
                if (row_bits >> (FONT_W - 1 - rx)) & 1 == 1 {
                    // draw a scale x scale block
                    for oy in 0..scale {
                        for ox in 0..scale {
                            let px = pen_x + rx * scale + ox;
                            let py = y + ry * scale + oy;
                            if px < w && py < h {
                                let idx = (py * w + px) * 4;
                                buf[idx] = color[0];
                                buf[idx + 1] = color[1];
                                buf[idx + 2] = color[2];
                                buf[idx + 3] = color[3];
                            }
                        }
                    }
                }
            }
        }
        // width = FONT_W*scale + gap
        pen_x += FONT_W * scale + gap;
    }
}

fn make_tray_icon(top_text_in: &str, bottom_text_in: &str) -> tauri::image::Image<'static> {
    let w: usize = 32;
    let h: usize = 32;
    let mut rgba = vec![0u8; w * h * 4]; // transparent background

    // 准备两行文本（由调用方传入）：上行与下行
    let top_initial = top_text_in.to_string();
    let bottom_initial = bottom_text_in.to_string();

    // 计算文本宽度：chars*FONT_W*scale + (chars-1)*gap
    let calc_text_w = |chars: usize, scale: usize, gap: usize| chars * FONT_W * scale + chars.saturating_sub(1) * gap;
    // 优先使用大字号 scale=2，gap=0；若仍溢出，则降到 scale=1，gap=1
    // 顶部文本优先保持大字号，必要时去掉单位字符('C')再判断
    let mut top = top_initial.clone();
    let mut top_scale = 2usize; let mut top_gap = 0usize;
    if calc_text_w(top.chars().count(), top_scale, top_gap) > w {
        if top.ends_with('C') { top.pop(); }
        if calc_text_w(top.chars().count(), top_scale, top_gap) > w { top_scale = 1; top_gap = 1; }
    }
    // 底部文本优先保持大字号，必要时去掉单位字符('%')再判断
    let mut bottom = bottom_initial.clone();
    let mut bot_scale = 2usize; let mut bot_gap = 0usize;
    if calc_text_w(bottom.chars().count(), bot_scale, bot_gap) > w {
        if bottom.ends_with('%') { bottom.pop(); }
        if calc_text_w(bottom.chars().count(), bot_scale, bot_gap) > w { bot_scale = 1; bot_gap = 1; }
    }

    // 水平居中坐标
    let text_w_top = calc_text_w(top.chars().count(), top_scale, top_gap);
    let text_w_bot = calc_text_w(bottom.chars().count(), bot_scale, bot_gap);
    let x_top = (w.saturating_sub(text_w_top)) / 2;
    let x_bot = (w.saturating_sub(text_w_bot)) / 2;

    // 垂直布局：顶部留 3px，行间距 2px
    let y_top = 3usize;
    let y_bot = y_top + FONT_H * top_scale + 2;

    draw_text_rgba(&mut rgba, w, h, x_top, y_top, top_scale, top_gap, &top);
    draw_text_rgba(&mut rgba, w, h, x_bot, y_bot, bot_scale, bot_gap, &bottom);

    tauri::image::Image::new_owned(rgba, w as u32, h as u32)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::thread;
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        image::Image,
        Emitter,
        tray::TrayIconBuilder,
        WebviewWindowBuilder,
        WebviewUrl,
        Manager,
    };

    use tauri::path::BaseDirectory;

    // ---- App configuration (persisted as JSON) ----
    #[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
    struct AppConfig {
        // 托盘第二行显示模式："cpu" | "mem" | "fan"
        // 兼容旧字段 tray_show_mem：若为 true 则等价于 "mem"，否则为 "cpu"
        tray_bottom_mode: Option<String>,
        // 兼容保留（已弃用）：托盘第二行 true=显示内存%，false=显示CPU%
        tray_show_mem: bool,
        // 网络接口白名单：为空或缺省表示聚合全部
        net_interfaces: Option<Vec<String>>,
    }

    struct AppState(std::sync::Arc<std::sync::Mutex<AppConfig>>);

    fn resolve_config_path(app: &tauri::AppHandle) -> std::path::PathBuf {
        app.path()
            .resolve("config.json", BaseDirectory::AppConfig)
            .unwrap_or_else(|_| std::path::PathBuf::from("config.json"))
    }

    fn load_config(app: &tauri::AppHandle) -> AppConfig {
        let path = resolve_config_path(app);
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(cfg) = serde_json::from_slice::<AppConfig>(&bytes) {
                return cfg;
            }
        }
        AppConfig::default()
    }

    fn save_config(app: &tauri::AppHandle, cfg: &AppConfig) -> std::io::Result<()> {
        let path = resolve_config_path(app);
        if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
        let data = serde_json::to_vec_pretty(cfg).unwrap_or_else(|_| b"{}".to_vec());
        std::fs::write(path, data)
    }

    #[tauri::command]
    fn get_config(state: tauri::State<'_, AppState>) -> AppConfig {
        if let Ok(guard) = state.0.lock() { guard.clone() } else { AppConfig::default() }
    }

    #[tauri::command]
    fn set_config(app: tauri::AppHandle, state: tauri::State<'_, AppState>, new_cfg: AppConfig) -> Result<(), String> {
        save_config(&app, &new_cfg).map_err(|e| e.to_string())?;
        if let Ok(mut guard) = state.0.lock() { *guard = new_cfg; }
        let _ = app.emit("config://changed", "ok");
        Ok(())
    }

    #[tauri::command]
    fn list_net_interfaces() -> Vec<String> {
        use sysinfo::Networks;
        let nets = Networks::new_with_refreshed_list();
        nets.iter().map(|(name, _)| name.to_string()).collect()
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_config, set_config, list_net_interfaces])
        .setup(|app| {
            use tauri::WindowEvent;
            // Windows 下：启动时自动检测管理员权限，若非管理员则尝试以管理员身份重启并退出当前进程
            // 但在开发模式（debug 或存在 TAURI_DEV_SERVER_URL）下禁用自动提权，避免断开 tauri dev server 导致 localhost 拒绝连接。
            #[cfg(windows)]
            {
                let is_dev_mode = cfg!(debug_assertions) || std::env::var("TAURI_DEV_SERVER_URL").is_ok();
                if !is_dev_mode {
                    let is_admin = std::process::Command::new("powershell")
                        .args([
                            "-NoProfile",
                            "-Command",
                            "([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
                        ])
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|s| s.trim().eq_ignore_ascii_case("True"))
                        .unwrap_or(false);
                    if !is_admin {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new("powershell")
                                .args([
                                    "-NoProfile",
                                    "-Command",
                                    &format!("Start-Process -FilePath '{}' -Verb runas", exe.display()),
                                ])
                                .spawn();
                        }
                        eprintln!("[sys-sensor] 正在请求管理员权限运行，请在UAC中确认...");
                        std::process::exit(0);
                    }
                }
            }
            // 为已存在的主窗口（label: "main"）注册关闭->隐藏处理
            if let Some(main_win) = app.get_webview_window("main") {
                let main_win_c = main_win.clone();
                let _ = main_win.on_window_event(move |e| {
                    if let WindowEvent::CloseRequested { api, .. } = e {
                        let _ = main_win_c.hide();
                        api.prevent_close();
                    }
                });
            }
            use std::io::{BufRead, BufReader};
            use std::process::{Command, Stdio};
            use std::sync::{Arc, Mutex};
            use std::time::Instant as StdInstant;
            // --- Build non-clickable info area as disabled menu items ---
            let info_cpu = MenuItem::with_id(app, "info_cpu", "CPU: —", false, None::<&str>)?;
            let info_mem = MenuItem::with_id(app, "info_mem", "内存: —", false, None::<&str>)?;
            let info_temp = MenuItem::with_id(app, "info_temp", "温度: —", false, None::<&str>)?;
            let info_fan = MenuItem::with_id(app, "info_fan", "风扇: —", false, None::<&str>)?;
            let info_net = MenuItem::with_id(app, "info_net", "网络: —", false, None::<&str>)?;
            let info_disk = MenuItem::with_id(app, "info_disk", "磁盘: —", false, None::<&str>)?;
            let info_store = MenuItem::with_id(app, "info_store", "存储: —", false, None::<&str>)?;
            let info_bridge = MenuItem::with_id(app, "info_bridge", "桥接: —", false, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;

            // --- Clickable action items ---
            let show_details = MenuItem::with_id(app, "show_details", "显示详情", true, None::<&str>)?;
            let quick_settings = MenuItem::with_id(app, "quick_settings", "快速设置", true, None::<&str>)?;
            let about = MenuItem::with_id(app, "about", "关于我们", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?;

            // 初始化配置并注入状态
            let cfg_arc: Arc<Mutex<AppConfig>> = Arc::new(Mutex::new(load_config(&app.handle())));
            app.manage(AppState(cfg_arc.clone()));

            let menu = Menu::with_items(
                app,
                &[
                    &info_cpu,
                    &info_mem,
                    &info_temp,
                    &info_fan,
                    &info_net,
                    &info_disk,
                    &info_store,
                    &info_bridge,
                    &sep,
                    &show_details,
                    &quick_settings,
                    &about,
                    &exit,
                ],
            )?;

            // --- Create tray icon ---
            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("系统监控 - 初始化中...");

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let tray = tray_builder.build(app)?;
            let app_handle = app.handle();
            // 预计算打包资源中的桥接可执行文件路径（如存在，优先使用）
            let packaged_bridge_exe = app_handle
                .path()
                .resolve("sensor-bridge/sensor-bridge.exe", BaseDirectory::Resource)
                .ok();

            // 退出控制与子进程 PID 记录（用于退出时清理）
            let shutdown_flag: Arc<std::sync::atomic::AtomicBool> = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let bridge_pid: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));

            // --- Spawn sensor-bridge (.NET) and share latest output ---
            let bridge_data: Arc<Mutex<(Option<BridgeOut>, StdInstant)>> = Arc::new(Mutex::new((None, StdInstant::now())));
            {
                let bridge_data_c = bridge_data.clone();
                let packaged_bridge_exe_c = packaged_bridge_exe.clone();
                let shutdown_c = shutdown_flag.clone();
                let bridge_pid_c = bridge_pid.clone();
                std::thread::spawn(move || {
                    // Resolve project root by walking up until we find `sensor-bridge/sensor-bridge.csproj`
                    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
                    let mut cursor = exe_dir.clone();
                    let mut found_root: Option<std::path::PathBuf> = None;
                    for _ in 0..6 {
                        if let Some(dir) = cursor {
                            let probe = dir.join("sensor-bridge").join("sensor-bridge.csproj");
                            if probe.exists() {
                                found_root = Some(dir.clone());
                                break;
                            }
                            cursor = dir.parent().map(|p| p.to_path_buf());
                        } else {
                            break;
                        }
                    }
                    let project_root = found_root
                        .or_else(|| exe_dir.and_then(|p| p.parent().map(|p| p.to_path_buf())))
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
                    eprintln!("[bridge] Using project_root: {}", project_root.display());

                    // 便携版额外兜底：exe 同目录/resources/sensor-bridge/sensor-bridge.exe
                    let portable_bridge_exe: Option<std::path::PathBuf> = std::env::current_exe()
                        .ok()
                        .and_then(|p| p.parent().map(|d| d.join("resources").join("sensor-bridge").join("sensor-bridge.exe")));

                    loop {
                        if shutdown_c.load(std::sync::atomic::Ordering::SeqCst) { break; }
                        // 0) 若存在打包资源中的自包含 EXE，优先直接启动
                        if let Some(ref p) = packaged_bridge_exe_c {
                            if p.exists() {
                                eprintln!("[bridge] spawning packaged exe: {}", p.display());
                                let mut cmd = std::process::Command::new(p);
                                cmd.current_dir(p.parent().unwrap_or(&project_root));
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                                }
                                let mut spawned = cmd
                                    .stdout(Stdio::piped())
                                    .stderr(Stdio::piped())
                                    .spawn()
                                    .ok();
                                if let Some(ref mut child_proc) = spawned {
                                    if let Ok(mut g) = bridge_pid_c.lock() { *g = Some(child_proc.id()); }
                                    if let Some(stdout) = child_proc.stdout.take() {
                                        let reader = BufReader::new(stdout);
                                        for line in reader.lines().flatten() {
                                            if line.trim().is_empty() { continue; }
                                            if let Ok(parsed) = serde_json::from_str::<BridgeOut>(&line) {
                                                if let Ok(mut guard) = bridge_data_c.lock() {
                                                    *guard = (Some(parsed), StdInstant::now());
                                                }
                                            } else {
                                                eprintln!("[bridge] Non-JSON line: {}", line);
                                            }
                                        }
                                    }
                                    if let Some(stderr) = child_proc.stderr.take() {
                                        std::thread::spawn(move || {
                                            use std::io::{BufRead, BufReader};
                                            let rdr = BufReader::new(stderr);
                                            for line in rdr.lines().flatten() {
                                                if line.trim().is_empty() { continue; }
                                                eprintln!("[bridge][stderr] {}", line);
                                            }
                                        });
                                    }
                                    let _ = child_proc.wait();
                                    if let Ok(mut g) = bridge_pid_c.lock() { *g = None; }
                                    eprintln!("[bridge] packaged bridge exited, respawn in 3s...");
                                    std::thread::sleep(std::time::Duration::from_secs(3));
                                    continue;
                                } else {
                                    eprintln!("[bridge] Failed to spawn packaged sensor-bridge.exe, fallback to dev paths in 3s...");
                                    std::thread::sleep(std::time::Duration::from_secs(3));
                                    // 继续进入后续 dev 启动分支
                                }
                            }
                        }
                        // 0b) 便携版兜底：尝试 exe 同目录下的 resources 路径
                        if let Some(ref p) = portable_bridge_exe {
                            if p.exists() {
                                eprintln!("[bridge] spawning portable packaged exe: {}", p.display());
                                let mut cmd = std::process::Command::new(p);
                                cmd.current_dir(p.parent().unwrap_or(&project_root));
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                                }
                                let mut spawned = cmd
                                    .stdout(Stdio::piped())
                                    .stderr(Stdio::piped())
                                    .spawn()
                                    .ok();
                                if let Some(ref mut child_proc) = spawned {
                                    if let Ok(mut g) = bridge_pid_c.lock() { *g = Some(child_proc.id()); }
                                    if let Some(stdout) = child_proc.stdout.take() {
                                        let reader = BufReader::new(stdout);
                                        for line in reader.lines().flatten() {
                                            if line.trim().is_empty() { continue; }
                                            if let Ok(parsed) = serde_json::from_str::<BridgeOut>(&line) {
                                                if let Ok(mut guard) = bridge_data_c.lock() {
                                                    *guard = (Some(parsed), StdInstant::now());
                                                }
                                            } else {
                                                eprintln!("[bridge] Non-JSON line: {}", line);
                                            }
                                        }
                                    }
                                    if let Some(stderr) = child_proc.stderr.take() {
                                        std::thread::spawn(move || {
                                            use std::io::{BufRead, BufReader};
                                            let rdr = BufReader::new(stderr);
                                            for line in rdr.lines().flatten() {
                                                if line.trim().is_empty() { continue; }
                                                eprintln!("[bridge][stderr] {}", line);
                                            }
                                        });
                                    }
                                    let _ = child_proc.wait();
                                    if let Ok(mut g) = bridge_pid_c.lock() { *g = None; }
                                    eprintln!("[bridge] portable packaged bridge exited, respawn in 3s...");
                                    std::thread::sleep(std::time::Duration::from_secs(3));
                                    continue;
                                } else {
                                    eprintln!("[bridge] Failed to spawn portable sensor-bridge.exe, fallback to dev paths in 3s...");
                                    std::thread::sleep(std::time::Duration::from_secs(3));
                                }
                            }
                        }
                        let dll_candidates = [
                            project_root.join("sensor-bridge/bin/Release/net8.0/sensor-bridge.dll"),
                            project_root.join("sensor-bridge/bin/Debug/net8.0/sensor-bridge.dll"),
                        ];
                        let exe_candidates = [
                            project_root.join("sensor-bridge/bin/Release/net8.0/sensor-bridge.exe"),
                            project_root.join("sensor-bridge/bin/Debug/net8.0/sensor-bridge.exe"),
                        ];

                        // 1) 优先使用 dll: dotnet <dll>
                        let mut child = if let Some(dll) = dll_candidates.iter().find(|p| p.exists()) {
                            eprintln!("[bridge] spawning via dotnet: {}", dll.display());
                            {
                                let mut cmd = Command::new("dotnet");
                                cmd.arg(dll)
                                    .current_dir(project_root.clone());
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                                }
                                cmd.stdout(Stdio::piped())
                                    .stderr(Stdio::piped())
                                    .spawn()
                                    .ok()
                            }
                        // 2) 其次尝试 exe 直接启动
                        } else if let Some(exe) = exe_candidates.iter().find(|p| p.exists()) {
                            eprintln!("[bridge] spawning exe: {}", exe.display());
                            {
                                let mut cmd = Command::new(exe);
                                cmd.current_dir(project_root.clone());
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                                }
                                cmd.stdout(Stdio::piped())
                                    .stderr(Stdio::piped())
                                    .spawn()
                                    .ok()
                            }
                        } else {
                            // 3) 最后 fallback 到 dotnet run
                            eprintln!("[bridge] fallback to 'dotnet run --project sensor-bridge'");
                            {
                                let mut cmd = Command::new("dotnet");
                                cmd.args(["run", "--project", "sensor-bridge"]) 
                                    .current_dir(project_root.clone());
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                                }
                                cmd.stdout(Stdio::piped())
                                    .stderr(Stdio::piped())
                                    .spawn()
                                    .ok()
                            }
                        };

                        if let Some(ref mut child_proc) = child {
                            if let Ok(mut g) = bridge_pid_c.lock() { *g = Some(child_proc.id()); }
                            if let Some(stdout) = child_proc.stdout.take() {
                                let reader = BufReader::new(stdout);
                                for line in reader.lines().flatten() {
                                    if line.trim().is_empty() { continue; }
                                    if let Ok(parsed) = serde_json::from_str::<BridgeOut>(&line) {
                                        if let Ok(mut guard) = bridge_data_c.lock() {
                                            *guard = (Some(parsed), StdInstant::now());
                                        }
                                    } else {
                                        eprintln!("[bridge] Non-JSON line: {}", line);
                                    }
                                }
                            }
                            // Drain and print stderr if available for diagnostics
                            if let Some(stderr) = child_proc.stderr.take() {
                                std::thread::spawn(move || {
                                    use std::io::{BufRead, BufReader};
                                    let rdr = BufReader::new(stderr);
                                    for line in rdr.lines().flatten() {
                                        if line.trim().is_empty() { continue; }
                                        eprintln!("[bridge][stderr] {}", line);
                                    }
                                });
                            }
                            // Wait child and then respawn
                            let _ = child_proc.wait();
                            if let Ok(mut g) = bridge_pid_c.lock() { *g = None; }
                            eprintln!("[bridge] bridge process exited, will respawn in 3s...");
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            continue;
                        } else {
                            eprintln!("[bridge] Failed to spawn sensor-bridge process, retry in 3s.");
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            continue;
                        }
                    }
                });
            }

            // --- Handle menu events ---
            let shutdown_for_exit = shutdown_flag.clone();
            let bridge_pid_for_exit = bridge_pid.clone();
            tray.on_menu_event(move |app, event| match event.id.as_ref() {
                "show_details" => {
                    println!("[tray] 点击 显示详情");
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                        let _ = win.eval("if (location.hash !== '#/details') { location.hash = '#/details'; }");
                    } else {
                        // 兜底：若没有主窗口（理论上不会发生），创建一个并直接进入 details
                        let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html#/details".into()))
                            .title("sys-sensor · 详情")
                            .inner_size(900.0, 600.0)
                            .min_inner_size(600.0, 400.0)
                            .resizable(true)
                            .build();
                    }
                }
                "quick_settings" => {
                    println!("[tray] 点击 快速设置");
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                        let _ = win.eval("if (location.hash !== '#/settings') { location.hash = '#/settings'; }");
                    } else {
                        let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html#/settings".into()))
                            .title("sys-sensor · 快速设置")
                            .inner_size(640.0, 520.0)
                            .min_inner_size(480.0, 360.0)
                            .resizable(true)
                            .build();
                    }
                }
                "about" => {
                    println!("[tray] 点击 关于我们");
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                        let _ = win.eval("if (location.hash !== '#/about') { location.hash = '#/about'; }");
                    } else {
                        let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html#/about".into()))
                            .title("关于 sys-sensor")
                            .inner_size(420.0, 360.0)
                            .min_inner_size(380.0, 320.0)
                            .resizable(false)
                            .build();
                    }
                }
                "exit" => {
                    println!("[tray] 退出");
                    // 标记关闭，尝试结束桥接进程
                    shutdown_for_exit.store(true, std::sync::atomic::Ordering::SeqCst);
                    if let Ok(pid_opt) = bridge_pid_for_exit.lock() {
                        if let Some(pid) = *pid_opt {
                            #[cfg(windows)]
                            {
                                let _ = std::process::Command::new("taskkill")
                                    .args(["/PID", &pid.to_string(), "/T", "/F"]) 
                                    .status();
                            }
                        }
                    }
                    app.exit(0);
                }
                other => {
                    println!("[tray] 未处理的菜单项: {}", other);
                }
            });

            // --- Spawn background refresh thread (1s) ---
            let info_cpu_c = info_cpu.clone();
            let info_mem_c = info_mem.clone();
            let info_temp_c = info_temp.clone();
            let info_fan_c = info_fan.clone();
            let info_net_c = info_net.clone();
            let info_disk_c = info_disk.clone();
            let info_store_c = info_store.clone();
            let info_bridge_c = info_bridge.clone();
            let tray_c = tray.clone();
            let app_handle_c = app_handle.clone();
            let bridge_data_sampling = bridge_data.clone();
            let cfg_state_c = cfg_arc.clone();

            thread::spawn(move || {
                use std::time::{Duration, Instant};
                use sysinfo::{Networks, System};

                // 初始化 WMI 连接（在后台线程中初始化 COM）
                let mut wmi_temp_conn: Option<wmi::WMIConnection> = {
                    if let Ok(com) = wmi::COMLibrary::new() {
                        wmi::WMIConnection::with_namespace_path("ROOT\\WMI", com.into()).ok()
                    } else { None }
                };
                let mut wmi_fan_conn: Option<wmi::WMIConnection> = {
                    if let Ok(com) = wmi::COMLibrary::new() {
                        wmi::WMIConnection::new(com).ok() // 默认 ROOT\CIMV2
                    } else { None }
                };
                let mut wmi_perf_conn: Option<wmi::WMIConnection> = {
                    if let Ok(com) = wmi::COMLibrary::new() {
                        wmi::WMIConnection::new(com).ok() // ROOT\CIMV2: PerfFormattedData
                    } else { None }
                };

                // --- sysinfo contexts ---
                let mut sys = System::new_all();
                let mut networks = Networks::new_with_refreshed_list();

                // 初次刷新以建立基线
                sys.refresh_cpu_usage();
                sys.refresh_memory();

                // 累计计数与 EMA
                let mut last_net_rx: u64 = 0;
                let mut last_net_tx: u64 = 0;
                let mut last_disk_r: u64 = 0;
                let mut last_disk_w: u64 = 0;
                let mut last_t = Instant::now();
                let alpha = 0.3f64;
                let mut ema_net_rx: f64 = 0.0;
                let mut ema_net_tx: f64 = 0.0;
                let mut ema_disk_r: f64 = 0.0;
                let mut ema_disk_w: f64 = 0.0;
                let mut has_prev = false;
                let mut last_bridge_fresh: Option<bool> = None;
                // WMI 健壮性：失败计数与周期重开
                let mut wmi_fail_perf: u32 = 0;
                let mut last_wmi_reopen = Instant::now();

                // 单位格式化（bytes/s -> KB/s 或 MB/s）
                let fmt_bps = |bps: f64| -> String {
                    let kbps = bps / 1024.0;
                    if kbps < 1024.0 {
                        format!("{:.1} KB/s", kbps)
                    } else {
                        format!("{:.1} MB/s", kbps / 1024.0)
                    }
                };

                loop {
                    // 刷新数据
                    sys.refresh_cpu_usage();
                    sys.refresh_memory();
                    let _ = networks.refresh();
                    sys.refresh_processes();

                    // CPU 使用率（0~100）
                    let cpu_usage = sys.global_cpu_info().cpu_usage();
                    // 内存（以字节为单位读取后格式化为 GB）
                    let used = sys.used_memory() as f64;
                    let total = sys.total_memory() as f64;
                    let mem_pct = if total > 0.0 { (used / total) * 100.0 } else { 0.0 };
                    let used_gb = used / 1073741824.0; // 1024^3
                    let total_gb = total / 1073741824.0;

                    // --- 网络累计字节合计（可按配置过滤接口）---
                    let (net_rx_total, net_tx_total): (u64, u64) = {
                        let selected: Option<Vec<String>> = cfg_state_c
                            .lock().ok()
                            .and_then(|c| c.net_interfaces.clone())
                            .filter(|v| !v.is_empty());
                        if let Some(allow) = selected {
                            let mut rx = 0u64; let mut tx = 0u64;
                            for (name, data) in &networks {
                                if allow.iter().any(|n| n == name) {
                                    rx = rx.saturating_add(data.total_received());
                                    tx = tx.saturating_add(data.total_transmitted());
                                }
                            }
                            (rx, tx)
                        } else {
                            let mut rx = 0u64; let mut tx = 0u64;
                            for (_, data) in &networks {
                                rx = rx.saturating_add(data.total_received());
                                tx = tx.saturating_add(data.total_transmitted());
                            }
                            (rx, tx)
                        }
                    };

                    // --- 磁盘累计字节合计（按进程聚合）---
                    let mut disk_r_total: u64 = 0;
                    let mut disk_w_total: u64 = 0;
                    for (_pid, proc_) in sys.processes() {
                        let du = proc_.disk_usage();
                        disk_r_total = disk_r_total.saturating_add(du.total_read_bytes);
                        disk_w_total = disk_w_total.saturating_add(du.total_written_bytes);
                    }

                    // 计算速率（bytes/s）
                    let now = Instant::now();
                    let dt = now.duration_since(last_t).as_secs_f64().max(1e-6);
                    // 若系统经历了睡眠/长间隔（>5s），重置速率基线并尝试重建 WMI 连接
                    let slept = dt > 5.0;
                    if slept {
                        // 重置 EMA 基线：跳过本次差分，下一轮重新建立基线
                        has_prev = false;
                        // 重建 WMI 连接（分别初始化，避免单次失败影响全部）
                        if let Ok(com) = wmi::COMLibrary::new() {
                            wmi_temp_conn = wmi::WMIConnection::with_namespace_path("ROOT\\WMI", com.into()).ok();
                        }
                        if let Ok(com) = wmi::COMLibrary::new() {
                            wmi_fan_conn = wmi::WMIConnection::new(com).ok();
                        }
                        if let Ok(com) = wmi::COMLibrary::new() {
                            wmi_perf_conn = wmi::WMIConnection::new(com).ok();
                        }
                        last_wmi_reopen = Instant::now();
                        eprintln!("[wmi][reopen] due to long gap {:.1}s (sleep/resume?)", dt);
                    }
                    let mut net_rx_rate = 0.0;
                    let mut net_tx_rate = 0.0;
                    let mut disk_r_rate = 0.0;
                    let mut disk_w_rate = 0.0;
                    if has_prev {
                        net_rx_rate = (net_rx_total.saturating_sub(last_net_rx)) as f64 / dt;
                        net_tx_rate = (net_tx_total.saturating_sub(last_net_tx)) as f64 / dt;
                        disk_r_rate = (disk_r_total.saturating_sub(last_disk_r)) as f64 / dt;
                        disk_w_rate = (disk_w_total.saturating_sub(last_disk_w)) as f64 / dt;
                    }

                    // EMA 平滑
                    if !has_prev {
                        ema_net_rx = net_rx_rate;
                        ema_net_tx = net_tx_rate;
                        ema_disk_r = disk_r_rate;
                        ema_disk_w = disk_w_rate;
                        has_prev = true;
                    } else {
                        ema_net_rx = alpha * net_rx_rate + (1.0 - alpha) * ema_net_rx;
                        ema_net_tx = alpha * net_tx_rate + (1.0 - alpha) * ema_net_tx;
                        ema_disk_r = alpha * disk_r_rate + (1.0 - alpha) * ema_disk_r;
                        ema_disk_w = alpha * disk_w_rate + (1.0 - alpha) * ema_disk_w;
                    }

                    // 保存本次累计与时间
                    last_net_rx = net_rx_total;
                    last_net_tx = net_tx_total;
                    last_disk_r = disk_r_total;
                    last_disk_w = disk_w_total;
                    last_t = now;

                    // 读取第二梯队：磁盘 IOPS/队列、网络错误、RTT
                    let (disk_r_iops, disk_w_iops, disk_queue_len) = match &wmi_perf_conn {
                        Some(c) => wmi_perf_disk(c),
                        None => (None, None, None),
                    };
                    let (net_rx_err_ps, net_tx_err_ps) = match &wmi_perf_conn {
                        Some(c) => wmi_perf_net_err(c),
                        None => (None, None),
                    };
                    let ping_rtt_ms = tcp_rtt_ms("1.1.1.1:443", 300);
                    // 根据查询结果更新失败计数并在需要时重建 WMI Perf 连接
                    if wmi_perf_conn.is_some()
                        && disk_r_iops.is_none()
                        && disk_w_iops.is_none()
                        && disk_queue_len.is_none()
                        && net_rx_err_ps.is_none()
                        && net_tx_err_ps.is_none() {
                        wmi_fail_perf = wmi_fail_perf.saturating_add(1);
                    } else {
                        wmi_fail_perf = 0;
                    }
                    if wmi_fail_perf >= 3 || last_wmi_reopen.elapsed().as_secs() >= 1800 {
                        if let Ok(com) = wmi::COMLibrary::new() {
                            wmi_perf_conn = wmi::WMIConnection::new(com).ok();
                            eprintln!(
                                "[wmi][reopen] perf conn recreated (fail_cnt={}, periodic={})",
                                wmi_fail_perf,
                                (last_wmi_reopen.elapsed().as_secs() >= 1800)
                            );
                            wmi_fail_perf = 0;
                            last_wmi_reopen = Instant::now();
                        }
                    }

                    // 组织显示文本
                    let cpu_line = format!("CPU: {:.0}%", cpu_usage);
                    let mem_line = format!("内存: {:.1}/{:.1} GB ({:.0}%)", used_gb, total_gb, mem_pct);
                    // 读取温度与风扇（优先桥接数据，其次 WMI）
                    let (
                        bridge_cpu_temp,
                        bridge_mobo_temp,
                        bridge_cpu_fan,
                        case_fan,
                        bridge_cpu_fan_pct,
                        case_fan_pct,
                        is_admin,
                        has_temp,
                        has_temp_value,
                        has_fan,
                        has_fan_value,
                        storage_temps,
                        gpus,
                        hb_tick,
                        idle_sec,
                        exc_count,
                        uptime_sec,
                        cpu_pkg_power_w,
                        cpu_avg_freq_mhz,
                        cpu_throttle_active,
                        cpu_throttle_reasons,
                        since_reopen_sec,
                        cpu_core_loads_pct,
                        cpu_core_clocks_mhz,
                        cpu_core_temps_c,
                    ) = {
                        let mut cpu_t: Option<f32> = None;
                        let mut mobo_t: Option<f32> = None;
                        let mut cpu_fan: Option<u32> = None;
                        let mut case_fan: Option<u32> = None;
                        let mut cpu_fan_pct: Option<u32> = None;
                        let mut case_fan_pct: Option<u32> = None;
                        let mut is_admin: Option<bool> = None;
                        let mut has_temp: Option<bool> = None;
                        let mut has_temp_value: Option<bool> = None;
                        let mut has_fan: Option<bool> = None;
                        let mut has_fan_value: Option<bool> = None;
                        let mut storage_temps: Option<Vec<StorageTempPayload>> = None;
                        let mut gpus: Option<Vec<GpuPayload>> = None;
                        let mut hb_tick: Option<i64> = None;
                        let mut idle_sec: Option<i32> = None;
                        let mut exc_count: Option<i32> = None;
                        let mut uptime_sec: Option<i32> = None;
                        let mut cpu_pkg_power_w: Option<f64> = None;
                        let mut cpu_avg_freq_mhz: Option<f64> = None;
                        let mut cpu_throttle_active: Option<bool> = None;
                        let mut cpu_throttle_reasons: Option<Vec<String>> = None;
                        let mut since_reopen_sec: Option<i32> = None;
                        let mut cpu_core_loads_pct: Option<Vec<Option<f32>>> = None;
                        let mut cpu_core_clocks_mhz: Option<Vec<Option<f64>>> = None;
                        let mut cpu_core_temps_c: Option<Vec<Option<f32>>> = None;
                        let mut fresh_now: Option<bool> = None;
                        if let Ok(guard) = bridge_data_sampling.lock() {
                            if let (Some(ref b), ts) = (&guard.0, guard.1) {
                                // 若超过 30s 未更新则视为过期（原为 5s）。
                                // 现场发现：桥接在长时间运行、系统休眠/杀软打扰、或桥接短暂重启期间，输出间隔可能>5s，
                                // 过低阈值会导致误判为过期，从而丢弃桥接温度/风扇数据（WMI 又常无值），UI 显示“—”。
                                if ts.elapsed().as_secs() <= 30 {
                                    fresh_now = Some(true);
                                    cpu_t = b.cpu_temp_c;
                                    mobo_t = b.mobo_temp_c;
                                    is_admin = b.is_admin;
                                    has_temp = b.has_temp;
                                    has_temp_value = b.has_temp_value;
                                    has_fan = b.has_fan;
                                    has_fan_value = b.has_fan_value;
                                    // 存储温度
                                    if let Some(st) = &b.storage_temps {
                                        let mapped: Vec<StorageTempPayload> = st.iter().map(|x| StorageTempPayload {
                                            name: x.name.clone(),
                                            temp_c: x.temp_c,
                                        }).collect();
                                        if !mapped.is_empty() { storage_temps = Some(mapped); }
                                    }
                                    // GPU 列表
                                    if let Some(gg) = &b.gpus {
                                        let mapped: Vec<GpuPayload> = gg.iter().map(|x| GpuPayload {
                                            name: x.name.clone(),
                                            temp_c: x.temp_c,
                                            load_pct: x.load_pct,
                                            core_mhz: x.core_mhz,
                                            fan_rpm: x.fan_rpm,
                                            vram_used_mb: x.vram_used_mb,
                                            power_w: x.power_w,
                                        }).collect();
                                        if !mapped.is_empty() { gpus = Some(mapped); }
                                    }
                                    // 健康指标
                                    hb_tick = b.hb_tick;
                                    idle_sec = b.idle_sec;
                                    exc_count = b.exc_count;
                                    uptime_sec = b.uptime_sec;
                                    // 第二梯队：CPU 扩展与重建秒数
                                    cpu_pkg_power_w = b.cpu_pkg_power_w;
                                    cpu_avg_freq_mhz = b.cpu_avg_freq_mhz;
                                    cpu_throttle_active = b.cpu_throttle_active;
                                    cpu_throttle_reasons = b.cpu_throttle_reasons.clone();
                                    since_reopen_sec = b.since_reopen_sec;
                                    // 每核心数组
                                    cpu_core_loads_pct = b.cpu_core_loads_pct.clone();
                                    cpu_core_clocks_mhz = b.cpu_core_clocks_mhz.clone();
                                    cpu_core_temps_c = b.cpu_core_temps_c.clone();
                                    if let Some(fans) = &b.fans {
                                        let mut best_cpu: Option<i32> = None;
                                        let mut best_case: Option<i32> = None;
                                        let mut best_cpu_pct: Option<i32> = None;
                                        let mut best_case_pct: Option<i32> = None;
                                        for f in fans {
                                            if let Some(rpm) = f.rpm {
                                                let name_lc = f.name.as_deref().unwrap_or("").to_ascii_lowercase();
                                                if name_lc.contains("cpu") {
                                                    best_cpu = Some(best_cpu.map_or(rpm, |v| v.max(rpm)));
                                                } else {
                                                    best_case = Some(best_case.map_or(rpm, |v| v.max(rpm)));
                                                }
                                            }
                                            if let Some(p) = f.pct {
                                                let name_lc = f.name.as_deref().unwrap_or("").to_ascii_lowercase();
                                                if name_lc.contains("cpu") {
                                                    best_cpu_pct = Some(best_cpu_pct.map_or(p, |v| v.max(p)));
                                                } else {
                                                    best_case_pct = Some(best_case_pct.map_or(p, |v| v.max(p)));
                                                }
                                            }
                                        }
                                        cpu_fan = best_cpu.map(|v| v.max(0) as u32);
                                        case_fan = best_case.map(|v| v.max(0) as u32);
                                        cpu_fan_pct = best_cpu_pct.map(|v| v.clamp(0, 100) as u32);
                                        case_fan_pct = best_case_pct.map(|v| v.clamp(0, 100) as u32);
                                    }
                                } else {
                                    fresh_now = Some(false);
                                }
                            }
                        }
                        if let Some(f) = fresh_now {
                            if last_bridge_fresh.map(|x| x != f).unwrap_or(true) {
                                if f { eprintln!("[bridge][status] data became FRESH"); } else { eprintln!("[bridge][status] data became STALE"); }
                            }
                            last_bridge_fresh = Some(f);
                        }
                        (
                            cpu_t,
                            mobo_t,
                            cpu_fan,
                            case_fan,
                            cpu_fan_pct,
                            case_fan_pct,
                            is_admin,
                            has_temp,
                            has_temp_value,
                            has_fan,
                            has_fan_value,
                            storage_temps,
                            gpus,
                            hb_tick,
                            idle_sec,
                            exc_count,
                            uptime_sec,
                            cpu_pkg_power_w,
                            cpu_avg_freq_mhz,
                            cpu_throttle_active,
                            cpu_throttle_reasons,
                            since_reopen_sec,
                            cpu_core_loads_pct,
                            cpu_core_clocks_mhz,
                            cpu_core_temps_c,
                        )
                    };

                    let temp_opt = bridge_cpu_temp.or_else(|| wmi_temp_conn.as_ref().and_then(|c| wmi_read_cpu_temp_c(c)));
                    let fan_opt = bridge_cpu_fan.or_else(|| wmi_fan_conn.as_ref().and_then(|c| wmi_read_fan_rpm(c)));

                    let temp_line = if let Some(t) = temp_opt {
                        match bridge_mobo_temp {
                            Some(mb) => format!("温度: {:.1}°C  主板: {:.1}°C", t, mb),
                            None => format!("温度: {:.1}°C", t),
                        }
                    } else if let Some(mb) = bridge_mobo_temp {
                        format!("温度: —  主板: {:.1}°C", mb)
                    } else {
                        let mut s = "温度: —".to_string();
                        if has_temp == Some(true) && has_temp_value == Some(false) {
                            if is_admin == Some(false) { s.push_str(" (需管理员)"); }
                            else { s.push_str(" (无读数)"); }
                        }
                        s
                    };

                    // 风扇行：优先 RPM，否则占空比
                    let fan_line = {
                        if fan_opt.is_some() || case_fan.is_some() {
                            match (fan_opt, case_fan) {
                                (Some(c), Some(k)) => format!("风扇: CPU {} RPM / {} RPM", c, k),
                                (Some(c), None) => format!("风扇: CPU {} RPM", c),
                                (None, Some(k)) => format!("风扇: {} RPM", k),
                                _ => unreachable!(),
                            }
                        } else if bridge_cpu_fan_pct.is_some() || case_fan_pct.is_some() {
                            match (bridge_cpu_fan_pct, case_fan_pct) {
                                (Some(c), Some(k)) => format!("风扇: CPU {}% / {}%", c, k),
                                (Some(c), None) => format!("风扇: CPU {}%", c),
                                (None, Some(k)) => format!("风扇: {}%", k),
                                _ => unreachable!(),
                            }
                        } else {
                            let mut s = "风扇: —".to_string();
                            if has_fan == Some(true) && has_fan_value == Some(false) {
                                if is_admin == Some(false) { s.push_str(" (需管理员)"); }
                                else { s.push_str(" (无读数)"); }
                            }
                            s
                        }
                    };

                    // 网络/磁盘行
                    let net_line = format!(
                        "网络: 下行 {} 上行 {}",
                        fmt_bps(ema_net_rx),
                        fmt_bps(ema_net_tx)
                    );
                    let disk_line = format!(
                        "磁盘: 读 {} 写 {}",
                        fmt_bps(ema_disk_r),
                        fmt_bps(ema_disk_w)
                    );

                    // 存储温度行（最多显示 3 个，余量以 +N 表示）
                    let storage_line: String = match &storage_temps {
                        Some(sts) if !sts.is_empty() => {
                            let mut parts: Vec<String> = Vec::new();
                            for (i, st) in sts.iter().enumerate().take(3) {
                                let label = st.name.clone().unwrap_or_else(|| format!("驱动{}", i + 1));
                                let val = st.temp_c.map(|t| format!("{:.1}°C", t)).unwrap_or_else(|| "—".to_string());
                                parts.push(format!("{} {}", label, val));
                            }
                            let mut s = format!("存储: {}", parts.join(", "));
                            if sts.len() > 3 { s.push_str(&format!(" +{}", sts.len() - 3)); }
                            s
                        }
                        _ => "存储: —".to_string(),
                    };

                    // 桥接健康行
                    let bridge_line: String = {
                        let mut parts: Vec<String> = Vec::new();
                        if let Some(t) = hb_tick { parts.push(format!("hb {}", t)); }
                        if let Some(idle) = idle_sec { parts.push(format!("idle {}s", idle)); }
                        if let Some(ex) = exc_count { parts.push(format!("exc {}", ex)); }
                        if let Some(up) = uptime_sec {
                            let h = up / 3600; let m = (up % 3600) / 60; let s = up % 60;
                            if h > 0 { parts.push(format!("up {}h{}m", h, m)); }
                            else if m > 0 { parts.push(format!("up {}m{}s", m, s)); }
                            else { parts.push(format!("up {}s", s)); }
                        }
                        if let Some(sr) = since_reopen_sec { parts.push(format!("reopen {}s", sr)); }
                        if parts.is_empty() { "桥接: —".to_string() } else { format!("桥接: {}", parts.join(" ")) }
                    };

                    // 供托盘与前端使用的最佳风扇 RPM（优先 CPU 再机箱）
                    let fan_best = fan_opt.or(case_fan);

                    // 更新菜单只读信息（忽略错误）
                    let _ = info_cpu_c.set_text(&cpu_line);
                    let _ = info_mem_c.set_text(&mem_line);
                    let _ = info_temp_c.set_text(&temp_line);
                    let _ = info_fan_c.set_text(&fan_line);
                    let _ = info_net_c.set_text(&net_line);
                    let _ = info_disk_c.set_text(&disk_line);
                    let _ = info_store_c.set_text(&storage_line);
                    let _ = info_bridge_c.set_text(&bridge_line);

                    // 更新托盘 tooltip，避免一直停留在“初始化中”
                    let tooltip = format!(
                        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                        cpu_line, mem_line, temp_line, fan_line, net_line, disk_line, storage_line, bridge_line
                    );
                    let _ = tray_c.set_tooltip(Some(&tooltip));

                    // 托盘顶部文本：优先温度整数（如 65C），否则 CPU%
                    let top_text = if let Some(t) = temp_opt.map(|v| v.round() as i32) {
                        format!("{}C", t)
                    } else {
                        format!("{}%", cpu_usage.round() as u32)
                    };

                    // 读取配置决定底部文本：cpu% | mem% | fanRPM（无读数则回退 CPU%）
                    let mode = cfg_state_c
                        .lock().ok()
                        .and_then(|c| c.tray_bottom_mode.clone())
                        .unwrap_or_else(|| if cfg_state_c.lock().ok().map(|c| c.tray_show_mem).unwrap_or(false) { "mem".to_string() } else { "cpu".to_string() });
                    let bottom_text = match mode.as_str() {
                        "mem" => format!("{}%", mem_pct.round() as u32),
                        "fan" => match fan_best {
                            Some(rpm) if rpm > 0 => format!("{}", rpm), // 仅数字，节省宽度
                            _ => format!("{}%", cpu_usage.round() as u32), // 回退
                        },
                        _ => format!("{}%", cpu_usage.round() as u32),
                    };

                    let icon_img: Image = make_tray_icon(&top_text, &bottom_text);
                    let _ = tray_c.set_icon(Some(icon_img));

                    // 广播到前端
                    // 读取 Wi‑Fi 信息（Windows）
                    let wi = read_wifi_info_ext();
                    // 读取网络接口、逻辑磁盘
                    let net_ifs = match &wmi_fan_conn { Some(c) => wmi_list_net_ifs(c), None => None };
                    let logical_disks = match &wmi_fan_conn { Some(c) => wmi_list_logical_disks(c), None => None };
                    // SMART 健康：优先 ROOT\WMI 的 FailurePredictStatus，失败则回退 ROOT\CIMV2 的 DiskDrive.Status
                    let mut smart_health = match &wmi_temp_conn { Some(c) => wmi_list_smart_status(c), None => None };
                    if smart_health.is_none() {
                        smart_health = match &wmi_fan_conn { Some(c) => wmi_fallback_disk_status(c), None => None };
                    }

                    let now_ts = chrono::Local::now().timestamp_millis();
                    let snapshot = SensorSnapshot {
                        cpu_usage,
                        mem_used_gb: used_gb as f32,
                        mem_total_gb: total_gb as f32,
                        mem_pct: mem_pct as f32,
                        net_rx_bps: ema_net_rx,
                        net_tx_bps: ema_net_tx,
                        wifi_ssid: wi.ssid,
                        wifi_signal_pct: wi.signal_pct,
                        wifi_link_mbps: wi.link_mbps.or(wi.rx_mbps).or(wi.tx_mbps),
                        wifi_bssid: wi.bssid,
                        wifi_channel: wi.channel,
                        wifi_radio: wi.radio,
                        wifi_band: wi.band,
                        wifi_rx_mbps: wi.rx_mbps,
                        wifi_tx_mbps: wi.tx_mbps,
                        wifi_rssi_dbm: wi.rssi_dbm,
                        wifi_rssi_estimated: if wi.rssi_dbm.is_some() { Some(wi.rssi_estimated) } else { None },
                        net_ifs,
                        disk_r_bps: ema_disk_r,
                        disk_w_bps: ema_disk_w,
                        cpu_temp_c: temp_opt.map(|v| v as f32),
                        mobo_temp_c: bridge_mobo_temp,
                        fan_rpm: fan_best,
                        storage_temps,
                        logical_disks,
                        smart_health,
                        gpus,
                        hb_tick,
                        idle_sec,
                        exc_count,
                        uptime_sec,
                        cpu_pkg_power_w,
                        cpu_avg_freq_mhz,
                        cpu_throttle_active,
                        cpu_throttle_reasons,
                        since_reopen_sec,
                        cpu_core_loads_pct,
                        cpu_core_clocks_mhz,
                        cpu_core_temps_c,
                        disk_r_iops,
                        disk_w_iops,
                        disk_queue_len,
                        net_rx_err_ps,
                        net_tx_err_ps,
                        ping_rtt_ms,
                        timestamp_ms: now_ts,
                    };
                    eprintln!(
                        "[emit] sensor://snapshot ts={} cpu={:.0}% mem={:.0}% net_rx={} net_tx={}",
                        now_ts,
                        cpu_usage,
                        mem_pct,
                        ema_net_rx as u64,
                        ema_net_tx as u64
                    );
                    let _ = app_handle_c.emit("sensor://snapshot", snapshot);

                    thread::sleep(Duration::from_secs(1));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
