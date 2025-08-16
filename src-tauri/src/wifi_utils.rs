// ================================================================================
// Wi-Fi 信息查询工具模块
// ================================================================================
// 
// 本模块包含Wi-Fi相关功能：
// - Wi-Fi连接信息查询（SSID、信号强度、速率等）
// - Windows netsh命令解析
// - 多语言支持（中英文）
// ================================================================================


// ---- Wi-Fi 信息结构体 ----

#[derive(Clone, Debug, Default)]
pub struct WifiInfoExt {
    pub ssid: Option<String>,
    pub signal_pct: Option<i32>,
    pub link_mbps: Option<i32>,
    pub bssid: Option<String>,
    pub channel: Option<i32>,
    pub radio: Option<String>,
    pub band: Option<String>,
    pub rx_mbps: Option<i32>,
    pub tx_mbps: Option<i32>,
    pub rssi_dbm: Option<i32>,
    // 标记 rssi_dbm 是否为根据 Signal% 估算
    pub rssi_estimated: bool,
    // 安全/加密/信道宽度（MHz）
    pub auth: Option<String>,
    pub cipher: Option<String>,
    pub chan_width_mhz: Option<i32>,
}

// ---- 控制台输出解码助手 ----

/// 控制台输出解码助手：优先 UTF-8，失败则回退 GBK（中文 Windows 常见），最后退回损失性 UTF-8
fn decode_console_bytes(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    // 回退 GBK（Windows 中文）
    let (decoded_str, _encoding, had_errors) = encoding_rs::GBK.decode(bytes);
    if !had_errors {
        return decoded_str.into_owned();
    }
    // 最后回退：损失性 UTF-8
    String::from_utf8_lossy(bytes).to_string()
}

// ---- Wi-Fi 信息查询函数 ----

