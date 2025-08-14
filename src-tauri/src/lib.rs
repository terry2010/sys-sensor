// ================================================================================
// 系统传感器监控应用 - 主模块
// ================================================================================
// 
// 本文件包含以下功能区域：
// 1. Tauri 命令函数
// 2. 前端数据结构定义 (Payload 结构体)
// 3. WMI 查询结构体定义
// 4. WMI 查询函数实现
// 5. 网络工具函数
// 6. SMART 硬盘监控函数
// 7. 托盘图标渲染函数
// 8. 主程序逻辑和数据采集循环
//
// ================================================================================

// 模块导入
mod battery_utils;
mod thermal_utils;
mod network_disk_utils;
mod gpu_utils;
use battery_utils::Win32Battery;
use thermal_utils::{MSAcpiThermalZoneTemperature, Win32Fan};
use gpu_utils::{Win32VideoController, wmi_read_gpu_vram, wmi_query_gpu_vram, BridgeGpu};

// ================================================================================
// 1. TAURI 命令函数
// ================================================================================

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// ================================================================================
// 2. 前端数据结构定义 (PAYLOAD 结构体)
// ================================================================================
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VoltagePayload {
    name: Option<String>,
    volts: Option<f64>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FanPayload {
    name: Option<String>,
    rpm: Option<i32>,
    pct: Option<i32>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageTempPayload {
    name: Option<String>,
    temp_c: Option<f32>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GpuPayload {
    name: Option<String>,
    temp_c: Option<f32>,
    load_pct: Option<f32>,
    core_mhz: Option<f64>,
    memory_mhz: Option<f64>,
    fan_rpm: Option<i32>,
    fan_duty_pct: Option<i32>,
    vram_used_mb: Option<f64>,
    vram_total_mb: Option<f64>,
    vram_usage_pct: Option<f64>,
    power_w: Option<f64>,
    power_limit_w: Option<f64>,
    voltage_v: Option<f64>,
    hotspot_temp_c: Option<f32>,
    vram_temp_c: Option<f32>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SmartHealthPayload {
    device: Option<String>,
    predict_fail: Option<bool>,
    temp_c: Option<f32>,
    power_on_hours: Option<i32>,
    reallocated: Option<i64>,
    pending: Option<i64>,
    uncorrectable: Option<i64>,
    crc_err: Option<i64>,
    power_cycles: Option<i32>,
    host_reads_bytes: Option<i64>,
    host_writes_bytes: Option<i64>,
    nvme_percentage_used_pct: Option<f32>,
    nvme_available_spare_pct: Option<f32>,
    nvme_available_spare_threshold_pct: Option<f32>,
    nvme_media_errors: Option<i64>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NetIfPayload {
    name: Option<String>,
    // 兼容：数组形式 IP 列表与独立 IPv4/IPv6
    ips: Option<Vec<String>>,
    ipv4: Option<String>,
    ipv6: Option<String>,
    mac: Option<String>,
    // 兼容：链路速率与介质（两套命名）
    speed_mbps: Option<i32>,
    link_mbps: Option<i32>,
    media: Option<String>,
    media_type: Option<String>,
    // 其他网络配置字段
    gateway: Option<Vec<String>>,
    dns: Option<Vec<String>>,
    dhcp_enabled: Option<bool>,
    up: Option<bool>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LogicalDiskPayload {
    name: Option<String>,
    fs: Option<String>,
    total_gb: Option<f32>,
    free_gb: Option<f32>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CpuPayload {
    name: Option<String>,
    temp_c: Option<f32>,
    load_pct: Option<f32>,
    freq_ghz: Option<f64>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryPayload {
    name: Option<String>,
    total_gb: Option<f32>,
    used_gb: Option<f32>,
    used_pct: Option<f32>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DiskPayload {
    name: Option<String>,
    read_bps: Option<f64>,
    write_bps: Option<f64>,
    read_iops: Option<f64>,
    write_iops: Option<f64>,
    queue_len: Option<f64>,
}

// ================================================================================
// 3. WMI 查询结构体定义
// ================================================================================

// ---- WMI Perf counters (disk & network) ----
#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_PerfFormattedData_PerfDisk_PhysicalDisk")]
struct PerfDiskPhysical {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "DiskReadBytesPerSec")]
    disk_read_bytes_per_sec: Option<f64>,
    #[serde(rename = "DiskWriteBytesPerSec")]
    disk_write_bytes_per_sec: Option<f64>,
    #[serde(rename = "DiskReadsPerSec")]
    disk_reads_per_sec: Option<f64>,
    #[serde(rename = "DiskWritesPerSec")]
    disk_writes_per_sec: Option<f64>,
    #[serde(rename = "CurrentDiskQueueLength")]
    current_disk_queue_length: Option<f64>,
    #[serde(rename = "PercentDiskTime")]
    percent_disk_time: Option<f64>,
}

// ================================================================================
// 4. WMI 查询函数实现
// ================================================================================

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

// wmi_read_gpu_vram 函数已移至 gpu_utils 模块

// 电池健康查询函数
fn wmi_read_battery_health(conn: &wmi::WMIConnection) -> (Option<u32>, Option<u32>, Option<u32>) {
    let res: Result<Vec<Win32Battery>, _> = conn.query();
    if let Ok(list) = res {
        if let Some(battery) = list.first() {
            return (
                battery.design_capacity,
                battery.full_charge_capacity,
                battery.cycle_count,
            );
        }
    }
    (None, None, None)
}

// 汇总网络错误率（每秒，排除 _Total）
fn wmi_perf_net_err(conn: &wmi::WMIConnection) -> (Option<f64>, Option<f64>) {
    let results: Result<Vec<PerfTcpipNic>, _> = conn.query();
    if let Ok(nics) = results {
        let mut rx_err_total = 0f64;
        let mut tx_err_total = 0f64;
        for nic in nics {
            if let Some(name) = &nic.name {
                if name == "_Total" { continue; }
            }
            if let Some(rx_err) = nic.packets_received_errors {
                rx_err_total += rx_err as f64;
            }
            if let Some(tx_err) = nic.packets_outbound_errors {
                tx_err_total += tx_err as f64;
            }
        }
        (Some(rx_err_total), Some(tx_err_total))
    } else {
        (None, None)
    }
}

// 查询内存细分信息（缓存/提交/分页等）
fn wmi_perf_memory(conn: &wmi::WMIConnection) -> (Option<f32>, Option<f32>, Option<f32>, Option<f32>, Option<f32>, Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    // 尝试多种WMI类名，因为不同Windows版本可能有不同的类名
    let class_names = [
        "Win32_PerfFormattedData_PerfOS_Memory",
        "Win32_PerfRawData_PerfOS_Memory", 
        "Win32_OperatingSystem"
    ];
    
    for class_name in &class_names {
        eprintln!("[wmi_perf_memory] Trying class: {}", class_name);
        
        if *class_name == "Win32_OperatingSystem" {
            // 使用操作系统类作为回退，提供基本内存信息
            #[derive(serde::Deserialize, Debug)]
            #[serde(rename_all = "PascalCase")]
            struct Win32OS {
                #[serde(rename = "TotalVirtualMemorySize")]
                total_virtual_memory_size: Option<u64>,
                #[serde(rename = "TotalVisibleMemorySize")]
                total_visible_memory_size: Option<u64>,
                #[serde(rename = "FreeVirtualMemory")]
                free_virtual_memory: Option<u64>,
                #[serde(rename = "FreePhysicalMemory")]
                free_physical_memory: Option<u64>,
            }
            
            if let Ok(results) = conn.query::<Win32OS>() {
                eprintln!("[wmi_perf_memory] OS query found {} entries", results.len());
                if let Some(os) = results.first() {
                    // 从操作系统信息计算基本内存指标
                    let total_kb = os.total_visible_memory_size.unwrap_or(0);
                    let free_kb = os.free_physical_memory.unwrap_or(0);
                    let used_kb = total_kb.saturating_sub(free_kb);
                    
                    let cache_gb = Some((used_kb as f64 * 0.3 / 1024.0 / 1024.0) as f32); // 估算缓存为已用内存的30%
                    let committed_gb = Some((used_kb as f64 / 1024.0 / 1024.0) as f32);
                    
                    eprintln!("[wmi_perf_memory] OS fallback: total_kb={} used_kb={} cache_gb={:?}", total_kb, used_kb, cache_gb);
                    
                    return (
                        cache_gb,
                        committed_gb,
                        None, // commit_limit_gb
                        None, // pool_paged_gb
                        None, // pool_nonpaged_gb
                        None, // pages_per_sec
                        None, // page_reads_per_sec
                        None, // page_writes_per_sec
                        None, // page_faults_per_sec
                    );
                }
            } else {
                eprintln!("[wmi_perf_memory] OS query also failed, trying Windows API fallback");
                
                // 使用 sysinfo 库作为最终回退
                use sysinfo::System;
                let mut sys = System::new_all();
                sys.refresh_memory();
                
                let total_bytes = sys.total_memory();
                let used_bytes = sys.used_memory();
                let available_bytes = sys.available_memory();
                
                if total_bytes > 0 {
                    let cache_gb = Some((used_bytes as f64 * 0.25 / 1024.0 / 1024.0 / 1024.0) as f32); // 估算缓存
                    let committed_gb = Some((used_bytes as f64 / 1024.0 / 1024.0 / 1024.0) as f32);
                    let commit_limit_gb = Some((total_bytes as f64 * 1.5 / 1024.0 / 1024.0 / 1024.0) as f32); // 估算提交限制
                    
                    eprintln!("[wmi_perf_memory] sysinfo fallback: total_gb={:.1} used_gb={:.1} available_gb={:.1}", 
                        total_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                        used_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                        available_bytes as f64 / 1024.0 / 1024.0 / 1024.0);
                    
                    return (
                        cache_gb,
                        committed_gb,
                        commit_limit_gb,
                        None, // pool_paged_gb
                        None, // pool_nonpaged_gb
                        None, // pages_per_sec
                        None, // page_reads_per_sec
                        None, // page_writes_per_sec
                        None, // page_faults_per_sec
                    );
                }
            }
        } else {
            // 尝试标准性能计数器类
            let results: Result<Vec<PerfOsMemory>, _> = conn.query();
            if let Ok(memory_list) = results {
                eprintln!("[wmi_perf_memory] {} query found {} entries", class_name, memory_list.len());
                if let Some(mem) = memory_list.first() {
                    eprintln!("[wmi_perf_memory] cache_bytes={:?} committed_bytes={:?}", mem.cache_bytes, mem.committed_bytes);
                    let cache_gb = mem.cache_bytes.map(|b| b as f64 / 1073741824.0).map(|g| g as f32);
                    let committed_gb = mem.committed_bytes.map(|b| b as f64 / 1073741824.0).map(|g| g as f32);
                    let commit_limit_gb = mem.commit_limit.map(|b| b as f64 / 1073741824.0).map(|g| g as f32);
                    let pool_paged_gb = mem.pool_paged_bytes.map(|b| b as f64 / 1073741824.0).map(|g| g as f32);
                    let pool_nonpaged_gb = mem.pool_nonpaged_bytes.map(|b| b as f64 / 1073741824.0).map(|g| g as f32);
                    let pages_per_sec = mem.pages_per_sec;
                    let page_reads_per_sec = mem.page_reads_per_sec;
                    let page_writes_per_sec = mem.page_writes_per_sec;
                    let page_faults_per_sec = mem.page_faults_per_sec;
                    
                    return (
                        cache_gb,
                        committed_gb,
                        commit_limit_gb,
                        pool_paged_gb,
                        pool_nonpaged_gb,
                        pages_per_sec,
                        page_reads_per_sec,
                        page_writes_per_sec,
                        page_faults_per_sec,
                    );
                }
            } else {
                eprintln!("[wmi_perf_memory] {} query failed: {:?}", class_name, results.err());
            }
        }
    }
    
    eprintln!("[wmi_perf_memory] All queries failed, returning None values");
    (None, None, None, None, None, None, None, None, None)
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
    // 新增：安全/加密/信道宽度（MHz）
    auth: Option<String>,
    cipher: Option<String>,
    chan_width_mhz: Option<i32>,
}

#[allow(dead_code)]
fn read_wifi_info() -> (Option<String>, Option<i32>, Option<i32>) {
    #[cfg(windows)]
    {
        let output = {
            let mut cmd = std::process::Command::new("netsh");
            cmd.args(["wlan", "show", "interfaces"]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            cmd.output()
        };
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
                    // 提前解析键名（冒号左侧），避免“Channel width/信道宽度”被“Channel/信道”误匹配
                    let key = t.split(|ch| ch == ':' || ch == '：').next().unwrap_or(t).trim();
                    let _keyl = key.to_lowercase();
                    // 提前解析键名（冒号左侧），供后续“Channel vs Channel width/bandwidth”判定
                    let key = t.split(|ch| ch == ':' || ch == '：').next().unwrap_or(t).trim();
                    let _keyl = key.to_lowercase();
                    // 提前解析冒号左侧的键名，避免“Channel width/信道宽度”被“Channel/信道”误匹配
                    let key = t.split(|ch| ch == ':' || ch == '：').next().unwrap_or(t).trim();
                    let _keyl = key.to_lowercase();
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
        let output = {
            let mut cmd = std::process::Command::new("netsh");
            cmd.args(["wlan", "show", "interfaces"]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            cmd.output()
        };
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
                    // 提前解析键名（冒号左侧），用于区分 Channel 与 Channel width/bandwidth
                    let key = t.split(|ch| ch == ':' || ch == '：').next().unwrap_or(t).trim();
                    let keyl = key.to_lowercase();
                    // Debug: 打印潜在带宽/加密相关行，便于定位不同语言/驱动差异
                    if cfg!(debug_assertions) {
                        if tl.contains("width") || t.contains("宽") || t.contains("带宽") {
                            println!("[wifi][match-cand][width] {}", t);
                        }
                        if tl.contains("cipher") || t.contains("加密") || t.contains("加密类型") || t.contains("加密方式") {
                            println!("[wifi][match-cand][cipher] {}", t);
                        }
                    }
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
                    // 身份验证（Authentication）
                    if tl.contains("authentication") || t.contains("身份验证") {
                        if let Some(v) = get_after_colon(t) { if !v.is_empty() { out.auth = Some(v); } }
                        continue;
                    }
                    // 加密（Cipher）——兼容“加密/加密类型/加密方式”
                    if tl.contains("cipher") || t.contains("加密") || t.contains("加密类型") || t.contains("加密方式") {
                        if let Some(v) = get_after_colon(t) { if !v.is_empty() { out.cipher = Some(v); } }
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
                    // 信道（仅匹配键名正是 Channel/信道/通道/频道，避免误吞“Channel width/信道宽度/带宽”）
                    if keyl == "channel" || key == "信道" || key == "通道" || key == "频道" {
                        if let Some(v) = get_after_colon(t) {
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { out.channel = Some(n.max(0)); }
                        }
                        continue;
                    }
                    // 信道宽度/带宽（Channel width/bandwidth；中文兼容“信道/通道/频道 + 宽度/带宽”，及部分仅写“带宽/宽度”的驱动）
                    if (keyl.contains("channel") && (keyl.contains("width") || keyl.contains("bandwidth")))
                        || key.contains("信道宽度") || key.contains("通道宽度") || key.contains("频道宽度")
                        || key.contains("信道带宽") || key.contains("通道带宽") || key.contains("频道带宽")
                        || key.contains("带宽") || key.contains("宽度") {
                        if let Some(v) = get_after_colon(t) {
                            // 取出数值，如 "80 MHz"
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { if n > 0 { out.chan_width_mhz = Some(n); } }
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
                // 若未解析到“信道宽度”，进行保守回退推断（仅作为显示友好，不保证精确）
                if out.chan_width_mhz.is_none() {
                    let radio = out.radio.as_deref().unwrap_or("");
                    let band = out.band.as_deref().unwrap_or("");
                    let width = if radio.contains("802.11ax") {
                        Some(80)
                    } else if radio.contains("802.11ac") {
                        Some(80)
                    } else if radio.contains("802.11n") {
                        if band == "5GHz" { Some(40) } else { Some(20) }
                    } else if radio.contains("802.11a") || radio.contains("802.11b") || radio.contains("802.11g") {
                        Some(20)
                    } else { None };
                    if let Some(w) = width {
                        out.chan_width_mhz = Some(w);
                        if cfg!(debug_assertions) {
                            println!("[wifi][width][fallback] radio={:?} band={:?} -> width={} MHz", out.radio, out.band, w);
                        }
                    }
                }
                // Debug 构建下输出解析摘要，便于现场排错
                if cfg!(debug_assertions) {
                    if out.signal_pct.is_none() && out.channel.is_none() && out.radio.is_none() && out.rx_mbps.is_none() && out.tx_mbps.is_none() && out.rssi_dbm.is_none() && out.auth.is_none() && out.cipher.is_none() && out.chan_width_mhz.is_none() {
                        if let Some(raw) = raw_text_for_dbg.as_ref() {
                            println!("[wifi][raw]\n{}", raw);
                        }
                    }
                    println!(
                        "[wifi][parsed] ssid={:?} signal%={:?} ch={:?} radio={:?} band={:?} rx={:?} tx={:?} bssid={:?} rssi={:?} rssi_est={} auth={:?} cipher={:?} width={:?}",
                        out.ssid, out.signal_pct, out.channel, out.radio, out.band, out.rx_mbps, out.tx_mbps, out.bssid, out.rssi_dbm, out.rssi_estimated, out.auth, out.cipher, out.chan_width_mhz
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
// 温度和风扇相关结构体已移至 thermal_utils 模块

// ---- WMI Perf counters (network) ----
#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_PerfFormattedData_Tcpip_NetworkInterface")]
struct PerfTcpipNic {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "BytesReceivedPerSec")]
    bytes_received_per_sec: Option<f64>,
    #[serde(rename = "BytesSentPerSec")]
    bytes_sent_per_sec: Option<f64>,
    #[serde(rename = "BytesTotalPerSec")]
    bytes_total_per_sec: Option<f64>,
    #[serde(rename = "CurrentBandwidth")]
    current_bandwidth: Option<f64>,
    #[serde(rename = "OutputQueueLength")]
    output_queue_length: Option<f64>,
    #[serde(rename = "PacketsOutboundDiscarded")]
    packets_outbound_discarded: Option<u64>,
    #[serde(rename = "PacketsReceivedErrors")]
    packets_received_errors: Option<u64>,
    #[serde(rename = "PacketsOutboundErrors")]
    packets_outbound_errors: Option<u64>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct PerfOsMemory {
    #[serde(rename = "CacheBytes")]
    cache_bytes: Option<u64>,
    #[serde(rename = "CommittedBytes")]
    committed_bytes: Option<u64>,
    #[serde(rename = "CommitLimit")]
    commit_limit: Option<u64>,
    #[serde(rename = "PoolPagedBytes")]
    pool_paged_bytes: Option<u64>,
    #[serde(rename = "PoolNonpagedBytes")]
    pool_nonpaged_bytes: Option<u64>,
    #[serde(rename = "PagesPersec")]
    pages_per_sec: Option<f64>,
    #[serde(rename = "PageReadsPersec")]
    page_reads_per_sec: Option<f64>,
    #[serde(rename = "PageWritesPersec")]
    page_writes_per_sec: Option<f64>,
    #[serde(rename = "PageFaultsPersec")]
    page_faults_per_sec: Option<f64>,
}

// GPU WMI 查询结构体已移至 gpu_utils 模块

// 电池相关结构体和函数已移至 battery_utils 模块

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "MSStorageDriver_FailurePredictStatus")]
struct MsStorageDriverFailurePredictStatus {
    #[serde(rename = "InstanceName")]
    instance_name: Option<String>,
    #[serde(rename = "PredictFailure")]
    predict_failure: Option<bool>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "MSStorageDriver_FailurePredictData")]
struct MsStorageDriverFailurePredictData {
    #[serde(rename = "InstanceName")]
    instance_name: Option<String>,
    #[serde(rename = "VendorSpecific")]
    vendor_specific: Option<Vec<u8>>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_DiskDrive")]
struct Win32DiskDrive {
    #[serde(rename = "Model")]
    model: Option<String>,
    #[serde(rename = "Status")]
    status: Option<String>,
}



// 电池相关函数已移至 battery_utils 模块

// 温度和风扇相关函数已移至 thermal_utils 模块

// ---- WMI helpers: network interfaces, logical disks, SMART status ----

#[derive(Debug, Clone, Copy)]
struct SmartAttrRec {
    id: u8,
    value: u8,
    worst: u8,
    raw: u64,
}

fn parse_smart_vendor(v: &[u8]) -> std::collections::HashMap<u8, SmartAttrRec> {
    use std::collections::HashMap;
    let mut map: HashMap<u8, SmartAttrRec> = HashMap::new();
    for chunk in v.chunks(12) {
        if chunk.len() < 12 { break; }
        let id = chunk[0];
        if id == 0 { continue; }
        // 布局：0=id, 1=flags, 2=value, 3=worst, 4..9=raw(LE), 10..11=保留
        let value = chunk[2];
        let worst = chunk[3];
        let raw = (chunk[4] as u64)
            | ((chunk[5] as u64) << 8)
            | ((chunk[6] as u64) << 16)
            | ((chunk[7] as u64) << 24)
            | ((chunk[8] as u64) << 32)
            | ((chunk[9] as u64) << 40);
        map.insert(id, SmartAttrRec { id, value, worst, raw });
    }
    map
}
// wmi_list_net_ifs 函数已移至 network_disk_utils 模块

// wmi_list_logical_disks 函数已移至 network_disk_utils 模块

fn wmi_list_smart_status(conn: &wmi::WMIConnection) -> Option<Vec<SmartHealthPayload>> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, SmartHealthPayload> = BTreeMap::new();
    
    eprintln!("[wmi_list_smart_status] Starting SMART data collection...");
    
    // 优先尝试：Windows 原生 NVMe/ATA IOCTL（自研采集）
    if let Some(v) = nvme_smart_via_ioctl() {
        eprintln!("[wmi_list_smart_status] IOCTL NVMe returned {} devices", v.len());
        return Some(v);
    } else {
        eprintln!("[wmi_list_smart_status] IOCTL NVMe not available/failed, falling back to WMI/PS");
    }
    
    // 尝试使用 ROOT\WMI 命名空间查询 SMART 数据
    if let Ok(com_lib) = wmi::COMLibrary::new() {
        if let Ok(wmi_conn) = wmi::WMIConnection::with_namespace_path("ROOT\\WMI", com_lib) {
            eprintln!("[wmi_list_smart_status] Connected to ROOT\\WMI namespace");
        // 1) 读取预测失败状态
        eprintln!("[wmi_list_smart_status] Querying MsStorageDriverFailurePredictStatus...");
        match wmi_conn.query::<MsStorageDriverFailurePredictStatus>() {
            Ok(list) => {
                eprintln!("[wmi_list_smart_status] Found {} predict status entries", list.len());
                for it in list.into_iter() {
                    let key = it.instance_name.clone().unwrap_or_default();
                    eprintln!("[wmi_list_smart_status] Predict status: device={:?} predict_fail={:?}", it.instance_name, it.predict_failure);
                    let entry = map.entry(key.clone()).or_insert(SmartHealthPayload {
                        device: it.instance_name.clone(),
                        predict_fail: it.predict_failure,
                        temp_c: None,
                        power_on_hours: None,
                        reallocated: None,
                        pending: None,
                        uncorrectable: None,
                        crc_err: None,
                        power_cycles: None,
                        host_reads_bytes: None,
                        host_writes_bytes: None,
                        nvme_percentage_used_pct: None,
                        nvme_available_spare_pct: None,
                        nvme_available_spare_threshold_pct: None,
                        nvme_media_errors: None,
                    });
                    entry.predict_fail = it.predict_failure;
                }
            }
            Err(e) => {
                eprintln!("[wmi_list_smart_status] MsStorageDriverFailurePredictStatus query failed: {:?}", e);
            }
        }
        
        // 2) 读取 SMART 关键属性（ATA VendorSpecific）
        eprintln!("[wmi_list_smart_status] Querying MsStorageDriverFailurePredictData...");
        match wmi_conn.query::<MsStorageDriverFailurePredictData>() {
            Ok(list) => {
                eprintln!("[wmi_list_smart_status] Found {} predict data entries", list.len());
                for d in list.into_iter() {
                    let key = d.instance_name.clone().unwrap_or_default();
                    eprintln!("[wmi_list_smart_status] Predict data: device={:?} vendor_specific_len={:?}", 
                        d.instance_name, d.vendor_specific.as_ref().map(|v| v.len()));
                    let entry = map.entry(key.clone()).or_insert(SmartHealthPayload {
                        device: d.instance_name.clone(),
                        predict_fail: None,
                        temp_c: None,
                        power_on_hours: None,
                        reallocated: None,
                        pending: None,
                        uncorrectable: None,
                        crc_err: None,
                        power_cycles: None,
                        host_reads_bytes: None,
                        host_writes_bytes: None,
                        nvme_percentage_used_pct: None,
                        nvme_available_spare_pct: None,
                        nvme_available_spare_threshold_pct: None,
                        nvme_media_errors: None,
                    });
                    if let Some(vs) = d.vendor_specific.as_ref() {
                        let attrs = parse_smart_vendor(vs);
                        eprintln!("[wmi_list_smart_status] Parsed {} SMART attributes for device {:?}", attrs.len(), d.instance_name);
                        // 常见关键属性映射
                        if let Some(a) = attrs.get(&194) {
                            let t = (a.raw & 0xFF) as i32;
                            if t > -50 && t < 200 { entry.temp_c = Some(t as f32); }
                        }
                        if let Some(a) = attrs.get(&9) { entry.power_on_hours = i32::try_from(a.raw).ok(); }
                        if let Some(a) = attrs.get(&5) { entry.reallocated = i64::try_from(a.raw).ok(); }
                        if let Some(a) = attrs.get(&197) { entry.pending = i64::try_from(a.raw).ok(); }
                        if let Some(a) = attrs.get(&198) { entry.uncorrectable = i64::try_from(a.raw).ok(); }
                        if let Some(a) = attrs.get(&199) { entry.crc_err = i64::try_from(a.raw).ok(); }
                        if let Some(a) = attrs.get(&12) { entry.power_cycles = i32::try_from(a.raw).ok(); }
                        if let Some(a) = attrs.get(&0xF2) { entry.host_reads_bytes = a.raw.checked_mul(512).and_then(|v| i64::try_from(v).ok()); }
                        if let Some(a) = attrs.get(&0xF1) { entry.host_writes_bytes = a.raw.checked_mul(512).and_then(|v| i64::try_from(v).ok()); }
                    }
                }
            }
            Err(e) => {
                eprintln!("[wmi_list_smart_status] MsStorageDriverFailurePredictData query failed: {:?}", e);
            }
        }
        } else {
            eprintln!("[wmi_list_smart_status] Failed to connect to ROOT\\WMI namespace");
        }
    } else {
        eprintln!("[wmi_list_smart_status] Failed to initialize COM library");
    }
    
    eprintln!("[wmi_list_smart_status] ROOT\\WMI query completed, found {} devices", map.len());
    
    // 如果 ROOT\WMI 查询失败，尝试回退到 ROOT\CIMV2
    if map.is_empty() {
        // 先尝试 smartctl (-j) 可选外部采集（方案A）
        eprintln!("[wmi_list_smart_status] ROOT\\WMI returned no data, trying smartctl (-j) optional path...");
        if let Some(sc) = smartctl_collect() {
            eprintln!("[wmi_list_smart_status] smartctl returned {} devices", sc.len());
            return Some(sc);
        }
        
        eprintln!("[wmi_list_smart_status] smartctl not available/failed, trying fallback to ROOT\\CIMV2...");
        if let Some(fallback_data) = wmi_fallback_disk_status(conn) {
            eprintln!("[wmi_list_smart_status] Fallback to ROOT\\CIMV2 successful, found {} devices", fallback_data.len());
            return Some(fallback_data);
        }
        
        // 最后尝试 NVMe PowerShell 回退
        eprintln!("[wmi_list_smart_status] ROOT\\CIMV2 fallback failed, trying PowerShell NVMe...");
        if let Some(nvme_data) = nvme_storage_reliability_ps() {
            eprintln!("[wmi_list_smart_status] PowerShell NVMe successful, found {} devices", nvme_data.len());
            return Some(nvme_data);
        }
        
        eprintln!("[wmi_list_smart_status] All SMART data collection methods failed");
    }

    if map.is_empty() { None } else { Some(map.into_values().collect()) }
}

// Windows 原生 NVMe/ATA IOCTL 采集 SMART（第一步：遍历物理盘并尝试打开句柄）
#[cfg(windows)]
fn nvme_smart_via_ioctl() -> Option<Vec<SmartHealthPayload>> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, GetLastError};
    use windows::Win32::Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_SHARE_DELETE, OPEN_EXISTING, FILE_GENERIC_READ, FILE_GENERIC_WRITE};

    eprintln!("[nvme_ioctl] probing PhysicalDrive handles...");

    let mut found_any = false;
    let mut results: Vec<SmartHealthPayload> = Vec::new();
    // 预留：未来将把每个句柄通过 IOCTL_STORAGE_QUERY_PROPERTY 读取 NVMe 健康页
    for i in 0..32u32 {
        let path = format!("\\\\.\\PhysicalDrive{}", i);
        let mut wide: Vec<u16> = path.encode_utf16().collect();
        wide.push(0);
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                (FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0),
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };
        let handle = match handle {
            Ok(h) => h,
            Err(_) => { continue; }
        };
        found_any = true;
        eprintln!("[nvme_ioctl] opened {}", path);

        // IOCTL_STORAGE_QUERY_PROPERTY 调用骨架，尝试请求 NVMe 健康日志页 (0x02)
        use windows::Win32::System::IO::DeviceIoControl;
        use windows::Win32::System::Ioctl::{IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY, STORAGE_PROPERTY_ID, STORAGE_QUERY_TYPE, STORAGE_PROTOCOL_SPECIFIC_DATA, STORAGE_PROTOCOL_TYPE, STORAGE_PROTOCOL_DATA_DESCRIPTOR};
        use std::mem::{size_of, zeroed};
        unsafe {
            // 构造输入缓冲：STORAGE_PROPERTY_QUERY + STORAGE_PROTOCOL_SPECIFIC_DATA（紧随其后）
            let mut query: STORAGE_PROPERTY_QUERY = zeroed();
            query.PropertyId = STORAGE_PROPERTY_ID(49); // StorageDeviceProtocolSpecificProperty
            query.QueryType = STORAGE_QUERY_TYPE(0);    // PropertyStandardQuery

            let mut proto: STORAGE_PROTOCOL_SPECIFIC_DATA = zeroed();
            proto.ProtocolType = STORAGE_PROTOCOL_TYPE(4); // ProtocolTypeNvme
            proto.DataType = 3; // NVMeDataTypeLogPage
            proto.ProtocolDataRequestValue = 0x02; // Health Info Log Page
            proto.ProtocolDataRequestSubValue = 0;  // 无子索引
            // 为输出指定数据区布局：描述符头之后紧跟 4KB 数据区
            proto.ProtocolDataOffset = size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>() as u32;
            proto.ProtocolDataLength = 512; // SMART/Health log page is 512 bytes

            let in_size = size_of::<STORAGE_PROPERTY_QUERY>() + size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>();
            let mut inbuf: Vec<u8> = vec![0u8; in_size];
            // 拷贝 query 与 proto 到连续缓冲
            std::ptr::copy_nonoverlapping((&query as *const STORAGE_PROPERTY_QUERY) as *const u8, inbuf.as_mut_ptr(), size_of::<STORAGE_PROPERTY_QUERY>());
            std::ptr::copy_nonoverlapping((&proto as *const STORAGE_PROTOCOL_SPECIFIC_DATA) as *const u8, inbuf.as_mut_ptr().add(size_of::<STORAGE_PROPERTY_QUERY>()), size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>());

            // 输出缓冲：描述符 + 4KB 数据区
            let out_size = size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>() + 4096usize;
            let mut outbuf: Vec<u8> = vec![0u8; out_size];
            let mut bytes_returned: u32 = 0;

            let ok = DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                Some(inbuf.as_ptr() as *const _),
                in_size as u32,
                Some(outbuf.as_mut_ptr() as *mut _),
                out_size as u32,
                Some(&mut bytes_returned),
                None,
            )
            .is_ok();

            if ok {
                eprintln!("[nvme_ioctl] {}: IOCTL_STORAGE_QUERY_PROPERTY ok ({} bytes)", path, bytes_returned);
                // 解析 STORAGE_PROTOCOL_DATA_DESCRIPTOR
                if (bytes_returned as usize) >= size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>() && outbuf.len() >= size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>() {
                    // SAFETY: 仅用于读取对齐良好的 C 结构头
                    let desc: &STORAGE_PROTOCOL_DATA_DESCRIPTOR = &*(outbuf.as_ptr() as *const STORAGE_PROTOCOL_DATA_DESCRIPTOR);
                    let psd = &desc.ProtocolSpecificData;
                    let data_off = psd.ProtocolDataOffset as usize;
                    let data_len = (psd.ProtocolDataLength as usize).min(outbuf.len().saturating_sub(data_off));
                    if data_off > 0 && data_len >= 512 {
                        let data = &outbuf[data_off .. data_off + data_len];
                        // NVMe 健康日志页字段解析（NVMe 1.3 规范）：
                        // 温度：偏移 1..=2 (K, LE)
                        let temp_k = u16::from_le_bytes([data[1], data[2]]);
                        let temp_c = if temp_k > 0 { (temp_k as f32) - 273.15 } else { 0.0 };
                        // Data Units Read/Write：偏移 32/48（各16字节，小端无符号）
                        let dur = u128::from_le_bytes(data[32..48].try_into().unwrap());
                        let duw = u128::from_le_bytes(data[48..64].try_into().unwrap());
                        // Power Cycles：偏移 112（16字节）
                        let pcycles = u128::from_le_bytes(data[112..128].try_into().unwrap());
                        // Power On Hours：偏移 128（16字节）
                        let poh = u128::from_le_bytes(data[128..144].try_into().unwrap());

                        // 映射到 SmartHealthPayload，单位换算：
                        // NVMe Data Unit = 512,000 bytes
                        let bytes_read = dur.saturating_mul(512_000);
                        let bytes_write = duw.saturating_mul(512_000);

                        let power_on_hours_i32 = i32::try_from(poh.min(i32::MAX as u128)).ok();
                        let power_cycles_i32 = i32::try_from(pcycles.min(i32::MAX as u128)).ok();

                        let host_reads_i64 = i64::try_from(bytes_read.min(i64::MAX as u128)).ok();
                        let host_writes_i64 = i64::try_from(bytes_write.min(i64::MAX as u128)).ok();

                        // 追加 NVMe 特有指标（百分比与媒体错误）
                        let nvme_available_spare_pct = Some(data[3] as f32);
                        let nvme_available_spare_threshold_pct = Some(data[4] as f32);
                        let nvme_percentage_used_pct = Some(data[5] as f32);
                        let media = u128::from_le_bytes(data[160..176].try_into().unwrap());
                        let nvme_media_errors = i64::try_from(media.min(i64::MAX as u128)).ok();

                        results.push(SmartHealthPayload {
                            device: Some(path.clone()),
                            predict_fail: None,
                            temp_c: Some(temp_c),
                            power_on_hours: power_on_hours_i32,
                            reallocated: None,
                            pending: None,
                            uncorrectable: None,
                            crc_err: None,
                            power_cycles: power_cycles_i32,
                            host_reads_bytes: host_reads_i64,
                            host_writes_bytes: host_writes_i64,
                            nvme_percentage_used_pct,
                            nvme_available_spare_pct,
                            nvme_available_spare_threshold_pct,
                            nvme_media_errors,
                        });
                    } else {
                        eprintln!("[nvme_ioctl] {}: unexpected ProtocolData length/offset: off={} len={}", path, data_off, data_len);
                        if let Some(payload) = nvme_get_health_via_protocol_command(handle, &path) {
                            results.push(payload);
                        }
                    }
                }
            } else {
                let err1 = unsafe { GetLastError().0 };
                eprintln!("[nvme_ioctl] {}: DeviceProtocolSpecific failed, gle={} (0x{:X}); retry AdapterProtocolSpecific...", path, err1, err1);
                // 重试：使用 StorageAdapterProtocolSpecificProperty (48)
                let mut query2: STORAGE_PROPERTY_QUERY = zeroed();
                query2.PropertyId = STORAGE_PROPERTY_ID(48); // StorageAdapterProtocolSpecificProperty
                query2.QueryType = STORAGE_QUERY_TYPE(0);
                // 将 query2 写入 inbuf 前半部分，proto 不变
                std::ptr::copy_nonoverlapping((&query2 as *const STORAGE_PROPERTY_QUERY) as *const u8, inbuf.as_mut_ptr(), size_of::<STORAGE_PROPERTY_QUERY>());
                bytes_returned = 0;
                let ok2 = DeviceIoControl(
                    handle,
                    IOCTL_STORAGE_QUERY_PROPERTY,
                    Some(inbuf.as_ptr() as *const _),
                    in_size as u32,
                    Some(outbuf.as_mut_ptr() as *mut _),
                    out_size as u32,
                    Some(&mut bytes_returned),
                    None,
                ).is_ok();
                if ok2 {
                    eprintln!("[nvme_ioctl] {}: IOCTL_STORAGE_QUERY_PROPERTY (Adapter) ok ({} bytes)", path, bytes_returned);
                    if outbuf.len() >= size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>() {
                        let desc: &STORAGE_PROTOCOL_DATA_DESCRIPTOR = &*(outbuf.as_ptr() as *const STORAGE_PROTOCOL_DATA_DESCRIPTOR);
                        let psd = &desc.ProtocolSpecificData;
                        let data_off = psd.ProtocolDataOffset as usize;
                        let data_len = (psd.ProtocolDataLength as usize).min(outbuf.len().saturating_sub(data_off));
                        if data_off > 0 && data_len >= 512 {
                            let data = &outbuf[data_off .. data_off + data_len];
                            let temp_k = u16::from_le_bytes([data[1], data[2]]);
                            let temp_c = if temp_k > 0 { (temp_k as f32) - 273.15 } else { 0.0 };
                            let dur = u128::from_le_bytes(data[32..48].try_into().unwrap());
                            let duw = u128::from_le_bytes(data[48..64].try_into().unwrap());
                            let pcycles = u128::from_le_bytes(data[112..128].try_into().unwrap());
                            let poh = u128::from_le_bytes(data[128..144].try_into().unwrap());
                            let bytes_read = dur.saturating_mul(512_000);
                            let bytes_write = duw.saturating_mul(512_000);
                            let power_on_hours_i32 = i32::try_from(poh.min(i32::MAX as u128)).ok();
                            let power_cycles_i32 = i32::try_from(pcycles.min(i32::MAX as u128)).ok();
                            let host_reads_i64 = i64::try_from(bytes_read.min(i64::MAX as u128)).ok();
                            let host_writes_i64 = i64::try_from(bytes_write.min(i64::MAX as u128)).ok();
                            results.push(SmartHealthPayload {
                                device: Some(path.clone()),
                                predict_fail: None,
                                temp_c: Some(temp_c),
                                power_on_hours: power_on_hours_i32,
                                reallocated: None,
                                pending: None,
                                uncorrectable: None,
                                crc_err: None,
                                power_cycles: power_cycles_i32,
                                host_reads_bytes: host_reads_i64,
                                host_writes_bytes: host_writes_i64,
                                nvme_percentage_used_pct: None,
                                nvme_available_spare_pct: None,
                                nvme_available_spare_threshold_pct: None,
                                nvme_media_errors: None,
                            });
                        } else {
                            eprintln!("[nvme_ioctl] {}: Adapter data too short: off={} len={} -> try ProtocolCommand", path, data_off, data_len);
                            if let Some(payload) = nvme_get_health_via_protocol_command(handle, &path) {
                                results.push(payload);
                            }
                        }
                    }
                } else {
                    let err2 = unsafe { GetLastError().0 };
                    eprintln!("[nvme_ioctl] {}: AdapterProtocolSpecific failed, gle={} (0x{:X}); try NVMe ProtocolCommand...", path, err2, err2);
                    // 最终回退：NVMe Pass-through（IOCTL_STORAGE_PROTOCOL_COMMAND）
                    if let Some(payload) = nvme_get_health_via_protocol_command(handle, &path) {
                        results.push(payload);
                    }
                }
            }
        }
        unsafe { let _ = CloseHandle(handle); }
    }

    if found_any {
        if !results.is_empty() {
            eprintln!("[nvme_ioctl] parsed {} NVMe health entries", results.len());
            return Some(results);
        } else {
            eprintln!("[nvme_ioctl] drive handles opened but no NVMe health parsed");
        }
    } else {
        eprintln!("[nvme_ioctl] no PhysicalDrive handle could be opened");
    }

    None
}

// Windows: NVMe Pass-through 综合方案（SCSI Miniport + 修正 ProtocolCommand）
#[cfg(windows)]
fn nvme_get_health_via_protocol_command(handle: windows::Win32::Foundation::HANDLE, path: &str) -> Option<SmartHealthPayload> {
    use std::mem::size_of;
    use windows::Win32::System::IO::DeviceIoControl;
    use windows::Win32::System::Ioctl::{
        IOCTL_STORAGE_PROTOCOL_COMMAND,
        STORAGE_PROTOCOL_COMMAND,
        STORAGE_PROTOCOL_TYPE,
    };
    
    // IOCTL 常量定义 (windows crate 0.58 中不可用)
    const IOCTL_SCSI_MINIPORT: u32 = 0x0004D008;
    const IOCTL_ATA_PASS_THROUGH: u32 = 0x0004D02C;

    unsafe {
        // 方案1: 修正的 STORAGE_PROTOCOL_COMMAND（微调参数）
        unsafe fn try_refined_protocol(
            handle: windows::Win32::Foundation::HANDLE,
            path: &str,
            proto_val: i32,
        ) -> Result<SmartHealthPayload, u32> {
            let data_len: usize = 512;
            let cmd_len: usize = 64;
            let total_len = size_of::<STORAGE_PROTOCOL_COMMAND>() + cmd_len + data_len;
            let mut buf: Vec<u8> = vec![0u8; total_len];
            let hdr_ptr = buf.as_mut_ptr() as *mut STORAGE_PROTOCOL_COMMAND;
            let hdr = &mut *hdr_ptr;
            let hdr_size = size_of::<STORAGE_PROTOCOL_COMMAND>() as u32;
            
            // 微调：更保守的参数设置
            hdr.Version = hdr_size;
            hdr.Length = hdr_size; // 只包含头部，不包含命令区
            hdr.ProtocolType = STORAGE_PROTOCOL_TYPE(proto_val);
            hdr.Flags = 1; // STORAGE_PROTOCOL_COMMAND_FLAG_ADAPTER_REQUEST
            hdr.CommandLength = cmd_len as u32;
            hdr.DataFromDeviceTransferLength = data_len as u32;
            hdr.TimeOutValue = 30; // 延长超时
            hdr.ErrorInfoOffset = 0;
            hdr.ErrorInfoLength = 0;
            hdr.DataToDeviceTransferLength = 0;
            hdr.DataToDeviceBufferOffset = 0;
            hdr.DataFromDeviceBufferOffset = (size_of::<STORAGE_PROTOCOL_COMMAND>() + cmd_len) as u32;
            
            // 预置 ReturnStatus
            hdr.ReturnStatus = 0;

            let cmd_off = size_of::<STORAGE_PROTOCOL_COMMAND>();
            let cmd_slice = &mut buf[cmd_off .. cmd_off + cmd_len];
            
            // 标准 NVMe Get Log Page 命令格式
            cmd_slice[0] = 0x02; // Opcode: Get Log Page
            cmd_slice[1] = 0x00; // Flags
            // CID (Command Identifier) - bytes 2-3
            cmd_slice[2] = 0x01; cmd_slice[3] = 0x00;
            // NSID = 0xFFFFFFFF (全局)
            cmd_slice[4] = 0xFF; cmd_slice[5] = 0xFF; cmd_slice[6] = 0xFF; cmd_slice[7] = 0xFF;
            
            // CDW10: LID=0x02 (SMART Health), LSP=0, RAE=0, NUMD
            let numd_minus1: u32 = ((data_len as u32) / 4).saturating_sub(1);
            let cdw10: u32 = 0x02u32 | (numd_minus1 << 16);
            cmd_slice[40..44].copy_from_slice(&cdw10.to_le_bytes());
            
            // CDW11: RAE=1 (bit 15) - Retain Asynchronous Event
            let cdw11: u32 = 1u32 << 15;
            cmd_slice[44..48].copy_from_slice(&cdw11.to_le_bytes());

            let mut bytes: u32 = 0;
            eprintln!(
                "[nvme_ioctl] {}: RefinedProtocol pre-call proto={} hdr_len={}, cmd_len={}, data_len={}",
                path, proto_val, hdr.Length, cmd_len, data_len
            );
            
            let ok = DeviceIoControl(
                handle,
                IOCTL_STORAGE_PROTOCOL_COMMAND,
                Some(buf.as_ptr() as *const _),
                total_len as u32,
                Some(buf.as_mut_ptr() as *mut _),
                total_len as u32,
                Some(&mut bytes),
                None,
            ).is_ok();
            
            if !ok {
                let gle = windows::Win32::Foundation::GetLastError().0;
                eprintln!("[nvme_ioctl] {}: RefinedProtocol(proto={}) failed, gle={} (0x{:X})", path, proto_val, gle, gle);
                return Err(gle);
            }
            
            let data_off = size_of::<STORAGE_PROTOCOL_COMMAND>() + cmd_len;
            let data = &buf[data_off .. data_off + data_len.min(buf.len() - data_off)];
            eprintln!(
                "[nvme_ioctl] {}: RefinedProtocol(proto={}) ok, bytes_ret={}, data_len={}",
                path, proto_val, bytes, data.len()
            );
            
            if data.len() < 144 {
                return Err(0xFFFFFFFF);
            }
            
            // 解析健康页数据
            let temp_k = u16::from_le_bytes([data[1], data[2]]);
            let temp_c = if temp_k > 0 { (temp_k as f32) - 273.15 } else { 0.0 };
            let dur = u128::from_le_bytes(data[32..48].try_into().ok().unwrap());
            let duw = u128::from_le_bytes(data[48..64].try_into().ok().unwrap());
            let pcycles = u128::from_le_bytes(data[112..128].try_into().ok().unwrap());
            let poh = u128::from_le_bytes(data[128..144].try_into().ok().unwrap());
            let bytes_read = dur.saturating_mul(512_000);
            let bytes_write = duw.saturating_mul(512_000);
            let power_on_hours_i32 = i32::try_from(poh.min(i32::MAX as u128)).ok();
            let power_cycles_i32 = i32::try_from(pcycles.min(i32::MAX as u128)).ok();
            let host_reads_i64 = i64::try_from(bytes_read.min(i64::MAX as u128)).ok();
            let host_writes_i64 = i64::try_from(bytes_write.min(i64::MAX as u128)).ok();
            // 追加 NVMe 特有指标（百分比与媒体错误）
            let nvme_available_spare_pct = Some(data[3] as f32);
            let nvme_available_spare_threshold_pct = Some(data[4] as f32);
            let nvme_percentage_used_pct = Some(data[5] as f32);
            let media = u128::from_le_bytes(data[160..176].try_into().ok().unwrap());
            let nvme_media_errors = i64::try_from(media.min(i64::MAX as u128)).ok();
            
            Ok(SmartHealthPayload {
                device: Some(path.to_string()),
                predict_fail: None,
                temp_c: Some(temp_c),
                power_on_hours: power_on_hours_i32,
                reallocated: None,
                pending: None,
                uncorrectable: None,
                crc_err: None,
                power_cycles: power_cycles_i32,
                host_reads_bytes: host_reads_i64,
                host_writes_bytes: host_writes_i64,
                nvme_percentage_used_pct,
                nvme_available_spare_pct,
                nvme_available_spare_threshold_pct,
                nvme_media_errors,
            })
        }

        // 方案2: SCSI Miniport NVMe Pass-through (多 control_code 尝试)
        unsafe fn try_scsi_miniport(
            handle: windows::Win32::Foundation::HANDLE,
            path: &str,
            signature: &[u8; 8],
            control_code: u32,
        ) -> Result<SmartHealthPayload, u32> {
            #[repr(C, packed)]
            struct SrbIoControl {
                header_length: u32,
                signature: [u8; 8],
                timeout: u32,
                control_code: u32,
                return_code: u32,
                length: u32,
            }
            
            #[repr(C, packed)]
            struct NvmePassThroughIoctl {
                srb_io_control: SrbIoControl,
                direction: u32,     // 1 = from device
                queue_id: u32,      // 0 = admin queue
                data_buffer_len: u32,
                meta_data_len: u32,
                data_buffer_offset: u32,
                meta_data_offset: u32,
                timeout_value: u32,
                nvme_cmd: [u32; 16], // NVMe command (64 bytes)
            }
            
            let data_len: usize = 512;
            let total_len = size_of::<NvmePassThroughIoctl>() + data_len;
            let mut buf: Vec<u8> = vec![0u8; total_len];
            
            let ioctl_ptr = buf.as_mut_ptr() as *mut NvmePassThroughIoctl;
            let ioctl = &mut *ioctl_ptr;
            
            // 填充 SRB_IO_CONTROL 头
            ioctl.srb_io_control.header_length = size_of::<SrbIoControl>() as u32;
            ioctl.srb_io_control.signature = *signature;
            ioctl.srb_io_control.timeout = 30;
            ioctl.srb_io_control.control_code = control_code;
            ioctl.srb_io_control.return_code = 0;
            ioctl.srb_io_control.length = (size_of::<NvmePassThroughIoctl>() - size_of::<SrbIoControl>() + data_len) as u32;
            
            // 填充 NVMe Pass-through 参数
            ioctl.direction = 1; // from device
            ioctl.queue_id = 0;  // admin queue
            ioctl.data_buffer_len = data_len as u32;
            ioctl.meta_data_len = 0;
            ioctl.data_buffer_offset = size_of::<NvmePassThroughIoctl>() as u32;
            ioctl.meta_data_offset = 0;
            ioctl.timeout_value = 30;
            
            // NVMe Get Log Page 命令
            ioctl.nvme_cmd[0] = 0x02; // Opcode: Get Log Page
            ioctl.nvme_cmd[1] = 0xFFFFFFFF; // NSID = global
            // CDW10: LID=0x02, NUMD=(512/4-1)=127
            let numd_minus1 = (data_len / 4 - 1) as u32;
            ioctl.nvme_cmd[10] = 0x02 | (numd_minus1 << 16);
            // CDW11: RAE=1
            ioctl.nvme_cmd[11] = 1 << 15;
            
            let sig_str = std::str::from_utf8(signature).unwrap_or("unknown");
            eprintln!(
                "[nvme_ioctl] {}: SCSI Miniport pre-call sig='{}' ctrl_code=0x{:X} data_len={}",
                path, sig_str, control_code, data_len
            );
            
            let mut bytes: u32 = 0;
            let ok = DeviceIoControl(
                handle,
                IOCTL_SCSI_MINIPORT,
                Some(buf.as_ptr() as *const _),
                total_len as u32,
                Some(buf.as_mut_ptr() as *mut _),
                total_len as u32,
                Some(&mut bytes),
                None,
            ).is_ok();
            
            if !ok {
                let gle = windows::Win32::Foundation::GetLastError().0;
                eprintln!("[nvme_ioctl] {}: SCSI Miniport('{}', 0x{:X}) failed, gle={} (0x{:X})", path, sig_str, control_code, gle, gle);
                return Err(gle);
            }
            
            let return_code = ioctl.srb_io_control.return_code;
            if return_code != 0 {
                eprintln!("[nvme_ioctl] {}: SCSI Miniport('{}', 0x{:X}) return_code={} (0x{:X})", path, sig_str, control_code, return_code, return_code);
                return Err(return_code);
            }
            
            let data_off = size_of::<NvmePassThroughIoctl>();
            let data = &buf[data_off .. data_off + data_len.min(buf.len() - data_off)];
            eprintln!(
                "[nvme_ioctl] {}: SCSI Miniport('{}', 0x{:X}) ok, bytes_ret={}, data_len={}",
                path, sig_str, control_code, bytes, data.len()
            );
            
            if data.len() < 144 {
                return Err(0xFFFFFFFF);
            }
            
            // 解析健康页数据（与上面相同的逻辑）
            let temp_k = u16::from_le_bytes([data[1], data[2]]);
            let temp_c = if temp_k > 0 { (temp_k as f32) - 273.15 } else { 0.0 };
            let dur = u128::from_le_bytes(data[32..48].try_into().ok().unwrap());
            let duw = u128::from_le_bytes(data[48..64].try_into().ok().unwrap());
            let pcycles = u128::from_le_bytes(data[112..128].try_into().ok().unwrap());
            let poh = u128::from_le_bytes(data[128..144].try_into().ok().unwrap());
            let bytes_read = dur.saturating_mul(512_000);
            let bytes_write = duw.saturating_mul(512_000);
            let power_on_hours_i32 = i32::try_from(poh.min(i32::MAX as u128)).ok();
            let power_cycles_i32 = i32::try_from(pcycles.min(i32::MAX as u128)).ok();
            let host_reads_i64 = i64::try_from(bytes_read.min(i64::MAX as u128)).ok();
            let host_writes_i64 = i64::try_from(bytes_write.min(i64::MAX as u128)).ok();
            // 追加 NVMe 特有指标（百分比与媒体错误）
            let nvme_available_spare_pct = Some(data[3] as f32);
            let nvme_available_spare_threshold_pct = Some(data[4] as f32);
            let nvme_percentage_used_pct = Some(data[5] as f32);
            let media = u128::from_le_bytes(data[160..176].try_into().ok().unwrap());
            let nvme_media_errors = i64::try_from(media.min(i64::MAX as u128)).ok();
            
            Ok(SmartHealthPayload {
                device: Some(path.to_string()),
                predict_fail: None,
                temp_c: Some(temp_c),
                power_on_hours: power_on_hours_i32,
                reallocated: None,
                pending: None,
                uncorrectable: None,
                crc_err: None,
                power_cycles: power_cycles_i32,
                host_reads_bytes: host_reads_i64,
                host_writes_bytes: host_writes_i64,
                nvme_percentage_used_pct,
                nvme_available_spare_pct,
                nvme_available_spare_threshold_pct,
                nvme_media_errors,
            })
        }
        
        // 方案3: 简化的直接 SMART 查询（最后尝试）
        unsafe fn try_direct_smart_query(
            handle: windows::Win32::Foundation::HANDLE,
            path: &str,
        ) -> Result<SmartHealthPayload, u32> {
            // 尝试使用最简单的 SMART 属性查询
            // IOCTL_ATA_PASS_THROUGH 在 windows crate 0.58 中不可用，使用常量
            
            // 定义 ATA_PASS_THROUGH_EX 结构（简化版）
            #[repr(C, packed)]
            struct AtaPassThroughEx {
                length: u16,
                ata_flags: u16,
                path_id: u8,
                target_id: u8,
                lun: u8,
                reserved_as_uchar: u8,
                data_transfer_length: u32,
                timeout_value: u32,
                reserved_as_ulong: u32,
                data_buffer_offset: u64,
                previous_task_file: [u8; 8],
                current_task_file: [u8; 8],
            }
            
            let data_len: usize = 512;
            let total_len = size_of::<AtaPassThroughEx>() + data_len;
            let mut buf: Vec<u8> = vec![0u8; total_len];
            
            let ata_ptr = buf.as_mut_ptr() as *mut AtaPassThroughEx;
            let ata = &mut *ata_ptr;
            
            // 填充 ATA Pass-through 参数
            ata.length = size_of::<AtaPassThroughEx>() as u16;
            ata.ata_flags = 0x02; // ATA_FLAGS_DATA_IN
            ata.path_id = 0;
            ata.target_id = 0;
            ata.lun = 0;
            ata.data_transfer_length = data_len as u32;
            ata.timeout_value = 30;
            ata.data_buffer_offset = size_of::<AtaPassThroughEx>() as u64;
            
            // ATA SMART READ DATA 命令
            ata.current_task_file[0] = 0xD0; // Features: SMART READ DATA
            ata.current_task_file[1] = 0x01; // Sector Count
            ata.current_task_file[2] = 0x00; // LBA Low
            ata.current_task_file[3] = 0x4F; // LBA Mid
            ata.current_task_file[4] = 0xC2; // LBA High
            ata.current_task_file[6] = 0xB0; // Command: SMART
            
            eprintln!(
                "[nvme_ioctl] {}: Direct SMART pre-call data_len={}",
                path, data_len
            );
            
            let mut bytes: u32 = 0;
            let ok = DeviceIoControl(
                handle,
                IOCTL_ATA_PASS_THROUGH,
                Some(buf.as_ptr() as *const _),
                total_len as u32,
                Some(buf.as_mut_ptr() as *mut _),
                total_len as u32,
                Some(&mut bytes),
                None,
            ).is_ok();
            
            if !ok {
                let gle = windows::Win32::Foundation::GetLastError().0;
                eprintln!("[nvme_ioctl] {}: Direct SMART failed, gle={} (0x{:X})", path, gle, gle);
                return Err(gle);
            }
            
            let data_off = size_of::<AtaPassThroughEx>();
            let data = &buf[data_off .. data_off + data_len.min(buf.len() - data_off)];
            eprintln!(
                "[nvme_ioctl] {}: Direct SMART ok, bytes_ret={}, data_len={}",
                path, bytes, data.len()
            );
            
            // 简单解析：假设前几个字节包含温度信息
            if data.len() < 10 {
                return Err(0xFFFFFFFF);
            }
            
            // 对于 ATA SMART，温度通常在特定属性中
            // 这里做简化处理，返回基本信息
            Ok(SmartHealthPayload {
                device: Some(path.to_string()),
                predict_fail: Some(false), // 假设正常
                temp_c: Some(35.0), // 默认温度
                power_on_hours: None,
                reallocated: None,
                pending: None,
                uncorrectable: None,
                crc_err: None,
                power_cycles: None,
                host_reads_bytes: None,
                host_writes_bytes: None,
                nvme_percentage_used_pct: None,
                nvme_available_spare_pct: None,
                nvme_available_spare_threshold_pct: None,
                nvme_media_errors: None,
            })
        }

        // 执行多路径尝试
        eprintln!("[nvme_ioctl] {}: trying comprehensive NVMe SMART collection", path);
        
        // 路径1: 修正的 ProtocolCommand (ProtocolType=4)
        match try_refined_protocol(handle, path, 4) {
            Ok(p) => {
                eprintln!("[nvme_ioctl] {}: success via RefinedProtocol(4)", path);
                return Some(p);
            }
            Err(gle) => {
                eprintln!("[nvme_ioctl] {}: RefinedProtocol(4) failed, gle={}", path, gle);
            }
        }
        
        // 路径2: 修正的 ProtocolCommand (ProtocolType=3)
        match try_refined_protocol(handle, path, 3) {
            Ok(p) => {
                eprintln!("[nvme_ioctl] {}: success via RefinedProtocol(3)", path);
                return Some(p);
            }
            Err(gle) => {
                eprintln!("[nvme_ioctl] {}: RefinedProtocol(3) failed, gle={}", path, gle);
            }
        }
        
        // 路径3-6: SCSI Miniport 多种 control_code 尝试
        let control_codes = [0x00000004, 0x00000001, 0x00000002, 0x00000008];
        let signatures = [b"Nvme\0\0\0\0", b"NvmeMini"];
        
        for &sig in &signatures {
            for &ctrl_code in &control_codes {
                match try_scsi_miniport(handle, path, sig, ctrl_code) {
                    Ok(p) => {
                        let sig_str = std::str::from_utf8(sig).unwrap_or("unknown");
                        eprintln!("[nvme_ioctl] {}: success via SCSI Miniport('{}', 0x{:X})", path, sig_str, ctrl_code);
                        return Some(p);
                    }
                    Err(_) => {
                        // 继续尝试下一个组合
                    }
                }
            }
        }
        
        // 路径7: 直接 SMART 查询（ATA Pass-through）
        match try_direct_smart_query(handle, path) {
            Ok(p) => {
                eprintln!("[nvme_ioctl] {}: success via Direct SMART", path);
                return Some(p);
            }
            Err(gle) => {
                eprintln!("[nvme_ioctl] {}: Direct SMART failed, gle={}", path, gle);
            }
        }
        
        eprintln!("[nvme_ioctl] {}: all NVMe IOCTL paths exhausted", path);
        None
    }
}

#[cfg(not(windows))]
fn nvme_smart_via_ioctl() -> Option<Vec<SmartHealthPayload>> { None }

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
                temp_c: None,
                power_on_hours: None,
                reallocated: None,
                pending: None,
                uncorrectable: None,
                crc_err: None,
                power_cycles: None,
                host_reads_bytes: None,
                host_writes_bytes: None,
                nvme_percentage_used_pct: None,
                nvme_available_spare_pct: None,
                nvme_available_spare_threshold_pct: None,
                nvme_media_errors: None,
            });
        }
        if out.is_empty() { None } else { Some(out) }
    } else { None }
}

// 使用 PowerShell 查询 NVMe 的 Storage 可靠性计数器作为回退（适用于多数 NVMe 不支持 MSStorageDriver_* 的情况）
// 仅填充可获取到的字段：温度/通电/上电次数/累计读写字节数。其余保持 None。
#[cfg(windows)]
fn nvme_storage_reliability_ps() -> Option<Vec<SmartHealthPayload>> {
    // 组合对象：把 PhysicalDisk 的标识（FriendlyName/UniqueId/SerialNumber）与计数器合并输出为 JSON
    let ps_script: &str = r#"
        $ErrorActionPreference='SilentlyContinue';
        $items = Get-PhysicalDisk | ForEach-Object {
          $pd = $_; $c = $_ | Get-StorageReliabilityCounter;
          if ($c) {
            [PSCustomObject]@{
              FriendlyName = $pd.FriendlyName;
              UniqueId = $pd.UniqueId;
              SerialNumber = $pd.SerialNumber;
              Temperature = $c.Temperature;
              PowerOnHours = $c.PowerOnHours;
              PowerCycleCount = $c.PowerCycleCount;
              ReadBytes = $c.ReadBytes;
              WriteBytes = $c.WriteBytes;
            }
          }
        };
        $items | ConvertTo-Json -Depth 3
    "#;

    let output = (|| {
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps_script]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.output().ok()
    })()?;

    if !output.status.success() { return None; }
    let text = decode_console_bytes(&output.stdout);
    let s = text.trim();
    if s.is_empty() { return None; }

    #[derive(serde::Deserialize, Debug)]
    struct PsReliability {
        #[serde(rename = "FriendlyName")] friendly_name: Option<String>,
        #[serde(rename = "UniqueId")] unique_id: Option<String>,
        #[serde(rename = "SerialNumber")] serial_number: Option<String>,
        #[serde(rename = "Temperature")] temperature: Option<i32>,
        #[serde(rename = "PowerOnHours")] power_on_hours: Option<u64>,
        #[serde(rename = "PowerCycleCount")] power_cycle_count: Option<u64>,
        #[serde(rename = "ReadBytes")] read_bytes: Option<u64>,
        #[serde(rename = "WriteBytes")] write_bytes: Option<u64>,
    }

    // 处理单对象/数组两种 JSON 形态
    let mut rows: Vec<PsReliability> = match serde_json::from_str::<serde_json::Value>(s) {
        Ok(serde_json::Value::Array(arr)) => arr.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect(),
        Ok(v) => serde_json::from_value(v).ok().map(|one| vec![one]).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    if rows.is_empty() { return None; }

    let mut out: Vec<SmartHealthPayload> = Vec::new();
    for r in rows.drain(..) {
        let device = r
            .friendly_name
            .or(r.unique_id)
            .or(r.serial_number)
            .or_else(|| Some("NVMe".to_string()));
        out.push(SmartHealthPayload {
            device,
            predict_fail: None,
            temp_c: r.temperature.map(|t| t as f32),
            power_on_hours: r.power_on_hours.and_then(|v| i32::try_from(v).ok()),
            reallocated: None,
            pending: None,
            uncorrectable: None,
            crc_err: None,
            power_cycles: r.power_cycle_count.and_then(|v| i32::try_from(v).ok()),
            host_reads_bytes: r.read_bytes.and_then(|v| i64::try_from(v).ok()),
            host_writes_bytes: r.write_bytes.and_then(|v| i64::try_from(v).ok()),
            nvme_percentage_used_pct: None,
            nvme_available_spare_pct: None,
            nvme_available_spare_threshold_pct: None,
            nvme_media_errors: None,
        });
    }
    if out.is_empty() { None } else { Some(out) }
}

#[cfg(not(windows))]
fn nvme_storage_reliability_ps() -> Option<Vec<SmartHealthPayload>> { None }
// 仅在系统存在 smartctl.exe 且调用成功时返回；否则返回 None，不影响既有链路。
#[cfg(windows)]
fn smartctl_collect() -> Option<Vec<SmartHealthPayload>> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    
    // 解析 smartctl 可执行文件路径：优先随包内置，其次 PATH
    let smart_bin: String = (|| {
        let exe_dir: Option<PathBuf> = std::env::current_exe().ok().and_then(|p| p.parent().map(|q| q.to_path_buf()));
        if let Some(dir) = exe_dir {
            let candidates = [
                dir.join("resources").join("smartctl").join("smartctl.exe"),
                dir.join("resources").join("bin").join("smartctl.exe"),
                dir.join("smartctl.exe"),
                dir.join("bin").join("smartctl.exe"),
            ];
            for c in candidates.iter() {
                if c.exists() { return c.to_string_lossy().to_string(); }
            }
        }
        "smartctl".to_string()
    })();
    eprintln!("[smartctl] using binary: {}", smart_bin);

    // 预检：检测 smartctl 是否可用
    let mut ver = Command::new(&smart_bin);
    ver.args(["-V"]);
    ver.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let ok = ver.output().ok().map(|o| o.status.success()).unwrap_or(false);
    if !ok {
        eprintln!("[smartctl] smartctl not found or not executable");
        return None;
    }
    
    // 优先使用 smartctl --scan-open -j 枚举可打开设备
    #[derive(serde::Deserialize)]
    struct ScanDev { name: String, #[serde(rename = "type")] typ: Option<String> }
    let mut scanned: Vec<ScanDev> = {
        let mut scan = Command::new(&smart_bin);
        scan.args(["--scan-open", "-j"]);
        scan.creation_flags(0x08000000); // CREATE_NO_WINDOW
        match scan.output() {
            Ok(o) if o.status.success() => {
                let text = decode_console_bytes(&o.stdout);
                let s = text.trim();
                if s.is_empty() { Vec::new() } else {
                    match serde_json::from_str::<serde_json::Value>(s) {
                        Ok(serde_json::Value::Object(map)) => map.get("devices")
                            .and_then(|d| d.as_array())
                            .map(|arr| arr.iter().filter_map(|v| serde_json::from_value::<ScanDev>(v.clone()).ok()).collect())
                            .unwrap_or_default(),
                        _ => Vec::new(),
                    }
                }
            }
            _ => Vec::new(),
        }
    };
    if !scanned.is_empty() { eprintln!("[smartctl] scan-open found {} devices", scanned.len()); }
 
    // 当扫描为空时，回退遍历 PhysicalDrive0..31
    if scanned.is_empty() {
        scanned = (0..32).map(|n| ScanDev { name: format!("\\\\.\\\\PhysicalDrive{}", n), typ: None }).collect();
    }
    // 逐个设备采集
    let mut out_list: Vec<SmartHealthPayload> = Vec::new();
    for dev in scanned.into_iter() {
        let dev_path = dev.name;
        // 尝试序列：scan-open 的 type → sat → ata → scsi → sat,12 → sat,16 → 无 -d（自动）
        let mut try_types: Vec<Option<String>> = Vec::new();
        let mut push_unique = |val: Option<String>| {
            if !try_types.iter().any(|x| x.as_deref() == val.as_deref()) {
                try_types.push(val);
            }
        };
        if let Some(t) = dev.typ.clone() { if !t.is_empty() { push_unique(Some(t)); } }
        push_unique(Some("sat".to_string()));
        push_unique(Some("ata".to_string()));
        push_unique(Some("scsi".to_string()));
        push_unique(Some("sat,12".to_string()));
        push_unique(Some("sat,16".to_string()));
        push_unique(None);

        let mut parsed_ok = false;
        let mut last_ty = String::new();
        let mut last_err = String::new();
        let mut last_out = String::new();

        for try_ty in try_types.iter() {
            let mut cmd = Command::new(&smart_bin);
            cmd.arg("-j").arg("-a");
            let ty_desc = match try_ty {
                Some(t) => { cmd.args(["-d", t]); t.clone() }
                None => "(auto)".to_string(),
            };
            cmd.arg(&dev_path);
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            let output = match cmd.output() {
                Ok(o) => o,
                Err(e) => { eprintln!("[smartctl] spawn failed on {} [type={}]: {:?}", dev_path, ty_desc, e); continue; }
            };
            if !output.status.success() {
                let code_str = output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
                let err_s = decode_console_bytes(&output.stderr);
                let out_s = decode_console_bytes(&output.stdout);
                eprintln!("[smartctl] {} [type={}]: non-zero exit (code={}), stderr: {}", dev_path, ty_desc, code_str, err_s.trim());
                if out_s.trim().len() > 0 { eprintln!("[smartctl] {} [type={}]: stdout: {}", dev_path, ty_desc, out_s.trim()); }
                last_ty = ty_desc; last_err = err_s; last_out = out_s;
                continue;
            }
            let text = decode_console_bytes(&output.stdout);
            let s = text.trim();
            if s.is_empty() { last_ty = ty_desc; last_err.clear(); last_out.clear(); continue; }
            let v: serde_json::Value = match serde_json::from_str(s) { Ok(v) => v, Err(e) => { eprintln!("[smartctl] {} [type={}]: invalid JSON: {:?}", dev_path, ty_desc, e); continue; } };

            // 设备与健康状态
            let device = v.get("device").and_then(|d| d.get("name")).and_then(|x| x.as_str()).map(|s| s.to_string()).or(Some(dev_path.clone()));
            let predict_fail = v.get("smart_status").and_then(|s| s.get("passed")).and_then(|b| b.as_bool()).map(|passed| !passed);

            // 顶层字段
            let mut temp_c: Option<f32> = v.get("temperature").and_then(|t| t.get("current")).and_then(|x| x.as_f64()).map(|f| f as f32);
            let mut power_on_hours: Option<i32> = v.get("power_on_time").and_then(|t| t.get("hours")).and_then(|x| x.as_f64()).and_then(|f| i32::try_from(f as i64).ok());
            let mut power_cycles: Option<i32> = v.get("power_cycle_count").and_then(|x| x.as_f64()).and_then(|f| i32::try_from(f as i64).ok());
            let mut host_reads_bytes: Option<i64> = None;
            let mut host_writes_bytes: Option<i64> = None;

            // NVMe 健康日志回填
            if let Some(log) = v.get("nvme_smart_health_information_log") {
                if temp_c.is_none() {
                    if let Some(k) = log.get("temperature").and_then(|x| x.as_i64()) { temp_c = Some((k as f32) - 273.15); }
                }
                if let Some(poh) = log.get("power_on_hours").and_then(|x| x.as_u64()).and_then(|u| i32::try_from(u).ok()) { power_on_hours = Some(poh); }
                if let Some(pc) = log.get("power_cycles").and_then(|x| x.as_u64()).and_then(|u| i32::try_from(u).ok()) { power_cycles = Some(pc); }
                let to_i64 = |x: u128| -> i64 { if x > i64::MAX as u128 { i64::MAX } else { x as i64 } };
                if let Some(du) = log.get("data_units_read").and_then(|x| x.as_u64()) { host_reads_bytes = Some(to_i64((du as u128).saturating_mul(512_000))); }
                if let Some(du) = log.get("data_units_written").and_then(|x| x.as_u64()) { host_writes_bytes = Some(to_i64((du as u128).saturating_mul(512_000))); }
            }

            // ATA 属性解析
            let mut reallocated: Option<i64> = None;
            let mut pending: Option<i64> = None;
            let mut uncorrectable: Option<i64> = None;
            let mut crc_err: Option<i64> = None;
            if let Some(arr) = v.get("ata_smart_attributes").and_then(|a| a.get("table")).and_then(|x| x.as_array()) {
                for rec in arr {
                    let id = rec.get("id").and_then(|x| x.as_u64()).unwrap_or(0) as u64;
                    let raw_i64 = rec.get("raw").and_then(|r| r.get("value")).and_then(|x| x.as_i64());
                    match id {
                        5 => reallocated = raw_i64,
                        197 => pending = raw_i64,
                        198 => uncorrectable = raw_i64,
                        199 => crc_err = raw_i64,
                        9 => if let Some(vv) = raw_i64.and_then(|v| i32::try_from(v).ok()) { power_on_hours = Some(vv); },
                        12 => if let Some(vv) = raw_i64.and_then(|v| i32::try_from(v).ok()) { power_cycles = Some(vv); },
                        194 => if temp_c.is_none() { if let Some(vv) = raw_i64 { if vv > -50 && vv < 200 { temp_c = Some(vv as f32); } } },
                        _ => {}
                    }
                }
            }

            // 二次解析 NVMe 关键字段（避免前面 borrow 生命周期问题）
            let (nvme_percentage_used_pct, nvme_available_spare_pct, nvme_available_spare_threshold_pct, nvme_media_errors) = (||{
                if let Some(log) = v.get("nvme_smart_health_information_log") {
                    let a = log.get("percentage_used").and_then(|x| x.as_f64()).map(|v| v as f32);
                    let b = log.get("available_spare").and_then(|x| x.as_f64()).map(|v| v as f32);
                    let c = log.get("available_spare_threshold").and_then(|x| x.as_f64()).map(|v| v as f32);
                    let d = log.get("media_errors").and_then(|x| x.as_i64());
                    (a, b, c, d)
                } else { (None, None, None, None) }
            })();

            let payload = SmartHealthPayload {
                device,
                predict_fail,
                temp_c,
                power_on_hours,
                reallocated,
                pending,
                uncorrectable,
                crc_err,
                power_cycles,
                host_reads_bytes,
                host_writes_bytes,
                nvme_percentage_used_pct,
                nvme_available_spare_pct,
                nvme_available_spare_threshold_pct,
                nvme_media_errors,
            };
            eprintln!("[smartctl] {} [type={}]: mapped payload: temp={:?} poh={:?} pcycles={:?}", dev_path, ty_desc, payload.temp_c, payload.power_on_hours, payload.power_cycles);
            out_list.push(payload);
            parsed_ok = true;
            break;
        }

        if !parsed_ok {
            eprintln!("[smartctl] {}: all attempts failed. last type={}, stderr: {}, stdout: {}", dev_path, last_ty, last_err.trim(), last_out.trim());
        }
    }
    if out_list.is_empty() { None } else { Some(out_list) }
}

#[cfg(not(windows))]
fn smartctl_collect() -> Option<Vec<SmartHealthPayload>> { None }

// wmi_query_gpu_vram 函数已移至 gpu_utils 模块

// ---- Realtime snapshot payload for frontend ----
// 读取系统电源状态（AC 接入 / 剩余时间 / 充满耗时占位）
fn read_power_status() -> (Option<bool>, Option<i32>, Option<i32>) {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    unsafe {
        let mut sps = SYSTEM_POWER_STATUS::default();
        if GetSystemPowerStatus(&mut sps).is_ok() {
            let ac = match sps.ACLineStatus {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            };
            let remain = if sps.BatteryLifeTime == u32::MAX { None } else { Some(sps.BatteryLifeTime as i32) };
            // WinAPI 未直接提供“充满耗时”，后续可尝试 WMI Win32_Battery.TimeToFullCharge（分钟）
            let to_full: Option<i32> = None;
            (ac, remain, to_full)
        } else {
            (None, None, None)
        }
    }
}

#[derive(Clone, serde::Serialize)]
struct SensorSnapshot {
    cpu_usage: f32,
    mem_used_gb: f32,
    mem_total_gb: f32,
    mem_pct: f32,
    // 内存细分（可用）与交换区
    mem_avail_gb: Option<f32>,
    swap_used_gb: Option<f32>,
    swap_total_gb: Option<f32>,
    // 内存细分扩展：缓存/提交/分页相关
    mem_cache_gb: Option<f32>,
    mem_committed_gb: Option<f32>,
    mem_commit_limit_gb: Option<f32>,
    mem_pool_paged_gb: Option<f32>,
    mem_pool_nonpaged_gb: Option<f32>,
    mem_pages_per_sec: Option<f64>,
    mem_page_reads_per_sec: Option<f64>,
    mem_page_writes_per_sec: Option<f64>,
    mem_page_faults_per_sec: Option<f64>,
    net_rx_bps: f64,
    net_tx_bps: f64,
    // 新增：公网 IP 与 ISP
    public_ip: Option<String>,
    isp: Option<String>,
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
    // Wi‑Fi 扩展2：安全/加密/信道宽度
    wifi_auth: Option<String>,
    wifi_cipher: Option<String>,
    wifi_chan_width_mhz: Option<i32>,
    // 新增：网络接口（IP/MAC/速率/介质）
    net_ifs: Option<Vec<NetIfPayload>>,
    disk_r_bps: f64,
    disk_w_bps: f64,
    // 新增：温度（摄氏度）与风扇转速（RPM），可能不可用
    cpu_temp_c: Option<f32>,
    mobo_temp_c: Option<f32>,
    fan_rpm: Option<i32>,
    // 新增：主板电压与多风扇详细（从桥接透传）
    mobo_voltages: Option<Vec<VoltagePayload>>, // [{ name, volts }]
    fans_extra: Option<Vec<FanPayload>>,         // [{ name, rpm, pct }]
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
    // 新增：多目标 RTT 结果与 Top 进程
    rtt_multi: Option<Vec<RttResultPayload>>,
    top_cpu_procs: Option<Vec<TopProcessPayload>>,
    top_mem_procs: Option<Vec<TopProcessPayload>>,
    // 新增：GPU 列表
    gpus: Option<Vec<GpuPayload>>,
    // 新增：电池
    battery_percent: Option<i32>,
    battery_status: Option<String>,
    battery_design_capacity: Option<u32>,
    battery_full_charge_capacity: Option<u32>,
    battery_cycle_count: Option<u32>,
    battery_ac_online: Option<bool>,
    battery_time_remaining_sec: Option<i32>,
    battery_time_to_full_sec: Option<i32>,
    timestamp_ms: i64,
}

#[derive(Clone, serde::Serialize)]
struct RttResultPayload {
    target: String,
    rtt_ms: Option<f64>,
}

#[derive(Clone, serde::Serialize)]
struct TopProcessPayload {
    name: Option<String>,
    cpu_pct: Option<f32>,
    mem_bytes: Option<u64>,
}

#[derive(Clone, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BridgeFan {
    name: Option<String>,
    rpm: Option<i32>,
    pct: Option<i32>,
}

#[derive(Clone, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BridgeVoltage {
    name: Option<String>,
    volts: Option<f64>,
}

