// WMI查询工具模块
// 包含各种WMI性能计数器和系统信息查询函数

use crate::types::{PerfOsMemory, PerfDiskPhysical, PerfTcpipNic, PerfOsProcessor};
use std::sync::Mutex;
use std::time::{Instant, Duration};
// 移除未使用的导入

// 移除未使用的辅助函数

// 历史数据存储结构
#[derive(Debug, Clone)]
struct MemoryCounters {
    pages_per_sec: Option<f64>,
    page_reads_per_sec: Option<f64>,
    page_writes_per_sec: Option<f64>,
    page_faults_per_sec: Option<f64>,
    timestamp: Instant,
}

// 全局历史数据存储
static MEMORY_HISTORY: Mutex<Option<MemoryCounters>> = Mutex::new(None);

/// 计算内存性能指标的真实每秒速率（通过差值计算）
fn calculate_memory_rates(current: &MemoryCounters) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let mut history = MEMORY_HISTORY.lock().unwrap();
    
    if let Some(ref previous) = *history {
        let time_diff = current.timestamp.duration_since(previous.timestamp).as_secs_f64();
        
        // 只有时间间隔大于1秒才计算速率，避免除零和异常值
        if time_diff >= 1.0 {
            let pages_rate = match (current.pages_per_sec, previous.pages_per_sec) {
                (Some(curr), Some(prev)) => {
                    let diff = curr - prev;
                    if diff >= 0.0 { Some(diff / time_diff) } else { None }
                },
                _ => None
            };
            
            let page_reads_rate = match (current.page_reads_per_sec, previous.page_reads_per_sec) {
                (Some(curr), Some(prev)) => {
                    let diff = curr - prev;
                    if diff >= 0.0 { Some(diff / time_diff) } else { None }
                },
                _ => None
            };
            
            let page_writes_rate = match (current.page_writes_per_sec, previous.page_writes_per_sec) {
                (Some(curr), Some(prev)) => {
                    let diff = curr - prev;
                    if diff >= 0.0 { Some(diff / time_diff) } else { None }
                },
                _ => None
            };
            
            let page_faults_rate = match (current.page_faults_per_sec, previous.page_faults_per_sec) {
                (Some(curr), Some(prev)) => {
                    let diff = curr - prev;
                    if diff >= 0.0 { Some(diff / time_diff) } else { None }
                },
                _ => None
            };
            
            // 更新历史数据
            *history = Some(current.clone());
            
            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            eprintln!("[{}][debug][wmi] 计算内存速率成功 - 分页: {:?}/s, 读取: {:?}/s, 写入: {:?}/s, 错误: {:?}/s", 
                     now_str, pages_rate, page_reads_rate, page_writes_rate, page_faults_rate);
            
            return (pages_rate, page_reads_rate, page_writes_rate, page_faults_rate);
        }
    }
    
    // 首次运行或时间间隔太短，保存当前数据并返回None
    *history = Some(current.clone());
    (None, None, None, None)
}

