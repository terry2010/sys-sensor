// ================================================================================
// SMART状态查询工具模块
// ================================================================================

use wmi::WMIConnection;
use crate::nvme_smart_utils::nvme_smart_via_ioctl;
use crate::types::SmartHealthPayload;
use std::collections::HashMap;

// ---- SMART相关结构体 ----

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "MSStorageDriver_FailurePredictStatus")]
pub struct MsStorageDriverFailurePredictStatus {
    #[serde(rename = "InstanceName")]
    pub instance_name: Option<String>,
    #[serde(rename = "PredictFailure")]
    pub predict_failure: Option<bool>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "MSStorageDriver_FailurePredictData")]
pub struct MsStorageDriverFailurePredictData {
    #[serde(rename = "InstanceName")]
    pub instance_name: Option<String>,
    #[serde(rename = "VendorSpecific")]
    pub vendor_specific: Option<Vec<u8>>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_DiskDrive")]
pub struct Win32DiskDrive {
    #[serde(rename = "Model")]
    pub model: Option<String>,
    #[serde(rename = "Status")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct SmartAttrRec {
    pub id: u8,
    pub value: u8,
    pub worst: u8,
    pub raw: u64,
}

// ---- 磁盘设备到盘符映射 ----

// 查询磁盘分区信息，建立物理磁盘到盘符的映射
fn get_disk_drive_letter_mapping(conn: &WMIConnection) -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    
    // 查询 Win32_LogicalDiskToPartition 关联
    if let Ok(results) = conn.raw_query::<serde_json::Value>(
        "SELECT * FROM Win32_LogicalDiskToPartition"
    ) {
        for item in results {
            if let (Some(antecedent), Some(dependent)) = (
                item.get("Antecedent").and_then(|v| v.as_str()),
                item.get("Dependent").and_then(|v| v.as_str())
            ) {
                // 从 Dependent 中提取盘符 (如 "Win32_LogicalDisk.DeviceID=\"C:\"") 
                if let Some(drive_letter) = extract_drive_letter_from_wmi_path(dependent) {
                    // 从 Antecedent 中提取磁盘和分区信息
                    if let Some(disk_info) = extract_disk_info_from_wmi_path(antecedent) {
                        mapping.insert(disk_info, drive_letter);
                    }
                }
            }
        }
    }
    
    // 查询 Win32_DiskDriveToDiskPartition 关联
    if let Ok(results) = conn.raw_query::<serde_json::Value>(
        "SELECT * FROM Win32_DiskDriveToDiskPartition"
    ) {
        for item in results {
            if let (Some(antecedent), Some(dependent)) = (
                item.get("Antecedent").and_then(|v| v.as_str()),
                item.get("Dependent").and_then(|v| v.as_str())
            ) {
                // 从 Antecedent 中提取物理磁盘信息
                if let Some(physical_disk) = extract_physical_disk_from_wmi_path(antecedent) {
                    // 从 Dependent 中提取分区信息
                    if let Some(partition_info) = extract_disk_info_from_wmi_path(dependent) {
                        // 如果已经有这个分区的盘符映射，则添加物理磁盘映射
                        if let Some(drive_letter) = mapping.get(&partition_info) {
                            mapping.insert(physical_disk, drive_letter.clone());
                        }
                    }
                }
            }
        }
    }
    
    mapping
}

// 从 WMI 路径中提取盘符
fn extract_drive_letter_from_wmi_path(path: &str) -> Option<String> {
    // 匹配 "Win32_LogicalDisk.DeviceID=\"C:\""
    if let Some(start) = path.find("DeviceID=\"") {
        let start = start + 10; // "DeviceID=\"".len()
        if let Some(end) = path[start..].find('\"') {
            return Some(path[start..start+end].to_string());
        }
    }
    None
}

// 从 WMI 路径中提取磁盘信息
fn extract_disk_info_from_wmi_path(path: &str) -> Option<String> {
    // 匹配 "Win32_DiskPartition.DeviceID=\"Disk #0, Partition #0\""
    if let Some(start) = path.find("DeviceID=\"") {
        let start = start + 10;
        if let Some(end) = path[start..].find('\"') {
            return Some(path[start..start+end].to_string());
        }
    }
    None
}

// 从 WMI 路径中提取物理磁盘信息
fn extract_physical_disk_from_wmi_path(path: &str) -> Option<String> {
    // 匹配 "Win32_DiskDrive.DeviceID=\"\\\\.\\PHYSICALDRIVE0\""
    if let Some(start) = path.find("DeviceID=\"") {
        let start = start + 10;
        if let Some(end) = path[start..].find('\"') {
            let device_id = &path[start..start+end];
            // 提取 PHYSICALDRIVE 编号
            if let Some(drive_start) = device_id.rfind("PHYSICALDRIVE") {
                return Some(device_id[drive_start..].to_string());
            }
        }
    }
    None
}

