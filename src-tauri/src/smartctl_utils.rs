// ================================================================================
// smartctl 查询工具模块
// ================================================================================
// 
// 本模块包含使用 smartctl 工具查询硬盘 SMART 信息的功能：
// 1. smartctl 可执行文件路径解析
// 2. 设备扫描和枚举
// 3. SMART 数据采集和解析
// 4. NVMe 和 ATA 设备支持
//
// ================================================================================

use crate::types::SmartHealthPayload;
use crate::wmi_utils::decode_console_bytes;
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn extract_drive_letter(device_path: &str) -> Option<String> {
    // 从 \\.\PhysicalDrive0 这样的路径提取磁盘编号
    if let Some(caps) = regex::Regex::new(r"\\\\\.\\PhysicalDrive(\d+)")
        .ok()?
        .captures(device_path) 
    {
        if let Some(disk_num) = caps.get(1) {
            let disk_index = disk_num.as_str();
            
            // 使用 PowerShell 查询该物理磁盘对应的盘符
            let ps_cmd = format!(
                "Get-WmiObject -Class Win32_LogicalDiskToPartition | Where-Object {{ $_.Antecedent -like '*Disk #{},*' }} | ForEach-Object {{ Get-WmiObject -Class Win32_LogicalDisk -Filter \"DeviceID='$($_.Dependent.Split('=')[1].Trim('\"'))\" }} | Select-Object -ExpandProperty DeviceID",
                disk_index
            );
            
            if let Ok(output) = Command::new("powershell")
                .args(&["-Command", &ps_cmd])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output() 
            {
                let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !result.is_empty() && result.len() >= 2 {
                    // 返回第一个找到的盘符，如 "C:"
                    let drive_letter = result.lines().next()?.trim();
                    if drive_letter.ends_with(':') {
                        return Some(drive_letter.to_string());
                    }
                }
            }
            
            // 如果 PowerShell 查询失败，返回磁盘编号作为标识
            return Some(format!("磁盘{}", disk_index));
        }
    }
    
    // 如果是其他格式的设备路径，尝试直接提取
    if device_path.contains("C:") { return Some("C:".to_string()); }
    if device_path.contains("D:") { return Some("D:".to_string()); }
    if device_path.contains("E:") { return Some("E:".to_string()); }
    
    None
}

// 仅在系统存在 smartctl.exe 且调用成功时返回；否则返回 None，不影响既有链路。
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

            // 尝试从设备路径提取盘符信息
            let drive_letter = extract_drive_letter(&dev_path);
            
            let payload = SmartHealthPayload {
                device,
                drive_letter,
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
pub fn smartctl_collect() -> Option<Vec<SmartHealthPayload>> { None }
