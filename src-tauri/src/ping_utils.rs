// =============================================================================
// Ping/RTT 工具模块（Windows）
// - ICMP RTT（优先）
// - TCP 连接 RTT 回退
// - HTTPS 请求 RTT 回退
// - 多目标并发测量与聚合
// =============================================================================

use std::net::{Ipv4Addr, ToSocketAddrs};
use std::time::{Duration, Instant};
use std::thread;

use crate::process_utils::{_tcp_rtt_ms, RttResultPayload};

// windows crate: Icmp API
use ::windows::Win32::NetworkManagement::IpHelper::{
    IcmpCreateFile, IcmpSendEcho, IcmpCloseHandle, ICMP_ECHO_REPLY,
};

/// 解析字符串为 IPv4 地址（支持 "host:port"，优先直接 IPv4 字符串）
fn parse_ipv4(s: &str) -> Option<Ipv4Addr> {
    // 1) 如果形如 host:port，先取 host 部分
    let host = if let Some(idx) = s.rfind(':') {
        // 排除 IPv6 情况（含有两个或以上冒号）
        if s.matches(':').count() > 1 { s } else { &s[..idx] }
    } else { s };

    // 2) 直接按 IPv4 解析
    if let Ok(ip) = host.parse::<Ipv4Addr>() { return Some(ip); }

    // 3) 尝试 DNS 解析（附加端口 0）
    let query = format!("{}:0", host);
    if let Ok(mut iter) = query.to_socket_addrs() {
        if let Some(std::net::SocketAddr::V4(v4)) = iter.find(|a| matches!(a, std::net::SocketAddr::V4(_))) {
            return Some(*v4.ip());
        }
    }
    None
}

/// ICMP RTT（毫秒）。优先用于 IPv4 目标。返回 None 表示失败
pub fn icmp_rtt_ms(target: &str, timeout_ms: u32) -> Option<f64> {
    let ip = parse_ipv4(target)?;

    // 创建 ICMP 句柄
    let handle = match unsafe { IcmpCreateFile() } {
        Ok(h) => h,
        Err(_) => return None,
    };

    // 请求数据（任意负载）
    let req_data: [u8; 32] = [0x61; 32];
    // 回复缓冲区：至少 sizeof(ICMP_ECHO_REPLY) + 请求数据
    let reply_size = std::mem::size_of::<ICMP_ECHO_REPLY>() + req_data.len() + 8;
    let mut reply_buf: Vec<u8> = vec![0u8; reply_size];

    let dest_addr: u32 = u32::from(ip);

    let ret = unsafe {
        IcmpSendEcho(
            handle,
            dest_addr,
            req_data.as_ptr().cast(),
            req_data.len() as u16,
            None,
            reply_buf.as_mut_ptr().cast(),
            reply_buf.len() as u32,
            timeout_ms,
        )
    };

    let out = if ret > 0 {
        // 解析回复结构
        let reply: &ICMP_ECHO_REPLY = unsafe { &*(reply_buf.as_ptr() as *const ICMP_ECHO_REPLY) };
        // Status==0 表示 IP_SUCCESS
        if reply.Status == 0 {
            Some(reply.RoundTripTime as f64)
        } else {
            None
        }
    } else {
        None
    };

    // 关闭句柄
    unsafe { let _ = IcmpCloseHandle(handle); }

    out
}

/// HTTPS RTT（毫秒）。对 URL 发起 HEAD 请求
pub fn https_head_rtt_ms(url: &str, timeout_ms: u64) -> Option<f64> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(timeout_ms))
        .timeout_read(Duration::from_millis(timeout_ms))
        .timeout_write(Duration::from_millis(timeout_ms))
        .build();
    let start = Instant::now();
    match agent.head(url).call() {
        Ok(_) => Some(start.elapsed().as_secs_f64() * 1000.0),
        Err(_) => None,
    }
}

/// 单目标 RTT（带回退）：ICMP -> TCP -> HTTPS
pub fn measure_single_rtt(target: &str, timeout_ms: u64) -> Option<f64> {
    // 1) ICMP（针对 IPv4 主机/地址）
    if parse_ipv4(target).is_some() {
        if let Some(ms) = icmp_rtt_ms(target, timeout_ms as u32) { return Some(ms); }
    }

    // 2) TCP 连接（要求 host:port）
    if target.contains(':') {
        if let Some(ms) = _tcp_rtt_ms(target, timeout_ms) { return Some(ms); }
    }

    // 3) HTTPS 回退（将目标转换为 URL）
    // - 若已带协议则直接使用
    // - 若是纯主机/IPv4，则拼接 https://host/
    let url = if target.starts_with("http://") || target.starts_with("https://") {
        target.to_string()
    } else {
        // 去掉端口
        let host = if let Some(idx) = target.rfind(':') {
            if target.matches(':').count() > 1 { target } else { &target[..idx] }
        } else { target };
        format!("https://{}/", host)
    };
    https_head_rtt_ms(&url, timeout_ms)
}

/// 多目标并发 RTT 测量
pub fn measure_multi_rtt(targets: &[String], timeout_ms: u64) -> Vec<RttResultPayload> {
    if targets.is_empty() { return Vec::new(); }

    let mut handles = Vec::with_capacity(targets.len());
    for t in targets.iter().cloned() {
        // 直接每目标一个线程（目标数量通常很少）
        handles.push(thread::spawn(move || {
            let rtt = measure_single_rtt(&t, timeout_ms);
            RttResultPayload { target: t, rtt_ms: rtt, success: Some(rtt.is_some()) }
        }));
    }

    let mut results = Vec::with_capacity(targets.len());
    for h in handles {
        match h.join() {
            Ok(item) => results.push(item),
            Err(_) => results.push(RttResultPayload { target: "<panic>".into(), rtt_ms: None, success: Some(false) }),
        }
    }
    results
}