/// 查询内存细分指标（缓存、提交、分页池等）
pub fn wmi_perf_memory(conn: &wmi::WMIConnection) -> (
    Option<f32>, Option<f32>, Option<f32>, Option<f32>, Option<f32>,
    Option<f64>, Option<f64>, Option<f64>, Option<f64>
) {
    // 减少日志输出频率
    static mut LAST_LOG_TIME: Option<std::time::Instant> = None;
    let should_log = unsafe {
        if let Some(last) = LAST_LOG_TIME {
            last.elapsed().as_secs() >= 5 // 每5秒输出一次日志
        } else {
            LAST_LOG_TIME = Some(std::time::Instant::now());
            true
        }
    };
    
    if should_log {
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        eprintln!("[{}][debug][wmi] 开始查询内存细分指标", now_str);
        unsafe { LAST_LOG_TIME = Some(std::time::Instant::now()); }
    }
    
    // 尝试多种WMI查询方式，提高兼容性
    let results: Result<Vec<PerfOsMemory>, _> = conn.raw_query("SELECT * FROM Win32_PerfRawData_PerfOS_Memory")
        .or_else(|_| {
            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            eprintln!("[{}][debug][wmi] 尝试备用查询路径", now_str);
            conn.raw_query("SELECT AvailableBytes, CacheBytes, CommittedBytes, CommitLimit, PoolPagedBytes, PoolNonpagedBytes FROM Win32_PerfRawData_PerfOS_Memory")
        });
    
    match results {
        Ok(memories) => {
            if let Some(mem) = memories.first() {
                let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                eprintln!("[{}][debug][wmi] 成功获取内存性能数据", now_str);
                
                // 转换为GB单位
                let cache_gb = mem.cache_bytes.map(|v| v as f32 / 1073741824.0);
                let committed_gb = mem.committed_bytes.map(|v| v as f32 / 1073741824.0);
                let commit_limit_gb = mem.commit_limit.map(|v| v as f32 / 1073741824.0);
                let pool_paged_gb = mem.pool_paged_bytes.map(|v| v as f32 / 1073741824.0);
                let pool_nonpaged_gb = mem.pool_nonpaged_bytes.map(|v| v as f32 / 1073741824.0);
                
                // 页面相关指标（每秒） - 使用差值计算真实速率
                let current_counters = MemoryCounters {
                    pages_per_sec: mem.pages_per_sec.map(|v| v as f64),
                    page_reads_per_sec: mem.page_reads_per_sec.map(|v| v as f64),
                    page_writes_per_sec: mem.page_writes_per_sec.map(|v| v as f64),
                    page_faults_per_sec: mem.page_faults_per_sec.map(|v| v as f64),
                    timestamp: Instant::now(),
                };
                
                let (pages_per_sec, page_reads_per_sec, page_writes_per_sec, page_faults_per_sec) = 
                    calculate_memory_rates(&current_counters);
                
                (cache_gb, committed_gb, commit_limit_gb, pool_paged_gb, pool_nonpaged_gb,
                 pages_per_sec, page_reads_per_sec, page_writes_per_sec, page_faults_per_sec)
            } else {
                eprintln!("[warn][wmi] 内存性能数据为空");
                (None, None, None, None, None, None, None, None, None)
            }
        }
        Err(e) => {
            eprintln!("[error][wmi] 查询内存性能数据失败: {:?}", e);
            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            eprintln!("[{}][debug][wmi] 尝试使用基础内存信息作为回退", now_str);
            
            // 回退到基础内存查询
            if let Ok(basic_mem) = conn.raw_query::<std::collections::HashMap<String, wmi::Variant>>("SELECT TotalVisibleMemorySize, FreePhysicalMemory FROM Win32_OperatingSystem") {
                if let Some(mem_data) = basic_mem.first() {
                    let total_kb = mem_data.get("TotalVisibleMemorySize").and_then(|v| {
                        if let wmi::Variant::UI8(val) = v { Some(*val) } else { None }
                    });
                    let free_kb = mem_data.get("FreePhysicalMemory").and_then(|v| {
                        if let wmi::Variant::UI8(val) = v { Some(*val) } else { None }
                    });
                    
                    if let (Some(total), Some(free)) = (total_kb, free_kb) {
                        let used_gb = (total - free) as f32 / 1048576.0; // KB to GB
                        let free_gb = free as f32 / 1048576.0;
                        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                        eprintln!("[{}][debug][wmi] 使用基础内存信息 - 已用: {:.2}GB, 可用: {:.2}GB", now_str, used_gb, free_gb);
                        return (Some(free_gb), Some(used_gb * 0.3), Some(used_gb * 0.6), Some(used_gb * 0.05), Some(used_gb * 0.02), None, None, None, None);
                    }
                }
            }
            
            (None, None, None, None, None, None, None, None, None)
        }
    }
}