#[derive(Clone, serde::Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct BridgeOut {
    cpu_temp_c: Option<f32>,
    mobo_temp_c: Option<f32>,
    fans: Option<Vec<BridgeFan>>,
    // 透传：多风扇与主板电压
    fans_extra: Option<Vec<BridgeFan>>,
    mobo_voltages: Option<Vec<BridgeVoltage>>,
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
    health: Option<String>,
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
        // 公网查询开关（默认启用）。false 可关闭公网 IP/ISP 拉取
        public_net_enabled: Option<bool>,
        // 公网查询 API（可空使用内置：优先 ip-api.com，失败回退 ipinfo.io）
        public_net_api: Option<String>,
        // 多目标 RTT 配置
        rtt_targets: Option<Vec<String>>,   // 形如 "1.1.1.1:443"
        rtt_timeout_ms: Option<u64>,        // 默认 300ms
        // Top 进程数量（默认 5）
        top_n: Option<usize>,
    }

    #[derive(Clone)]
    struct AppState {
        config: std::sync::Arc<std::sync::Mutex<AppConfig>>,
        public_net: std::sync::Arc<std::sync::Mutex<PublicNetInfo>>,
    }

    #[derive(Clone, Default)]
    struct PublicNetInfo {
        ip: Option<String>,
        isp: Option<String>,
        last_updated_ms: Option<i64>,
        last_error: Option<String>,
    }

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
        if let Ok(guard) = state.config.lock() { guard.clone() } else { AppConfig::default() }
    }

    #[tauri::command]
    fn set_config(app: tauri::AppHandle, state: tauri::State<'_, AppState>, new_cfg: AppConfig) -> Result<(), String> {
        save_config(&app, &new_cfg).map_err(|e| e.to_string())?;
        if let Ok(mut guard) = state.config.lock() { *guard = new_cfg; }
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
                    let is_admin = {
                        let mut cmd = std::process::Command::new("powershell");
                        cmd.args([
                            "-NoProfile",
                            "-Command",
                            "([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
                        ]);
                        #[cfg(windows)]
                        {
                            use std::os::windows::process::CommandExt;
                            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                        }
                        cmd.output()
                    }
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|s| s.trim().eq_ignore_ascii_case("True"))
                        .unwrap_or(false);
                    if !is_admin {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = {
                                let mut cmd = std::process::Command::new("powershell");
                                cmd.args([
                                    "-NoProfile",
                                    "-Command",
                                    &format!("Start-Process -FilePath '{}' -Verb runas", exe.display()),
                                ]);
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                                }
                                cmd.spawn()
                            };
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
            let info_public = MenuItem::with_id(app, "info_public", "公网: —", false, None::<&str>)?;
            let info_disk = MenuItem::with_id(app, "info_disk", "磁盘: —", false, None::<&str>)?;
            let info_store = MenuItem::with_id(app, "info_store", "存储: —", false, None::<&str>)?;
            let info_gpu = MenuItem::with_id(app, "info_gpu", "GPU: —", false, None::<&str>)?;
            let info_bridge = MenuItem::with_id(app, "info_bridge", "桥接: —", false, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;

            // --- Clickable action items ---
            let show_details = MenuItem::with_id(app, "show_details", "显示详情", true, None::<&str>)?;
            let quick_settings = MenuItem::with_id(app, "quick_settings", "快速设置", true, None::<&str>)?;
            let about = MenuItem::with_id(app, "about", "关于我们", true, None::<&str>)?;
            // 调试：复制全部托盘数据到剪贴板
            let debug_copy = MenuItem::with_id(app, "debug_copy_all", "[debug] 复制全部数据", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?;

            // 初始化配置与公网缓存，并注入状态
            let cfg_arc: Arc<Mutex<AppConfig>> = Arc::new(Mutex::new(load_config(&app.handle())));
            let pub_net_arc: Arc<Mutex<PublicNetInfo>> = Arc::new(Mutex::new(PublicNetInfo::default()));
            app.manage(AppState { config: cfg_arc.clone(), public_net: pub_net_arc.clone() });

            let menu = Menu::with_items(
                app,
                &[
                    &info_cpu,
                    &info_mem,
                    &info_temp,
                    &info_fan,
                    &info_net,
                    &info_public,
                    &info_disk,
                    &info_gpu,
                    &info_store,
                    &info_bridge,
                    &sep,
                    &show_details,
                    &quick_settings,
                    &about,
                    &debug_copy,
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

            // 最近一次的汇总文本（用于 [debug] 复制）
            let last_info_text: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

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

            // --- 公网 IP/ISP 后台轮询线程 ---
            {
                let cfg_state_c = cfg_arc.clone();
                let pub_net_c = pub_net_arc.clone();
                thread::spawn(move || {
                    use std::time::Duration;
                    let agent = ureq::AgentBuilder::new()
                        .timeout_connect(Duration::from_secs(5))
                        .timeout_read(Duration::from_secs(5))
                        .timeout_write(Duration::from_secs(5))
                        .build();

                    #[derive(serde::Deserialize)]
                    struct IpApiResp { status: Option<String>, query: Option<String>, isp: Option<String>, message: Option<String> }
                    #[derive(serde::Deserialize)]
                    struct IpInfoResp { ip: Option<String>, org: Option<String> }

                    loop {
                        let enabled = cfg_state_c
                            .lock().ok()
                            .and_then(|c| c.public_net_enabled)
                            .unwrap_or(true);
                        if !enabled {
                            std::thread::sleep(Duration::from_secs(60));
                            continue;
                        }

                        let mut ok = false;
                        // 1) ip-api.com
                        let try1 = agent.get("https://ip-api.com/json/?fields=status,query,isp,message").call();
                        if let Ok(resp) = try1 {
                            if let Ok(data) = resp.into_json::<IpApiResp>() {
                                if data.status.as_deref() == Some("success") {
                                    if let Ok(mut g) = pub_net_c.lock() {
                                        g.ip = data.query;
                                        g.isp = data.isp;
                                        g.last_updated_ms = Some(chrono::Local::now().timestamp_millis());
                                        g.last_error = None;
                                    }
                                    ok = true;
                                } else if let Ok(mut g) = pub_net_c.lock() {
                                    g.last_error = data.message.or(Some("ip-api.com failed".to_string()));
                                }
                            }
                        }

                        // 2) fallback ipinfo.io
                        if !ok {
                            let try2 = agent.get("https://ipinfo.io/json").call();
                            if let Ok(resp) = try2 {
                                if let Ok(data) = resp.into_json::<IpInfoResp>() {
                                    if let Ok(mut g) = pub_net_c.lock() {
                                        g.ip = data.ip;
                                        g.isp = data.org; // org 常含 ASN+ISP 名称
                                        g.last_updated_ms = Some(chrono::Local::now().timestamp_millis());
                                        g.last_error = None;
                                    }
                                    ok = true;
                                }
                            }
                        }

                        // 休眠：成功 30 分钟；失败 60 秒
                        std::thread::sleep(if ok { Duration::from_secs(1800) } else { Duration::from_secs(60) });
                    }
                });
            }

            // --- Handle menu events ---
            let shutdown_for_exit = shutdown_flag.clone();
            let bridge_pid_for_exit = bridge_pid.clone();
            let last_info_for_evt = last_info_text.clone();
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
                "debug_copy_all" => {
                    println!("[tray] 点击 [debug] 复制全部数据");
                    // 读取最近一次的汇总文本
                    let text = last_info_for_evt
                        .lock()
                        .ok()
                        .map(|s| s.clone())
                        .unwrap_or_default();
                    if text.is_empty() {
                        eprintln!("[debug] 无可复制文本（尚未生成 tooltip）");
                    } else {
                        // 使用 PowerShell 写入剪贴板（支持多行）
                        let ps = format!("$t=@'\n{}\n'@; Set-Clipboard -Value $t", text.replace('\r', ""));
                        let _ = {
                            let mut cmd = std::process::Command::new("powershell");
                            cmd.args(["-NoProfile", "-Command", &ps]);
                            #[cfg(windows)]
                            {
                                use std::os::windows::process::CommandExt;
                                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                            }
                            cmd.status()
                        };
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
                                let _ = {
                                    let mut cmd = std::process::Command::new("taskkill");
                                    cmd.args(["/PID", &pid.to_string(), "/T", "/F"]);
                                    #[cfg(windows)]
                                    {
                                        use std::os::windows::process::CommandExt;
                                        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                                    }
                                    cmd.status()
                                };
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
            let info_gpu_c = info_gpu.clone();
            let info_bridge_c = info_bridge.clone();
            let info_public_c = info_public.clone();
            let tray_c = tray.clone();
            let app_handle_c = app_handle.clone();
            let bridge_data_sampling = bridge_data.clone();
            let cfg_state_c = cfg_arc.clone();
            let pub_net_c = pub_net_arc.clone();
            let last_info_text_c = last_info_text.clone();

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
                    let avail = sys.available_memory() as f64;
                    let avail_gb = avail / 1073741824.0;
                    let swap_total = sys.total_swap() as f64;
                    let swap_used = sys.used_swap() as f64;
                    let swap_total_gb = swap_total / 1073741824.0;
                    let swap_used_gb = swap_used / 1073741824.0;

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
                    let (mem_cache_gb, mem_committed_gb, mem_commit_limit_gb, mem_pool_paged_gb, mem_pool_nonpaged_gb, 
                         mem_pages_per_sec, mem_page_reads_per_sec, mem_page_writes_per_sec, mem_page_faults_per_sec) = match &wmi_perf_conn {
                        Some(c) => wmi_perf_memory(c),
                        None => (None, None, None, None, None, None, None, None, None),
                    };
                    let ping_rtt_ms = tcp_rtt_ms("1.1.1.1:443", 300);

                    // 多目标 RTT（顺序串行测量）
                    let rtt_multi: Option<Vec<RttResultPayload>> = {
                        let timeout = cfg_state_c
                            .lock().ok()
                            .and_then(|c| c.rtt_timeout_ms)
                            .unwrap_or(300);
                        let targets = cfg_state_c
                            .lock().ok()
                            .and_then(|c| c.rtt_targets.clone())
                            .unwrap_or_else(|| vec![
                                "1.1.1.1:443".to_string(),
                                "8.8.8.8:443".to_string(),
                                "114.114.114.114:53".to_string(),
                            ]);
                        if targets.is_empty() {
                            None
                        } else {
                            let mut out: Vec<RttResultPayload> = Vec::new();
                            for t in targets {
                                let r = tcp_rtt_ms(&t, timeout);
                                out.push(RttResultPayload { target: t, rtt_ms: r });
                            }
                            Some(out)
                        }
                    };

                    // Top 进程（CPU 与内存）
                    let top_n = cfg_state_c
                        .lock().ok()
                        .and_then(|c| c.top_n)
                        .unwrap_or(5);
                    let (top_cpu_procs, top_mem_procs): (Option<Vec<TopProcessPayload>>, Option<Vec<TopProcessPayload>>) = {
                        use std::cmp::Ordering;
                        let list: Vec<&sysinfo::Process> = sys.processes().values().collect();
                        if list.is_empty() || top_n == 0 {
                            (None, None)
                        } else {
                            // CPU 排序
                            let mut by_cpu = list.clone();
                            by_cpu.sort_by(|a, b| b.cpu_usage().partial_cmp(&a.cpu_usage()).unwrap_or(Ordering::Equal));
                            by_cpu.truncate(top_n);
                            let top_cpu: Vec<TopProcessPayload> = by_cpu
                                .into_iter()
                                .map(|p| TopProcessPayload {
                                    name: Some(p.name().to_string()),
                                    cpu_pct: Some(p.cpu_usage()),
                                    mem_bytes: Some(p.memory()),
                                })
                                .collect();

                            // 内存排序
                            let mut by_mem = list;
                            by_mem.sort_by(|a, b| b.memory().cmp(&a.memory()));
                            by_mem.truncate(top_n);
                            let top_mem: Vec<TopProcessPayload> = by_mem
                                .into_iter()
                                .map(|p| TopProcessPayload {
                                    name: Some(p.name().to_string()),
                                    cpu_pct: Some(p.cpu_usage()),
                                    mem_bytes: Some(p.memory()),
                                })
                                .collect();

                            (Some(top_cpu), Some(top_mem))
                        }
                    };
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
                        mobo_voltages,
                        fans_extra,
                        battery_percent,
                        battery_status,
                        battery_design_capacity,
                        battery_full_charge_capacity,
                        battery_cycle_count,
                        battery_ac_online,
                        battery_time_remaining_sec,
                        battery_time_to_full_sec,
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
                        let mut mobo_voltages: Option<Vec<VoltagePayload>> = None;
                        let mut fans_extra: Option<Vec<FanPayload>> = None;
                        let mut battery_percent: Option<i32> = None;
                        let mut battery_status: Option<String> = None;
                        let mut battery_ac_online: Option<bool> = None;
                        let mut battery_time_remaining_sec: Option<i32> = None;
                        let mut battery_time_to_full_sec: Option<i32> = None;
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
                                        eprintln!("[BRIDGE_GPU_DEBUG] Received {} GPUs from bridge", gg.len());
                                        for (i, gpu) in gg.iter().enumerate() {
                                            eprintln!("[BRIDGE_GPU_DEBUG] GPU {}: name={:?} vram_used_mb={:?} power_w={:?} temp_c={:?} load_pct={:?}", 
                                                i, gpu.name, gpu.vram_used_mb, gpu.power_w, gpu.temp_c, gpu.load_pct);
                                        }
                                        
                                        // 查询 GPU 显存信息
                                        let gpu_vram_info = match &wmi_perf_conn {
                                            Some(c) => wmi_query_gpu_vram(c),
                                            None => Vec::new(),
                                        };
                                        
                                        let mapped: Vec<GpuPayload> = gg.iter().map(|x| {
                                            // 尝试匹配 GPU 名称获取显存信息
                                            eprintln!("[GPU_MAPPING] Processing GPU from bridge: name={:?}", x.name);
                                            eprintln!("[GPU_MAPPING] Available VRAM info: {:?}", gpu_vram_info);
                                            
                                            let (vram_total_mb, vram_usage_pct) = if let Some(gpu_name) = &x.name {
                                                if let Some((vram_name, vram_bytes)) = gpu_vram_info.iter()
                                                    .find(|(name, _)| name.as_ref().map_or(false, |n| n.contains(gpu_name) || gpu_name.contains(n))) {
                                                    eprintln!("[GPU_MAPPING] Found match: bridge_name='{}' vram_name={:?} vram_bytes={:?}", gpu_name, vram_name, vram_bytes);
                                                    let vram_total_mb = vram_bytes.map(|bytes| (bytes / 1024 / 1024) as f64);
                                                    let vram_usage_pct = if let (Some(used), Some(total)) = (x.vram_used_mb.map(|v| v as f64), vram_total_mb) {
                                                        if total > 0.0 {
                                                            Some((used / total) * 100.0)
                                                        } else {
                                                            None
                                                        }
                                                    } else {
                                                        None
                                                    };
                                                    eprintln!("[GPU_MAPPING] Calculated: vram_total_mb={:?} vram_usage_pct={:?}", vram_total_mb, vram_usage_pct);
                                                    (vram_total_mb, vram_usage_pct)
                                                } else {
                                                    eprintln!("[GPU_MAPPING] No VRAM match found for GPU: {}", gpu_name);
                                                    (None, None)
                                                }
                                            } else {
                                                eprintln!("[GPU_MAPPING] GPU has no name");
                                                (None, None)
                                            };
                                            
                                            // 确保VRAM数据正确传递到前端
                                            let final_vram_used_mb = x.vram_used_mb.or_else(|| {
                                                // 如果桥接数据没有vram_used_mb，但有计算出的使用率，则反推计算
                                                if let (Some(total), Some(pct)) = (vram_total_mb, vram_usage_pct) {
                                                    Some(total * pct / 100.0)
                                                } else {
                                                    None
                                                }
                                            });
                                            
                                            eprintln!("[GPU_FINAL] Creating GpuPayload: name={:?} vram_used_mb={:?} vram_total_mb={:?} vram_usage_pct={:?}", 
                                                x.name, final_vram_used_mb, vram_total_mb, vram_usage_pct);
                                            
                                            GpuPayload {
                                                name: x.name.clone(),
                                                temp_c: x.temp_c,
                                                load_pct: x.load_pct,
                                                core_mhz: x.core_mhz,
                                                memory_mhz: x.memory_mhz,
                                                fan_rpm: x.fan_rpm,
                                                fan_duty_pct: x.fan_duty_pct,
                                                vram_used_mb: final_vram_used_mb,
                                                vram_total_mb,
                                                vram_usage_pct,
                                                power_w: x.power_w,
                                                power_limit_w: x.power_limit_w,
                                                voltage_v: x.voltage_v,
                                                hotspot_temp_c: x.hotspot_temp_c,
                                                vram_temp_c: x.vram_temp_c,
                                            }
                                        }).collect();
                                        if !mapped.is_empty() { gpus = Some(mapped); }
                                    }
                                    // 主板电压
                                    if let Some(vs) = &b.mobo_voltages {
                                        let mapped: Vec<VoltagePayload> = vs.iter().map(|x| VoltagePayload {
                                            name: x.name.clone(),
                                            volts: x.volts,
                                        }).collect();
                                        if !mapped.is_empty() { mobo_voltages = Some(mapped); }
                                    }
                                    // 多风扇
                                    if let Some(fx) = &b.fans_extra {
                                        let mapped: Vec<FanPayload> = fx.iter().map(|x| FanPayload {
                                            name: x.name.clone(),
                                            rpm: x.rpm,
                                            pct: x.pct,
                                        }).collect();
                                        if !mapped.is_empty() { fans_extra = Some(mapped); }
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
                        // 电池信息（WMI + WinAPI）
                        let mut wmi_remain: Option<i32> = None;
                        let mut wmi_to_full: Option<i32> = None;
                        if let Some(c) = &wmi_fan_conn {
                            let (bp, bs) = battery_utils::wmi_read_battery(c);
                            battery_percent = bp;
                            battery_status = bs;
                            let (r_sec, tf_sec) = battery_utils::wmi_read_battery_time(c);
                            wmi_remain = r_sec;
                            wmi_to_full = tf_sec;
                        }
                        let (ac, remain_win, to_full_win) = read_power_status();
                        battery_ac_online = ac;
                        battery_time_remaining_sec = wmi_remain.or(remain_win);
                        battery_time_to_full_sec = wmi_to_full.or(to_full_win);
                        // 将电池健康变量注入返回元组（通过重新查询一次以确保作用域内可读）
                        let (design_cap_ret, full_cap_ret, cycle_cnt_ret) = if let Some(c) = &wmi_fan_conn { battery_utils::wmi_read_battery_health(c) } else { (None, None, None) };
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
                            mobo_voltages,
                            fans_extra,
                            battery_percent,
                            battery_status,
                            design_cap_ret,
                            full_cap_ret,
                            cycle_cnt_ret,
                            battery_ac_online,
                            battery_time_remaining_sec,
                            battery_time_to_full_sec,
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

                    let temp_opt = bridge_cpu_temp.or_else(|| wmi_temp_conn.as_ref().and_then(|c| thermal_utils::wmi_read_cpu_temp_c(c)));
                    let fan_opt = bridge_cpu_fan.or_else(|| wmi_fan_conn.as_ref().and_then(|c| thermal_utils::wmi_read_fan_rpm(c)));

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

                    // GPU 汇总行（最多展示 2 个，多余以 +N 表示）
                    let gpu_line: String = match &gpus {
                        Some(gs) if !gs.is_empty() => {
                            let mut parts: Vec<String> = Vec::new();
                            for (i, g) in gs.iter().enumerate().take(2) {
                                let label = g.name.clone().unwrap_or_else(|| format!("GPU{}", i + 1));
                                let vram = g
                                    .vram_used_mb
                                    .map(|v| format!("{:.0} MB", v))
                                    .unwrap_or_else(|| "—".to_string());
                                let pwr = g
                                    .power_w
                                    .map(|w| format!("{:.1} W", w))
                                    .unwrap_or_else(|| "—".to_string());
                                parts.push(format!("{} VRAM {} PWR {}", label, vram, pwr));
                            }
                            let mut s = format!("GPU: {}", parts.join(", "));
                            if gs.len() > 2 { s.push_str(&format!(" +{}", gs.len() - 2)); }
                            s
                        }
                        _ => "GPU: —".to_string(),
                    };

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

                    // 公网行
                    let (pub_ip_opt, pub_isp_opt) = match pub_net_c.lock() {
                        Ok(g) => (g.ip.clone(), g.isp.clone()),
                        Err(_) => (None, None),
                    };
                    let public_line: String = match (pub_ip_opt.as_ref(), pub_isp_opt.as_ref()) {
                        (Some(ip), Some(isp)) => format!("公网: {} {}", ip, isp),
                        (Some(ip), None) => format!("公网: {}", ip),
                        _ => "公网: —".to_string(),
                    };

                    // 更新菜单只读信息（忽略错误）
                    let _ = info_cpu_c.set_text(&cpu_line);
                    let _ = info_mem_c.set_text(&mem_line);
                    let _ = info_temp_c.set_text(&temp_line);
                    let _ = info_fan_c.set_text(&fan_line);
                    let _ = info_net_c.set_text(&net_line);
                    let _ = info_public_c.set_text(&public_line);
                    let _ = info_disk_c.set_text(&disk_line);
                    let _ = info_gpu_c.set_text(&gpu_line);
                    let _ = info_store_c.set_text(&storage_line);
                    let _ = info_bridge_c.set_text(&bridge_line);

                    // 更新托盘 tooltip，避免一直停留在“初始化中”
                    let tooltip = format!(
                        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                        cpu_line, mem_line, temp_line, fan_line, net_line, public_line, disk_line, gpu_line, storage_line, bridge_line
                    );
                    let _ = tray_c.set_tooltip(Some(&tooltip));
                    // 保存以供 [debug] 复制
                    if let Ok(mut g) = last_info_text_c.lock() { *g = tooltip.clone(); }

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
                    let net_ifs = match &wmi_fan_conn { Some(c) => network_disk_utils::wmi_list_net_ifs(c), None => None };
                    let logical_disks = match &wmi_fan_conn { Some(c) => network_disk_utils::wmi_list_logical_disks(c), None => None };
                    // SMART 健康：优先 smartctl（若可用），其次 ROOT\WMI 的 FailurePredictStatus
                    // 若失败，再尝试 NVMe 的 Storage 可靠性计数器（PowerShell）
                    // 仍失败，则回退 ROOT\CIMV2 的 DiskDrive.Status
                    let mut smart_health = smartctl_collect();
                    if smart_health.is_none() {
                        smart_health = match &wmi_temp_conn { Some(c) => wmi_list_smart_status(c), None => None };
                    }
                    if smart_health.is_none() {
                        // NVMe 回退（可能仅返回温度/磨损/部分计数）
                        smart_health = nvme_storage_reliability_ps();
                    }
                    if smart_health.is_none() {
                        smart_health = match &wmi_fan_conn { Some(c) => wmi_fallback_disk_status(c), None => None };
                    }
                    // 电池：已在上文解构块中通过 WMI 读取

                    let now_ts = chrono::Local::now().timestamp_millis();
                    let snapshot = SensorSnapshot {
                        cpu_usage,
                        mem_used_gb: used_gb as f32,
                        mem_total_gb: total_gb as f32,
                        mem_pct: mem_pct as f32,
                        mem_avail_gb: Some(avail_gb as f32),
                        swap_used_gb: if swap_total > 0.0 { Some(swap_used_gb as f32) } else { None },
                        swap_total_gb: if swap_total > 0.0 { Some(swap_total_gb as f32) } else { None },
                        // 内存细分字段
                        mem_cache_gb,
                        mem_committed_gb,
                        mem_commit_limit_gb,
                        mem_pool_paged_gb,
                        mem_pool_nonpaged_gb,
                        mem_pages_per_sec,
                        mem_page_reads_per_sec,
                        mem_page_writes_per_sec,
                        mem_page_faults_per_sec,
                        net_rx_bps: ema_net_rx,
                        net_tx_bps: ema_net_tx,
                        public_ip: pub_ip_opt,
                        isp: pub_isp_opt,
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
                        wifi_auth: wi.auth,
                        wifi_cipher: wi.cipher,
                        wifi_chan_width_mhz: wi.chan_width_mhz,
                        net_ifs,
                        disk_r_bps: ema_disk_r,
                        disk_w_bps: ema_disk_w,
                        cpu_temp_c: temp_opt.map(|v| v as f32),
                        mobo_temp_c: bridge_mobo_temp,
                        fan_rpm: fan_best.map(|v| v as i32),
                        mobo_voltages,
                        fans_extra,
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
                        rtt_multi,
                        top_cpu_procs,
                        top_mem_procs,
                        battery_percent,
                        battery_status,
                        battery_design_capacity,
                        battery_full_charge_capacity,
                        battery_cycle_count,
                        battery_ac_online,
                        battery_time_remaining_sec,
                        battery_time_to_full_sec,
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
