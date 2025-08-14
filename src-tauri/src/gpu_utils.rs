// ================================================================================
// GPU查询工具模块
// ================================================================================

use serde::{Deserialize, Serialize};
use wmi::WMIConnection;
use crate::GpuPayload;

// ---- GPU相关结构体 ----

#[derive(Clone, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BridgeGpu {
    pub name: Option<String>,
    pub temp_c: Option<f32>,
    pub load_pct: Option<f32>,
    pub core_mhz: Option<f64>,
    pub memory_mhz: Option<f64>,
    pub fan_rpm: Option<i32>,
    pub fan_duty_pct: Option<i32>,
    pub vram_used_mb: Option<f64>,
    pub vram_total_mb: Option<f64>,
    pub power_w: Option<f64>,
    pub power_limit_w: Option<f64>,
    pub voltage_v: Option<f64>,
    pub hotspot_temp_c: Option<f32>,
    pub vram_temp_c: Option<f32>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Win32VideoController {
    pub name: Option<String>,
    #[serde(rename = "AdapterRAM")]
    pub adapter_ram: Option<u64>,
    pub driver_version: Option<String>,
    pub video_processor: Option<String>,
}

// ---- GPU查询函数 ----

/// GPU 显存查询函数 - 返回GpuPayload格式
pub fn wmi_read_gpu_vram(conn: &wmi::WMIConnection) -> Option<Vec<GpuPayload>> {
    let res: Result<Vec<Win32VideoController>, _> = conn.query();
    if let Ok(list) = res {
        let mut gpus = Vec::new();
        for gpu in list {
            if let Some(name) = gpu.name {
                let _vram_total_mb = gpu.adapter_ram.map(|ram| (ram as f64 / 1048576.0) as f32);
                gpus.push(GpuPayload {
                    name: Some(name),
                    temp_c: None,
                    load_pct: None,
                    core_mhz: None,
                    memory_mhz: None,
                    fan_rpm: None,
                    fan_duty_pct: None,
                    vram_used_mb: None,
                    vram_total_mb: _vram_total_mb.map(|v| v as f64),
                    vram_usage_pct: None,
                    power_w: None,
                    power_limit_w: None,
                    voltage_v: None,
                    hotspot_temp_c: None,
                    vram_temp_c: None,
                });
            }
        }
        if gpus.is_empty() { None } else { Some(gpus) }
    } else {
        None
    }
}

/// GPU 显存查询函数 - 返回简化的元组格式
pub fn wmi_query_gpu_vram(conn: &wmi::WMIConnection) -> Vec<(Option<String>, Option<u64>)> {
    let mut gpu_vram = Vec::new();
    
    // 尝试多种GPU WMI类名
    let class_attempts = [
        ("Win32_VideoController", true),
        ("Win32_DisplayConfiguration", false),
        ("Win32_SystemEnclosure", false),
    ];
    
    for (class_name, _is_primary) in &class_attempts {
        eprintln!("[wmi_query_gpu_vram] Trying class: {}", class_name);
        
        if *class_name == "Win32_VideoController" {
            match conn.query::<Win32VideoController>() {
                Ok(results) => {
                    eprintln!("[wmi_query_gpu_vram] {} found {} GPU entries", class_name, results.len());
                    for gpu in results {
                        let name = gpu.name.clone();
                        let vram_bytes = gpu.adapter_ram;
                        eprintln!("[wmi_query_gpu_vram] GPU: name={:?} vram_bytes={:?}", name, vram_bytes);
                        gpu_vram.push((name, vram_bytes));
                    }
                    if !gpu_vram.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("[wmi_query_gpu_vram] {} query failed: {:?}", class_name, e);
                    // 暂时跳过原始SQL查询，直接使用回退机制
                }
            }
        } else {
            // 其他类的回退查询（如果需要的话）
            eprintln!("[wmi_query_gpu_vram] {} not implemented yet", class_name);
        }
    }
    
    // 如果所有查询都失败，尝试从注册表或其他方式获取GPU信息
    if gpu_vram.is_empty() {
        eprintln!("[wmi_query_gpu_vram] All WMI queries failed, trying registry fallback");
        
        // 尝试从注册表读取GPU信息（Windows常见路径）
        use std::process::Command;
        #[cfg(windows)]
        use std::os::windows::process::CommandExt;
        let output = Command::new("wmic")
            .args(&["path", "win32_VideoController", "get", "name,AdapterRAM", "/format:csv"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
            
        if let Ok(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            eprintln!("[wmi_query_gpu_vram] WMIC output: {}", output_str);
            
            for line in output_str.lines().skip(1) { // 跳过标题行
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 3 {
                    let node = parts[0].trim();
                    let adapter_ram = parts[1].trim();
                    let name = parts[2].trim();
                    
                    eprintln!("[wmi_query_gpu_vram] Parsing line: node='{}' ram='{}' name='{}'", node, adapter_ram, name);
                    
                    if !name.is_empty() && !adapter_ram.is_empty() && adapter_ram != "AdapterRAM" && name != "Name" {
                        if let Ok(ram_bytes) = adapter_ram.parse::<u64>() {
                            if ram_bytes > 0 {
                                eprintln!("[wmi_query_gpu_vram] WMIC found GPU: {} with {}MB VRAM", name, ram_bytes / 1024 / 1024);
                                gpu_vram.push((Some(name.to_string()), Some(ram_bytes)));
                            }
                        } else {
                            eprintln!("[wmi_query_gpu_vram] Failed to parse AdapterRAM: '{}'", adapter_ram);
                        }
                    }
                }
            }
        } else {
            eprintln!("[wmi_query_gpu_vram] WMIC command also failed");
        }
        
        // 如果WMIC也失败，提供一个基本的回退
        if gpu_vram.is_empty() {
            eprintln!("[wmi_query_gpu_vram] All methods failed, using basic fallback");
            // 提供一个通用的GPU条目，显存大小设为None，让前端显示"—"
            gpu_vram.push((Some("GPU".to_string()), None));
        }
    }
    
    gpu_vram
}