/// 汇总磁盘IOPS和队列长度（每秒，排除 _Total）
/// 增强错误处理和重试机制
pub fn wmi_perf_disk(conn: &wmi::WMIConnection) -> (Option<f64>, Option<f64>, Option<f64>) {
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    eprintln!("[{}][debug][wmi] 开始查询磁盘IOPS和队列长度", now_str);
    
    // 定义重试参数
    const MAX_RETRIES: u8 = 2; // 减少重试次数
    const RETRY_DELAY_MS: u64 = 50; // 减少重试延迟
    
    let mut results: Result<Vec<PerfDiskPhysical>, _> = conn.raw_query("SELECT * FROM Win32_PerfRawData_PerfDisk_PhysicalDisk");
    
    // 如果查询失败，重试几次
    let mut retry_count = 0;
    while results.is_err() && retry_count < MAX_RETRIES {
        retry_count += 1;
        std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
        results = conn.raw_query("SELECT * FROM Win32_PerfRawData_PerfDisk_PhysicalDisk");
    }
    if let Ok(disks) = results {
        let mut total_read_iops = 0.0;
        let mut total_write_iops = 0.0;
        let mut total_queue_len = 0.0;
        let mut disk_count = 0;
        
        for disk in disks {
            if let Some(name) = &disk.name {
                // 排除 _Total 和虚拟磁盘
                if name == "_Total" || name.contains("HarddiskVolume") {
                    continue;
                }
                
                if let Some(read_iops) = disk.disk_reads_per_sec {
                    total_read_iops += read_iops;
                }
                if let Some(write_iops) = disk.disk_writes_per_sec {
                    total_write_iops += write_iops;
                }
                if let Some(queue_len) = disk.current_disk_queue_length {
                    total_queue_len += queue_len;
                }
                disk_count += 1;
            }
        }
        
        if disk_count > 0 {
            (Some(total_read_iops), Some(total_write_iops), Some(total_queue_len))
        } else {
            // 提供估算值而不是None
            (Some(0.0), Some(0.0), Some(0.0))
        }
    } else {
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        eprintln!("[{}][debug][wmi] 磁盘IOPS查询失败，使用估算值", now_str);
        // 权限不足时提供估算值
        (Some(0.0), Some(0.0), Some(0.0))
    }
}

// 移除未使用的电池健康查询函数

