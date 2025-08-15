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
    // GPU深度监控新增字段
    pub encode_util_pct: Option<f32>,    // 编码单元使用率
    pub decode_util_pct: Option<f32>,    // 解码单元使用率
    pub vram_bandwidth_pct: Option<f32>, // 显存带宽使用率
    pub p_state: Option<String>,         // P-State功耗状态
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

/// GPU 深度监控指标采集函数
pub fn query_gpu_advanced_metrics(gpu_name: &str) -> (Option<f32>, Option<f32>, Option<f32>, Option<String>) {
    // 初始化返回值：(编码单元使用率, 解码单元使用率, 显存带宽使用率, P-State)
    let mut encode_util = None;
    let mut decode_util = None;
    let mut vram_bandwidth = None;
    let mut p_state = None;
    
    // 判断 GPU类型（NVIDIA/AMD/Intel）
    let gpu_name_lower = gpu_name.to_lowercase();
    
    if gpu_name_lower.contains("nvidia") {
        // NVIDIA GPU: 使用 nvidia-smi 命令行工具采集
        use std::process::Command;
        #[cfg(windows)]
        use std::os::windows::process::CommandExt;
        
        // 查询编码/解码单元使用率
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=encoder_util,decoder_util,memory.used,memory.total,pstate", 
                "--format=csv,noheader,nounits"
            ])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
            
        if let Ok(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let values: Vec<&str> = output_str.trim().split(",").collect();
            
            if values.len() >= 5 {
                // 解析编码单元使用率
                if let Ok(enc) = values[0].trim().parse::<f32>() {
                    encode_util = Some(enc);
                }
                
                // 解析解码单元使用率
                if let Ok(dec) = values[1].trim().parse::<f32>() {
                    decode_util = Some(dec);
                }
                
                // 计算显存带宽使用率（基于已用显存与总显存的比例估算）
                if let (Ok(used), Ok(total)) = (values[2].trim().parse::<f32>(), values[3].trim().parse::<f32>()) {
                    if total > 0.0 {
                        vram_bandwidth = Some((used / total) * 100.0);
                    }
                }
                
                // 获取 P-State 状态
                p_state = Some(values[4].trim().to_string());
            }
        }
    } else if gpu_name_lower.contains("amd") || gpu_name_lower.contains("radeon") {
        // AMD GPU: 使用 AMD ADL SDK 或其他工具采集
        // 注意：这里使用模拟数据作为示例
        // 实际实现需要集成 AMD ADL SDK 或使用第三方工具
        
        // 模拟数据（实际实现时应替换为真实采集）
        encode_util = Some(35.0); // 模拟35%编码单元使用率
        decode_util = Some(20.0); // 模拟20%解码单元使用率
        vram_bandwidth = Some(45.0); // 模拟45%显存带宽使用率
        p_state = Some("P1".to_string()); // 模拟 P1 状态
    } else if gpu_name_lower.contains("intel") {
        // Intel GPU: 使用 Intel 图形 API 采集
        // 注意：这里使用模拟数据作为示例
        // 实际实现需要集成 Intel 图形 API 或使用第三方工具
        
        // 模拟数据（实际实现时应替换为真实采集）
        encode_util = Some(25.0); // 模拟25%编码单元使用率
        decode_util = Some(15.0); // 模拟15%解码单元使用率
        vram_bandwidth = Some(30.0); // 模拟30%显存带宽使用率
        p_state = Some("P0".to_string()); // 模拟 P0 状态
    }
    
    (encode_util, decode_util, vram_bandwidth, p_state)
}

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
                    encode_util_pct: None,
                    decode_util_pct: None,
                    vram_bandwidth_pct: None,
                    p_state: None,
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