/// 简化版Wi-Fi信息查询（仅返回SSID、信号强度、链路速度）
#[allow(dead_code)]
pub fn read_wifi_info() -> (Option<String>, Option<i32>, Option<i32>) {
    #[cfg(windows)]
    {
        let output = {
            let mut cmd = std::process::Command::new("netsh");
            cmd.args(["wlan", "show", "interfaces"]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            cmd.output()
        };
        if let Ok(out) = output {
            if out.status.success() {
                let text = decode_console_bytes(&out.stdout);
                let mut ssid: Option<String> = None;
                let mut signal_pct: Option<i32> = None;
                let mut rx_mbps: Option<i32> = None;
                let mut tx_mbps: Option<i32> = None;

                for line in text.lines() {
                    let t = line.trim();
                    let tl = t.to_lowercase();
                    // SSID（避免匹配到 BSSID）
                    if tl.starts_with("ssid") && !tl.starts_with("bssid") {
                        if let Some(pos) = t.find(':') {
                            let v = t[pos + 1..].trim();
                            if !v.is_empty() { ssid = Some(v.to_string()); }
                        }
                        continue;
                    }
                    // 信号强度："Signal" 或 中文 "信号"
                    if tl.contains("signal") || t.contains("信号") {
                        if let Some(pos) = t.find(':') {
                            let v = t[pos + 1..].trim();
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { signal_pct = Some(n.clamp(0, 100)); }
                        }
                        continue;
                    }
                    // 速率：接收/发送（英/中文）
                    if tl.contains("receive rate (mbps)") || t.contains("接收速率 (Mbps)") {
                        if let Some(pos) = t.find(':') {
                            let v = t[pos + 1..].trim();
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { rx_mbps = Some(n.max(0)); }
                        }
                        continue;
                    }
                    if tl.contains("transmit rate (mbps)") || t.contains("传输速率 (Mbps)") {
                        if let Some(pos) = t.find(':') {
                            let v = t[pos + 1..].trim();
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { tx_mbps = Some(n.max(0)); }
                        }
                        continue;
                    }
                }

                let link = rx_mbps.or(tx_mbps);
                return (ssid, signal_pct, link);
            }
        }
        (None, None, None)
    }
    #[cfg(not(windows))]
    {
        (None, None, None)
    }
}

/// 扩展版Wi-Fi信息查询（返回完整的Wi-Fi连接详情）
pub fn read_wifi_info_ext() -> WifiInfoExt {
    #[cfg(windows)]
    {
        let mut out = WifiInfoExt::default();
        let output = {
            let mut cmd = std::process::Command::new("netsh");
            cmd.args(["wlan", "show", "interfaces"]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            cmd.output()
        };
        if let Ok(outp) = output {
            if outp.status.success() {
                let text = decode_console_bytes(&outp.stdout);
                let raw_text_for_dbg = if cfg!(debug_assertions) { Some(text.clone()) } else { None };
                let mut rx_mbps: Option<i32> = None;
                let mut tx_mbps: Option<i32> = None;
                
                // 兼容中文冒号，返回所有权字符串以避免生命周期问题
                let get_after_colon = |s: &str| -> Option<String> {
                    if let Some(pos) = s.find(':') { return Some(s[pos + 1..].trim().to_string()); }
                    if let Some(pos2) = s.find('：') { return Some(s[pos2 + 1..].trim().to_string()); }
                    None
                };
                
                for line in text.lines() {
                    let t = line.trim();
                    let tl = t.to_lowercase();
                    // 提前解析键名（冒号左侧），用于区分 Channel 与 Channel width/bandwidth
                    let key = t.split(|ch| ch == ':' || ch == '：').next().unwrap_or(t).trim();
                    let keyl = key.to_lowercase();
                    
                    // Debug: 打印潜在带宽/加密相关行，便于定位不同语言/驱动差异
                    if cfg!(debug_assertions) {
                        if tl.contains("width") || t.contains("宽") || t.contains("带宽") {
                            println!("[wifi][match-cand][width] {}", t);
                        }
                        if tl.contains("cipher") || t.contains("加密") || t.contains("加密类型") || t.contains("加密方式") {
                            println!("[wifi][match-cand][cipher] {}", t);
                        }
                    }
                    
                    // SSID（避免匹配到 BSSID）
                    if tl.starts_with("ssid") && !tl.starts_with("bssid") {
                        if let Some(v) = get_after_colon(t) { if !v.is_empty() { out.ssid = Some(v); } }
                        continue;
                    }
                    // BSSID
                    if tl.starts_with("bssid") {
                        if let Some(v) = get_after_colon(t) { if !v.is_empty() { out.bssid = Some(v); } }
                        continue;
                    }
                    // 身份验证（Authentication）
                    if tl.contains("authentication") || t.contains("身份验证") {
                        if let Some(v) = get_after_colon(t) { if !v.is_empty() { out.auth = Some(v); } }
                        continue;
                    }
                    // 加密（Cipher）——兼容"加密/加密类型/加密方式"
                    if tl.contains("cipher") || t.contains("加密") || t.contains("加密类型") || t.contains("加密方式") {
                        if let Some(v) = get_after_colon(t) { if !v.is_empty() { out.cipher = Some(v); } }
                        continue;
                    }
                    // 信号强度（含"信号质量"）
                    if tl.contains("signal") || t.contains("信号") || t.contains("信号质量") {
                        if let Some(v) = get_after_colon(t) {
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { out.signal_pct = Some(n.clamp(0, 100)); }
                        }
                        continue;
                    }
                    // 信道（仅匹配键名正是 Channel/信道/通道/频道，避免误吞"Channel width/信道宽度/带宽"）
                    if keyl == "channel" || key == "信道" || key == "通道" || key == "频道" {
                        if let Some(v) = get_after_colon(t) {
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { out.channel = Some(n.max(0)); }
                        }
                        continue;
                    }
                    // 信道宽度/带宽（Channel width/bandwidth；中文兼容"信道/通道/频道 + 宽度/带宽"，及部分仅写"带宽/宽度"的驱动）
                    if (keyl.contains("channel") && (keyl.contains("width") || keyl.contains("bandwidth")))
                        || key.contains("信道宽度") || key.contains("通道宽度") || key.contains("频道宽度")
                        || key.contains("信道带宽") || key.contains("通道带宽") || key.contains("频道带宽")
                        || key.contains("带宽") || key.contains("宽度") {
                        if let Some(v) = get_after_colon(t) {
                            // 取出数值，如 "80 MHz"
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { if n > 0 { out.chan_width_mhz = Some(n); } }
                        }
                        continue;
                    }
                    // 无线制式（放宽匹配：Radio type/无线电类型/无线类型/物理类型）
                    if tl.contains("radio type") || t.contains("无线电类型") || t.contains("无线类型") || t.contains("物理类型") {
                        if let Some(v) = get_after_colon(t) { if !v.is_empty() { out.radio = Some(v); } }
                        continue;
                    }
                    // RSSI（部分系统会展示为 RSSI 或 信号质量）
                    if tl.starts_with("rssi") {
                        if let Some(v) = get_after_colon(t) {
                            // 可能为 "-45 dBm"
                            let mut s = String::new();
                            for ch in v.chars() { if ch == '-' || ch.is_ascii_digit() { s.push(ch); } }
                            if let Ok(n) = s.parse::<i32>() { out.rssi_dbm = Some(n); out.rssi_estimated = false; }
                        }
                        continue;
                    }
                    // 速率：接收（英/中文，放宽空格/大小写/括号）
                    if (tl.contains("receive rate") && tl.contains("mbps")) || t.contains("接收速率") {
                        if let Some(v) = get_after_colon(t) {
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { rx_mbps = Some(n.max(0)); }
                        }
                        continue;
                    }
                    // 速率：发送（英/中文，放宽空格/大小写/括号），兼容"传输速率/发送速率"
                    if (tl.contains("transmit rate") && tl.contains("mbps")) || t.contains("传输速率") || t.contains("发送速率") {
                        if let Some(v) = get_after_colon(t) {
                            let num: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
                            if let Ok(n) = num.parse::<i32>() { tx_mbps = Some(n.max(0)); }
                        }
                        continue;
                    }
                }

                out.rx_mbps = rx_mbps;
                out.tx_mbps = tx_mbps;
                
                // 若能从信道推断频段
                if out.band.is_none() {
                    if let Some(ch) = out.channel {
                        out.band = Some(
                            if (1..=14).contains(&ch) { "2.4GHz".to_string() }
                            else if (32..=177).contains(&ch) { "5GHz".to_string() }
                            else { "".to_string() }
                        ).filter(|s| !s.is_empty());
                    }
                }
                
                // 若无原生 RSSI，则基于 Signal% 估算：RSSI ~= round(signal/2 - 100)
                if out.rssi_dbm.is_none() {
                    if let Some(q) = out.signal_pct { // 0..100
                        let est = (q as f64 / 2.0 - 100.0).round() as i32;
                        out.rssi_dbm = Some(est);
                        out.rssi_estimated = true;
                    }
                }
                
                // 若未解析到"信道宽度"，进行保守回退推断（仅作为显示友好，不保证精确）
                if out.chan_width_mhz.is_none() {
                    let radio = out.radio.as_deref().unwrap_or("");
                    let band = out.band.as_deref().unwrap_or("");
                    let width = if radio.contains("802.11ax") {
                        Some(80)
                    } else if radio.contains("802.11ac") {
                        Some(80)
                    } else if radio.contains("802.11n") {
                        if band == "5GHz" { Some(40) } else { Some(20) }
                    } else if radio.contains("802.11a") || radio.contains("802.11b") || radio.contains("802.11g") {
                        Some(20)
                    } else { None };
                    if let Some(w) = width {
                        out.chan_width_mhz = Some(w);
                        if cfg!(debug_assertions) {
                            println!("[wifi][width][fallback] radio={:?} band={:?} -> width={} MHz", out.radio, out.band, w);
                        }
                    }
                }
                
                // Debug 构建下输出解析摘要，便于现场排错
                if cfg!(debug_assertions) {
                    if out.signal_pct.is_none() && out.channel.is_none() && out.radio.is_none() && out.rx_mbps.is_none() && out.tx_mbps.is_none() && out.rssi_dbm.is_none() && out.auth.is_none() && out.cipher.is_none() && out.chan_width_mhz.is_none() {
                        if let Some(raw) = raw_text_for_dbg.as_ref() {
                            println!("[wifi][raw]\n{}", raw);
                        }
                    }
                    println!(
                        "[wifi][parsed] ssid={:?} signal%={:?} ch={:?} radio={:?} band={:?} rx={:?} tx={:?} bssid={:?} rssi={:?} rssi_est={} auth={:?} cipher={:?} width={:?}",
                        out.ssid, out.signal_pct, out.channel, out.radio, out.band, out.rx_mbps, out.tx_mbps, out.bssid, out.rssi_dbm, out.rssi_estimated, out.auth, out.cipher, out.chan_width_mhz
                    );
                }
                return out;
            }
        }
        out
    }
    #[cfg(not(windows))]
    {
        WifiInfoExt::default()
    }
}