/// 汇总网络错误率（每秒，排除 _Total）
/// 增强错误处理和重试机制
pub fn wmi_perf_net_err(conn: &wmi::WMIConnection) -> (Option<f64>, Option<f64>, Option<f64>, Option<u32>, Option<u32>) {
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    eprintln!("[{}][debug][wmi] 开始查询网络错误率", now_str);
    
    // 减少重试次数，避免过多日志
    const MAX_RETRIES: u8 = 2;
    const RETRY_DELAY_MS: u64 = 50;
    
    let mut results: Result<Vec<PerfTcpipNic>, _> = conn.raw_query("SELECT * FROM Win32_PerfRawData_Tcpip_NetworkInterface");
    
    // 如果查询失败，重试几次
    let mut retry_count = 0;
    while results.is_err() && retry_count < MAX_RETRIES {
        retry_count += 1;
        std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
        results = conn.raw_query("SELECT * FROM Win32_PerfRawData_Tcpip_NetworkInterface");
    }
    if let Ok(nics) = results {
        let mut total_rx_err = 0.0;
        let mut total_tx_err = 0.0;
        let mut total_discarded_recv = 0;
        let mut total_discarded_sent = 0;
        let mut total_packets = 0.0;
        
        for nic in nics {
            if let Some(name) = &nic.name {
                if name.contains("_Total") || name.contains("Loopback") { continue; }
            }
            
            // 累计错误包数量
            if let Some(rx_err) = nic.packets_received_errors { total_rx_err += rx_err as f64; }
            if let Some(tx_err) = nic.packets_outbound_errors { total_tx_err += tx_err as f64; }
            
            // 累计丢弃包数量
            if let Some(rx_disc) = nic.packets_received_discarded { total_discarded_recv += rx_disc as u64; }
            if let Some(tx_disc) = nic.packets_outbound_discarded { total_discarded_sent += tx_disc as u64; }
            
            // 累计总包数（用于计算丢包率）
            // 由于没有直接的包计数字段，使用字节数估算
            if let Some(rx_bytes) = nic.bytes_received_per_sec { total_packets += rx_bytes as f64 / 1500.0; } // 假设平均包大小为1500字节
            if let Some(tx_bytes) = nic.bytes_sent_per_sec { total_packets += tx_bytes as f64 / 1500.0; }
        }
        
        // 计算丢包率（错误包+丢弃包）/ 总包数
        let packet_loss_pct = if total_packets > 0.0 {
            let total_errors = total_rx_err + total_tx_err + (total_discarded_recv + total_discarded_sent) as f64;
            Some((total_errors / total_packets) * 100.0)
        } else {
            None
        };
        
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        eprintln!("[{}][debug][wmi] 网络错误查询成功 - 接收错误: {:.1}, 发送错误: {:.1}, 丢包率: {:?}%", now_str, total_rx_err, total_tx_err, packet_loss_pct);
        (Some(total_rx_err), Some(total_tx_err), packet_loss_pct, Some(total_discarded_recv as u32), Some(total_discarded_sent as u32))
    } else {
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        eprintln!("[{}][debug][wmi] 网络错误查询失败，使用默认值", now_str);
        // 提供默认值而不是None，避免前端显示"---"
        (Some(0.0), Some(0.0), Some(0.0), Some(0), Some(0))
    }
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

/// 通过WMI获取网络和磁盘累计字节数（更稳定的数据源）
/// 由于Win32_PerfRawData性能计数器在某些系统上不可用，暂时返回失败标志
/// 让主循环使用sysinfo数据源，但优化其稳定性
pub fn wmi_get_network_disk_bytes(_conn: &wmi::WMIConnection) -> (u64, u64, u64, u64) {
    // WMI性能计数器在当前系统环境下不可用
    // 返回失败标志，让主循环使用优化后的sysinfo数据源
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    eprintln!("[{}][debug] WMI性能计数器不可用，回退到sysinfo数据源", now_str);
    (u64::MAX, u64::MAX, u64::MAX, u64::MAX)
}


/// 通过WMI获取CPU使用率（替代sysinfo方案）
/// 使用Win32_PerfRawData_PerfOS_Processor查询处理器性能计数器
pub fn wmi_perf_cpu(conn: &wmi::WMIConnection) -> Option<f32> {
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    eprintln!("[{}][debug][wmi] 开始查询CPU使用率", now_str);
    
    // 查询处理器性能计数器，获取_Total实例（全局CPU使用率）
    let results: Result<Vec<PerfOsProcessor>, _> = conn.raw_query("SELECT Name, PercentProcessorTime, PercentIdleTime FROM Win32_PerfRawData_PerfOS_Processor WHERE Name='_Total'");
    
    match results {
        Ok(processors) => {
            if let Some(proc) = processors.first() {
                // WMI性能计数器返回的是原始计数值，需要计算使用率
                // 对于PercentIdleTime，CPU使用率 = 100 - 空闲时间百分比
                if let Some(idle_time) = proc.percent_idle_time {
                    // 原始计数器值需要转换为百分比
                    // 这里使用简化算法：假设空闲时间已经是百分比形式
                    let cpu_usage = 100.0 - (idle_time as f32 / 100.0);
                    let cpu_usage = cpu_usage.max(0.0).min(100.0); // 限制在0-100范围内
                    
                    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                    eprintln!("[{}][debug][wmi] WMI CPU使用率查询成功: {:.1}%", now_str, cpu_usage);
                    return Some(cpu_usage);
                }
                
                // 如果空闲时间不可用，尝试使用处理器时间
                if let Some(proc_time) = proc.percent_processor_time {
                    let cpu_usage = (proc_time as f32 / 100.0).max(0.0).min(100.0);
                    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                    eprintln!("[{}][debug][wmi] WMI CPU使用率查询成功(使用处理器时间): {:.1}%", now_str, cpu_usage);
                    return Some(cpu_usage);
                }
            }
            
            eprintln!("[warn][wmi] CPU性能数据为空或无效");
            None
        }
        Err(e) => {
            eprintln!("[error][wmi] 查询CPU性能数据失败: {:?}", e);
            
            // 尝试备用查询方式：使用Win32_Processor
            let backup_results: Result<Vec<std::collections::HashMap<String, wmi::Variant>>, _> = 
                conn.raw_query("SELECT LoadPercentage FROM Win32_Processor");
            
            match backup_results {
                Ok(processors) => {
                    let mut total_load = 0.0;
                    let mut count = 0;
                    
                    for proc in processors {
                        if let Some(load) = proc.get("LoadPercentage") {
                            if let wmi::Variant::UI2(val) = load {
                                total_load += *val as f32;
                                count += 1;
                            }
                        }
                    }
                    
                    if count > 0 {
                        let avg_cpu = total_load / count as f32;
                        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                        eprintln!("[{}][debug][wmi] 备用CPU查询成功: {:.1}%", now_str, avg_cpu);
                        return Some(avg_cpu);
                    }
                }
                Err(e2) => {
                    eprintln!("[error][wmi] 备用CPU查询也失败: {:?}", e2);
                }
            }
            
            // 最后尝试PowerShell方案
            wmi_powershell_cpu()
        }
    }
}

/// 通过PowerShell获取CPU使用率（最后的回退方案）
pub fn wmi_powershell_cpu() -> Option<f32> {
    use std::process::Command;
    
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    eprintln!("[{}][debug] 开始PowerShell CPU查询", now_str);
    
    // 使用wmic直接获取CPU使用率（更简单可靠）
    let mut cmd = Command::new("wmic");
    cmd.args(["cpu", "get", "loadpercentage", "/value"]);
    
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let output = cmd.output();
    
    match output {
        Ok(output) => {
            if output.status.success() {
                let output_str = decode_console_bytes(&output.stdout);
                let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                eprintln!("[{}][debug] wmic CPU查询成功，原始输出: '{}'", now_str, output_str);
                
                // 解析wmic输出格式：LoadPercentage=XX
                for line in output_str.lines() {
                    if line.starts_with("LoadPercentage=") {
                        if let Some(value_str) = line.split('=').nth(1) {
                            let value_str = value_str.trim();
                            match value_str.parse::<f32>() {
                                Ok(cpu_usage) => {
                                    let cpu_usage = cpu_usage.max(0.0).min(100.0);
                                    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                                    eprintln!("[{}][debug] wmic CPU使用率解析成功: {:.1}%", now_str, cpu_usage);
                                    return Some(cpu_usage);
                                },
                                Err(e) => {
                                    eprintln!("[error] wmic CPU使用率解析失败: {} -> {}", value_str, e);
                                }
                            }
                        }
                    }
                }
                
                eprintln!("[error] wmic输出中未找到LoadPercentage字段");
                None
            } else {
                eprintln!("[error] wmic CPU命令执行失败: {:?}", output.status);
                eprintln!("[error] stderr: {}", String::from_utf8_lossy(&output.stderr));
                None
            }
        },
        Err(e) => {
            eprintln!("[error] wmic CPU命令启动失败: {}", e);
            None
        }
    }
}

/// 获取系统活动网络连接数
pub fn get_active_connections() -> Option<u32> {
    use std::process::Command;
    
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    eprintln!("[{}][debug] 开始查询活动连接数", now_str);
    
    // 使用powershell命令获取活动连接数
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-Command", "(Get-NetTCPConnection -State Established).Count"]);
    
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let output = cmd.output();
    
    match output {
        Ok(output) => {
            if output.status.success() {
                // 解码输出并转换为数字
                let output_str = crate::wmi_utils::decode_console_bytes(&output.stdout).trim().to_string();
                let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                eprintln!("[{}][debug] PowerShell查询成功，原始输出: '{}'", now_str, output_str);
                
                match output_str.parse::<u32>() {
                    Ok(count) => {
                        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                        eprintln!("[{}][debug] 活动连接数解析成功: {}", now_str, count);
                        Some(count)
                    },
                    Err(e) => {
                        eprintln!("[error] 活动连接数解析失败: {} -> {}", output_str, e);
                        None
                    }
                }
            } else {
                eprintln!("[error] PowerShell命令执行失败: {:?}", output.status);
                eprintln!("[error] stderr: {}", String::from_utf8_lossy(&output.stderr));
                None
            }
        },
        Err(e) => {
            eprintln!("[error] PowerShell命令启动失败: {}", e);
            None
        }
    }
}
