// ================================================================================
// 进程监控工具模块
// ================================================================================

use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use std::time::{Duration, Instant};

// ================================================================================
// 1. 数据结构定义
// ================================================================================

#[derive(Clone, serde::Serialize)]
pub struct RttResultPayload {
    pub target: String,
    pub rtt_ms: Option<f64>,
    pub success: Option<bool>,
}

#[derive(Clone, serde::Serialize)]
pub struct TopProcessPayload {
    pub name: Option<String>,
    pub cpu_pct: Option<f32>,
    pub mem_bytes: Option<u64>,
}

// ================================================================================
// 2. RTT 测试函数
// ================================================================================

/// TCP RTT 测试（连接指定地址并测量往返时间）
pub fn tcp_rtt_ms(addr: &str, timeout_ms: u64) -> Option<f64> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    
    match TcpStream::connect_timeout(&addr.parse().ok()?, timeout) {
        Ok(_) => {
            let elapsed = start.elapsed();
            Some(elapsed.as_secs_f64() * 1000.0)
        }
        Err(_) => None,
    }
}

/// 多目标 RTT 测试（顺序串行测量）
pub fn measure_multi_rtt(targets: &[String], timeout_ms: u64) -> Option<Vec<RttResultPayload>> {
    if targets.is_empty() {
        return None;
    }
    
    let mut results = Vec::new();
    for target in targets {
        let rtt_ms = tcp_rtt_ms(target, timeout_ms);
        results.push(RttResultPayload {
            target: target.clone(),
            rtt_ms,
            success: Some(rtt_ms.is_some()),
        });
    }
    
    Some(results)
}

// ================================================================================
// 3. 进程监控函数
// ================================================================================

/// 获取Top N进程（按CPU和内存使用率排序）
pub fn get_top_processes(sys: &sysinfo::System, top_n: usize) -> (Option<Vec<TopProcessPayload>>, Option<Vec<TopProcessPayload>>) {
    use std::cmp::Ordering;
    
    let processes: Vec<&sysinfo::Process> = sys.processes().values().collect();
    if processes.is_empty() || top_n == 0 {
        return (None, None);
    }
    
    // CPU 排序
    let mut by_cpu = processes.clone();
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
    let mut by_mem = processes;
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

/// 计算磁盘累计读写字节数（按进程聚合）
pub fn calculate_disk_totals(sys: &sysinfo::System) -> (u64, u64) {
    let mut disk_r_total: u64 = 0;
    let mut disk_w_total: u64 = 0;
    
    for (_pid, proc) in sys.processes() {
        let disk_usage = proc.disk_usage();
        disk_r_total = disk_r_total.saturating_add(disk_usage.total_read_bytes);
        disk_w_total = disk_w_total.saturating_add(disk_usage.total_written_bytes);
    }
    
    (disk_r_total, disk_w_total)
}
