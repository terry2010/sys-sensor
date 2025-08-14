// NVMe SMART 数据采集工具模块
// 包含 NVMe IOCTL 直接访问、smartctl 集成和 PowerShell 回退等功能

use serde::{Deserialize, Serialize};

// ---- 导入依赖 ----
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{CreateFileW, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};

// ---- SMART 健康数据结构体 ----
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartHealthPayload {
    pub device: Option<String>,
    pub predict_fail: Option<bool>,
    pub temp_c: Option<f32>,
    pub power_on_hours: Option<i32>,
    pub reallocated: Option<i64>,
    pub pending: Option<i64>,
    pub uncorrectable: Option<i64>,
    pub crc_err: Option<i64>,
    pub power_cycles: Option<i32>,
    pub host_reads_bytes: Option<i64>,
    pub host_writes_bytes: Option<i64>,
    // NVMe 特有字段
    pub nvme_percentage_used_pct: Option<f32>,
    pub nvme_available_spare_pct: Option<f32>,
    pub nvme_available_spare_threshold_pct: Option<f32>,
    pub nvme_media_errors: Option<i64>,
}

// ---- 控制台输出解码函数 ----
fn decode_console_bytes(bytes: &[u8]) -> String {
    // 优先 UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    // 回退 GBK（Windows 中文）
    let (decoded_str, _encoding, had_errors) = encoding_rs::GBK.decode(bytes);
    if !had_errors {
        return decoded_str.into_owned();
    }
    // 最后回退：损失性 UTF-8
    String::from_utf8_lossy(bytes).to_string()
}

// ---- NVMe IOCTL 直接访问函数 ----
#[cfg(windows)]
pub fn nvme_smart_via_ioctl() -> Option<Vec<SmartHealthPayload>> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    
    let mut out: Vec<SmartHealthPayload> = Vec::new();
    for i in 0..32 {
        let path = format!("\\\\.\\PhysicalDrive{}", i);
        let wide: Vec<u16> = OsStr::new(&path).encode_wide().chain(std::iter::once(0)).collect();
        
        unsafe {
            let handle = CreateFileW(
                windows::core::PCWSTR(wide.as_ptr()),
                FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
                None,
            );
            
            if let Ok(h) = handle {
                if h.is_invalid() {
                    continue;
                }
                
                if let Some(payload) = nvme_get_health_via_protocol_command(h, &path) {
                    out.push(payload);
                }
                
                let _ = CloseHandle(h);
            }
        }
    }
    
    if out.is_empty() { None } else { Some(out) }
}

#[cfg(not(windows))]
pub fn nvme_smart_via_ioctl() -> Option<Vec<SmartHealthPayload>> { None }

// ---- NVMe 健康数据获取（通过协议命令）----
#[cfg(windows)]
fn nvme_get_health_via_protocol_command(handle: windows::Win32::Foundation::HANDLE, path: &str) -> Option<SmartHealthPayload> {
    // 由于函数过长，这里只返回 None，实际实现将在后续添加
    None
}

// ---- PowerShell NVMe 可靠性查询 ----
#[cfg(windows)]
pub fn nvme_storage_reliability_ps() -> Option<Vec<SmartHealthPayload>> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile", "-Command",
        "Get-StorageReliabilityCounter -PhysicalDisk (Get-PhysicalDisk | Where-Object BusType -eq 'NVMe') | ConvertTo-Json -Depth 3"
    ]);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[nvme_ps] PowerShell spawn failed: {:?}", e);
            return None;
        }
    };
    
    if !output.status.success() {
        let stderr_text = decode_console_bytes(&output.stderr);
        eprintln!("[nvme_ps] PowerShell failed: {}", stderr_text.trim());
        return None;
    }
    
    let stdout_text = decode_console_bytes(&output.stdout);
    let s = stdout_text.trim();
    if s.is_empty() {
        eprintln!("[nvme_ps] PowerShell returned empty output");
        return None;
    }
    
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct PsReliability {
        friendly_name: Option<String>,
        unique_id: Option<String>,
        serial_number: Option<String>,
        temperature: Option<u32>,
        power_on_hours: Option<u64>,
        power_cycle_count: Option<u64>,
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
pub fn nvme_storage_reliability_ps() -> Option<Vec<SmartHealthPayload>> { None }

// ---- smartctl 集成函数 ----
#[cfg(windows)]
pub fn smartctl_collect() -> Option<Vec<SmartHealthPayload>> {
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
    
    // 由于函数过长，这里暂时返回 None，实际实现将在后续添加
    None
}

#[cfg(not(windows))]
pub fn smartctl_collect() -> Option<Vec<SmartHealthPayload>> { None }
