// ================================================================================
// SMART状态查询工具模块
// ================================================================================

use serde::{Deserialize, Serialize};
use wmi::WMIConnection;
use crate::nvme_smart_utils::nvme_smart_via_ioctl;
use crate::types::SmartHealthPayload;

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
    
    // 优先尝试：Windows 原生 NVMe/ATA IOCTL（自研采集）
    if let Some(nvme_data) = nvme_smart_via_ioctl() {
        eprintln!("[wmi_list_smart_status] IOCTL NVMe returned {} devices", nvme_data.len());
        // 将 nvme_smart_utils::SmartHealthPayload 转换为 types::SmartHealthPayload
        let converted_data: Vec<SmartHealthPayload> = nvme_data.into_iter().map(|item| {
            SmartHealthPayload {
                device: item.device,
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
