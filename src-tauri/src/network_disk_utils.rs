// ================================================================================
// 网络接口和磁盘查询工具模块
// ================================================================================

use serde;
use wmi;
use crate::{NetIfPayload, LogicalDiskPayload};

// ---- 网络适配器相关结构体 ----

#[derive(Debug, serde::Deserialize)]
#[serde(rename = "Win32_NetworkAdapter")]
pub struct Win32NetworkAdapter {
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "Index")]
    pub index: Option<i32>,
    #[serde(rename = "MACAddress")]
    pub mac_address: Option<String>,
    #[serde(rename = "Speed")]
    pub speed: Option<u64>,
    #[serde(rename = "AdapterType")]
    pub adapter_type: Option<String>,
    #[serde(rename = "NetEnabled")]
    pub net_enabled: Option<bool>,
    #[serde(rename = "PhysicalAdapter")]
    pub physical_adapter: Option<bool>,
    #[serde(rename = "NetConnectionStatus")]
    pub net_connection_status: Option<u16>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_NetworkAdapterConfiguration")]
pub struct Win32NetworkAdapterConfiguration {
    #[serde(rename = "Index")]
    pub index: Option<i32>,
    #[serde(rename = "IPAddress")]
    pub ip_address: Option<Vec<String>>,
    #[serde(rename = "DefaultIPGateway")]
    pub default_ip_gateway: Option<Vec<String>>,
    #[serde(rename = "DNSServerSearchOrder")]
    pub dns_servers: Option<Vec<String>>,
    #[serde(rename = "DHCPEnabled")]
    pub dhcp_enabled: Option<bool>,
}

// ---- 逻辑磁盘相关结构体 ----

#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_LogicalDisk")]
pub struct Win32LogicalDisk {
    #[serde(rename = "DeviceID")]
    pub device_id: Option<String>,
    #[serde(rename = "DriveType")]
    pub drive_type: Option<u32>,
    #[serde(rename = "Size")]
    pub size: Option<u64>,
    #[serde(rename = "FreeSpace")]
    pub free_space: Option<u64>,
    #[serde(rename = "FileSystem")]
    pub file_system: Option<String>,
}

// ---- 网络接口查询函数 ----