// 根据设备名查找对应的盘符
fn find_drive_letter_for_device(device: &str, mapping: &HashMap<String, String>) -> Option<String> {
    // 直接匹配
    if let Some(letter) = mapping.get(device) {
        return Some(letter.clone());
    }
    
    // 模糊匹配：查找包含设备名关键信息的映射
    for (key, value) in mapping {
        if device.contains(key) || key.contains(device) {
            return Some(value.clone());
        }
        // 特殊处理 PHYSICALDRIVE 格式
        if device.contains("PHYSICALDRIVE") && key.contains("PHYSICALDRIVE") {
            if let (Some(dev_num), Some(key_num)) = (
                device.chars().last().and_then(|c| c.to_digit(10)),
                key.chars().last().and_then(|c| c.to_digit(10))
            ) {
                if dev_num == key_num {
                    return Some(value.clone());
                }
            }
        }
    }
    
    None
}

// ---- SMART查询函数 ----

pub fn parse_smart_vendor(v: &[u8]) -> std::collections::HashMap<u8, SmartAttrRec> {
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

pub fn wmi_list_smart_status(conn: &WMIConnection) -> Option<Vec<SmartHealthPayload>> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, SmartHealthPayload> = BTreeMap::new();
    
    eprintln!("[wmi_list_smart_status] Starting SMART data collection...");
    
    // 获取磁盘设备到盘符的映射
    let drive_mapping = get_disk_drive_letter_mapping(conn);
    eprintln!("[wmi_list_smart_status] Drive letter mapping: {:?}", drive_mapping);
    
    // 优先尝试：Windows 原生 NVMe/ATA IOCTL（自研采集）
    if let Some(nvme_data) = nvme_smart_via_ioctl() {
        eprintln!("[wmi_list_smart_status] IOCTL NVMe returned {} devices", nvme_data.len());
        // 将 nvme_smart_utils::SmartHealthPayload 转换为 types::SmartHealthPayload
        let converted_data: Vec<SmartHealthPayload> = nvme_data.into_iter().map(|item| {
            let drive_letter = item.device.as_ref()
                .and_then(|dev| find_drive_letter_for_device(dev, &drive_mapping));
            SmartHealthPayload {
                device: item.device,
                drive_letter,
                predict_fail: item.predict_fail,
                temp_c: item.temp_c,
                power_on_hours: item.power_on_hours,
                reallocated: item.reallocated,
                pending: item.pending,
                uncorrectable: item.uncorrectable,
                crc_err: item.crc_err,
                power_cycles: item.power_cycles,
                host_reads_bytes: item.host_reads_bytes,
                host_writes_bytes: item.host_writes_bytes,
                nvme_percentage_used_pct: item.nvme_percentage_used_pct,
                nvme_available_spare_pct: item.nvme_available_spare_pct,
                nvme_available_spare_threshold_pct: item.nvme_available_spare_threshold_pct,
                nvme_media_errors: item.nvme_media_errors,
            }
        }).collect();
        return Some(converted_data);
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
                    let instance_name = it.instance_name.clone().unwrap_or_default();
                    let key = instance_name.clone();
                    eprintln!("[wmi_list_smart_status] Predict status: device={:?} predict_fail={:?}", instance_name, it.predict_failure);
                    let entry = map.entry(key).or_insert(SmartHealthPayload {
                        device: Some(instance_name.clone()),
                        drive_letter: find_drive_letter_for_device(&instance_name, &drive_mapping),
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
                        drive_letter: None, // WMI 数据暂不包含盘符信息
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
        if let Some(sc) = crate::smartctl_collect() {
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
        if let Some(nvme_data) = crate::powershell_utils::nvme_storage_reliability_ps() {
            eprintln!("[wmi_list_smart_status] PowerShell NVMe successful, found {} devices", nvme_data.len());
            return Some(nvme_data);
        }
        
        eprintln!("[wmi_list_smart_status] All SMART data collection methods failed");
    }

    if map.is_empty() { None } else { Some(map.into_values().collect()) }
}

pub fn wmi_fallback_disk_status(conn: &WMIConnection) -> Option<Vec<SmartHealthPayload>> {
    let res: Result<Vec<Win32DiskDrive>, _> = conn.query();
    if let Ok(list) = res {
        let mut out: Vec<SmartHealthPayload> = Vec::new();
        for disk in list.into_iter() {
            let predict_fail = match disk.status.as_deref() {
                Some("OK") => Some(false),
                Some("Degraded") | Some("Error") => Some(true),
                _ => None,
            };
            out.push(SmartHealthPayload {
                device: disk.model,
                drive_letter: None, // WMI 回退数据暂不包含盘符信息
                predict_fail,
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
    } else {
        None
    }
}
