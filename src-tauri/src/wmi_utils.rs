// WMI查询工具模块
// 包含各种WMI性能计数器和系统信息查询函数

use crate::types::{NetIfPayload, LogicalDiskPayload, SmartHealthPayload, PerfOsMemory, PerfDiskPhysical, PerfTcpipNic};

/// 汇总磁盘 IOPS 与队列长度（排除 _Total）
pub fn wmi_perf_disk(conn: &wmi::WMIConnection) -> (Option<f64>, Option<f64>, Option<f64>) {
    let results: Result<Vec<PerfDiskPhysical>, _> = conn.raw_query("SELECT * FROM Win32_PerfRawData_PerfDisk_PhysicalDisk");
    if let Ok(disks) = results {
        let mut total_r_iops = 0.0;
        let mut total_w_iops = 0.0;
        let mut total_queue = 0.0;
        let mut count = 0;
        for disk in disks {
            if let Some(name) = &disk.name {
                if name.contains("_Total") { continue; }
            }
            if let Some(r) = disk.disk_reads_per_sec { total_r_iops += r; }
            if let Some(w) = disk.disk_writes_per_sec { total_w_iops += w; }
            if let Some(q) = disk.current_disk_queue_length { total_queue += q; count += 1; }
        }
        let avg_queue = if count > 0 { Some(total_queue / count as f64) } else { None };
        (Some(total_r_iops), Some(total_w_iops), avg_queue)
    } else {
        (None, None, None)
    }
}

/// 电池健康查询函数
pub fn wmi_read_battery_health(conn: &wmi::WMIConnection) -> (Option<u32>, Option<u32>, Option<u32>) {
    let results: Result<Vec<crate::battery_utils::Win32Battery>, _> = conn.raw_query("SELECT * FROM Win32_Battery");
    if let Ok(batteries) = results {
        if let Some(battery) = batteries.first() {
            return (
                battery.design_capacity,
                battery.full_charge_capacity,
                battery.cycle_count,
            );
        }
    }
    (None, None, None)
}

/// 汇总网络错误率（每秒，排除 _Total）
pub fn wmi_perf_net_err(conn: &wmi::WMIConnection) -> (Option<f64>, Option<f64>) {
    let results: Result<Vec<PerfTcpipNic>, _> = conn.raw_query("SELECT * FROM Win32_PerfRawData_Tcpip_NetworkInterface");
    if let Ok(nics) = results {
        let mut total_rx_err = 0.0;
        let mut total_tx_err = 0.0;
        for nic in nics {
            if let Some(name) = &nic.name {
                if name.contains("_Total") || name.contains("Loopback") { continue; }
            }
            if let Some(rx_err) = nic.packets_received_errors { total_rx_err += rx_err as f64; }
            if let Some(tx_err) = nic.packets_outbound_errors { total_tx_err += tx_err as f64; }
        }
        (Some(total_rx_err), Some(total_tx_err))
    } else {
        (None, None)
    }
}

/// 查询内存细分信息（缓存/提交/分页等）
pub fn wmi_perf_memory(conn: &wmi::WMIConnection) -> (
    Option<f32>, Option<f32>, Option<f32>, Option<f32>, Option<f32>,
    Option<f64>, Option<f64>, Option<f64>, Option<f64>
) {
    // 1) 查询 Win32_PerfRawData_PerfOS_Memory
    let mem_results: Result<Vec<PerfOsMemory>, _> = conn.raw_query("SELECT * FROM Win32_PerfRawData_PerfOS_Memory");
    let (cache_gb, committed_gb, commit_limit_gb, pool_paged_gb, pool_nonpaged_gb, pages_ps, page_reads_ps, page_writes_ps, page_faults_ps) = if let Ok(mem_data) = mem_results {
        if let Some(mem) = mem_data.first() {
            let cache_gb = mem.cache_bytes.map(|b| b as f32 / 1_073_741_824.0);
            let committed_gb = mem.committed_bytes.map(|b| b as f32 / 1_073_741_824.0);
            let commit_limit_gb = mem.commit_limit.map(|b| b as f32 / 1_073_741_824.0);
            let pool_paged_gb = mem.pool_paged_bytes.map(|b| b as f32 / 1_073_741_824.0);
            let pool_nonpaged_gb = mem.pool_nonpaged_bytes.map(|b| b as f32 / 1_073_741_824.0);
            (
                cache_gb, committed_gb, commit_limit_gb, pool_paged_gb, pool_nonpaged_gb,
                mem.pages_per_sec, mem.page_reads_per_sec, mem.page_writes_per_sec, mem.page_faults_per_sec
            )
        } else {
            (None, None, None, None, None, None, None, None, None)
        }
    } else {
        (None, None, None, None, None, None, None, None, None)
    };

    // 2) 查询 Win32_OperatingSystem 获取可用内存
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

    let os_results: Result<Vec<Win32OS>, _> = conn.raw_query("SELECT TotalVirtualMemorySize, TotalVisibleMemorySize, FreeVirtualMemory, FreePhysicalMemory FROM Win32_OperatingSystem");
    let avail_gb = if let Ok(os_data) = os_results {
        if let Some(os) = os_data.first() {
            // FreePhysicalMemory 单位为 KB，转换为 GB
            os.free_physical_memory.map(|kb| kb as f32 / 1_048_576.0)
        } else {
            None
        }
    } else {
        None
    };

    (avail_gb, cache_gb, committed_gb, commit_limit_gb, pool_paged_gb, pages_ps, page_reads_ps, page_writes_ps, page_faults_ps)
}

/// 控制台输出解码助手：优先 UTF-8，失败则回退 GBK（中文 Windows 常见），最后退回损失性 UTF-8
pub fn decode_console_bytes(bytes: &[u8]) -> String {
    // 1) 尝试 UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    // 2) 尝试 GBK
    let (decoded, _, _) = encoding_rs::GBK.decode(bytes);
    if !decoded.is_empty() {
        return decoded.into_owned();
    }
    // 3) 损失性 UTF-8
    String::from_utf8_lossy(bytes).into_owned()
}