/// 查询网络接口列表，包含IP地址、MAC地址、速度等信息
pub fn wmi_list_net_ifs(conn: &wmi::WMIConnection) -> Option<Vec<NetIfPayload>> {
    let cfgs: Result<Vec<Win32NetworkAdapterConfiguration>, _> = conn.query();
    let ads: Result<Vec<Win32NetworkAdapter>, _> = conn.query();
    if let (Ok(cfgs), Ok(ads)) = (cfgs, ads) {
        use std::collections::HashMap;
        let mut by_index_ip: HashMap<i32, Vec<String>> = HashMap::new();
        let mut by_index_gw: HashMap<i32, Vec<String>> = HashMap::new();
        let mut by_index_dns: HashMap<i32, Vec<String>> = HashMap::new();
        let mut by_index_dhcp: HashMap<i32, bool> = HashMap::new();
        for c in cfgs.into_iter() {
            if let Some(idx) = c.index {
                if let Some(ips) = c.ip_address { by_index_ip.insert(idx, ips); }
                if let Some(gw) = c.default_ip_gateway { by_index_gw.insert(idx, gw); }
                if let Some(dns) = c.dns_servers { by_index_dns.insert(idx, dns); }
                if let Some(dhcp) = c.dhcp_enabled { by_index_dhcp.insert(idx, dhcp); }
            }
        }
        let mut out: Vec<NetIfPayload> = Vec::new();
        for a in ads.into_iter() {
            let enabled = a.net_enabled.unwrap_or(true);
            let physical = a.physical_adapter.unwrap_or(true);
            if !enabled || !physical { continue; }
            if a.mac_address.is_none() { continue; }
            let link_mbps = a.speed.map(|bps| (bps / 1_000_000) as u64);
            let (ips, gateway, dns, dhcp_enabled) = if let Some(idx) = a.index {
                (
                    by_index_ip.remove(&idx),
                    by_index_gw.remove(&idx),
                    by_index_dns.remove(&idx),
                    by_index_dhcp.get(&idx).copied(),
                )
            } else { (None, None, None, None) };
            // up 判定：优先 NetConnectionStatus == 2 (Connected)，否则回退 NetEnabled
            let _up = match a.net_connection_status {
                Some(2) => Some(true),
                Some(7) => Some(false), // Media disconnected
                _ => a.net_enabled,
            };
            // 转换 IP 列表为首个 IPv4/IPv6
            let (ipv4, ipv6) = if let Some(list) = ips {
                let mut v4: Option<String> = None;
                let mut v6: Option<String> = None;
                for ip in list.into_iter() {
                    if ip.contains(':') {
                        if v6.is_none() { v6 = Some(ip); }
                    } else {
                        if v4.is_none() { v4 = Some(ip); }
                    }
                    if v4.is_some() && v6.is_some() { break; }
                }
                (v4, v6)
            } else { (None, None) };
            // 回填 ips 列表（若无则尝试由 ipv4/ipv6 组合）
            let ips_list: Option<Vec<String>> = match (&ipv4, &ipv6) {
                (None, None) => None,
                (v4, v6) => {
                    let mut v: Vec<String> = Vec::new();
                    if let Some(x) = v4 { v.push(x.clone()); }
                    if let Some(x) = v6 { v.push(x.clone()); }
                    if v.is_empty() { None } else { Some(v) }
                }
            };
            let _speed_mbps = link_mbps.and_then(|v| i32::try_from(v).ok());
            let up_status = match a.net_connection_status {
                Some(2) => Some(true),  // Connected
                Some(0) | Some(1) | Some(3) | Some(4) | Some(5) | Some(6) | Some(7) => Some(false), // Disconnected/Other
                _ => None,
            };
            
            out.push(NetIfPayload {
                name: a.name.clone(),
                desc: Some(a.name.clone().unwrap_or_default()),
                ip: ips_list.as_ref().and_then(|list| list.first().cloned()),
                ips: ips_list,
                mac: a.mac_address.clone(),
                speed_mbps: a.speed.map(|s| (s / 1_000_000) as u64),
                bytes_recv: None,
                bytes_sent: None,
                errors_recv: None,
                errors_sent: None,
                packets_recv: None,
                packets_sent: None,
                media_type: a.adapter_type.clone(),
                dhcp_enabled,
                gateway: gateway.clone(),
                dns_servers: dns.clone(),
                dns: dns, // 前端兼容性别名
                status: Some(a.net_connection_status.unwrap_or_default().to_string()),
                up: up_status,
                // 新增网络质量监控指标，初始值为None
                packet_loss_pct: None,
                active_connections: None,
                discarded_recv: None,
                discarded_sent: None,
            });
        }
        if out.is_empty() { None } else { Some(out) }
    } else {
        None
    }
}

// ---- 逻辑磁盘查询函数 ----

/// 查询逻辑磁盘列表，包含容量和可用空间信息
pub fn wmi_list_logical_disks(conn: &wmi::WMIConnection) -> Option<Vec<LogicalDiskPayload>> {
    let res: Result<Vec<Win32LogicalDisk>, _> = conn.query();
    if let Ok(list) = res {
        let mut out: Vec<LogicalDiskPayload> = Vec::new();
        for d in list.into_iter() {
            // 3 = 本地磁盘；过滤掉光驱、网络驱动器等
            if d.drive_type != Some(3) { continue; }
            let total_gb = d.size.and_then(|v| {
                let gb = (v as f64) / 1073741824.0; // 1024^3
                Some(gb as f32)
            });
            let free_gb = d.free_space.and_then(|v| {
                let gb = (v as f64) / 1073741824.0;
                Some(gb as f32)
            });
            out.push(LogicalDiskPayload {
                drive_letter: d.device_id,
                total_gb: total_gb.map(|x| x as f64),
                free_gb: free_gb.map(|x| x as f64),
                usage_pct: total_gb.and_then(|total| {
                    free_gb.map(|free| {
                        if total > 0.0 {
                            ((total - free) / total * 100.0) as f64
                        } else {
                            0.0
                        }
                    })
                }),
                fs: d.file_system,
            });
        }
        if out.is_empty() { None } else { Some(out) }
    } else { None }
}
