// 公网IP查询工具模块
// 包含公网IP和ISP信息的后台轮询功能

use crate::config_utils::PublicNetInfo;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// 启动公网IP/ISP后台轮询线程
pub fn start_public_net_polling(
    cfg_state: Arc<Mutex<crate::config_utils::AppConfig>>,
    pub_net: Arc<Mutex<PublicNetInfo>>,
) {
    thread::spawn(move || {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_secs(5))
            .timeout_write(Duration::from_secs(5))
            .build();

        #[derive(serde::Deserialize)]
        struct IpApiResp { 
            status: Option<String>, 
            query: Option<String>, 
            isp: Option<String>, 
            message: Option<String> 
        }
        
        #[derive(serde::Deserialize)]
        struct IpInfoResp { 
            ip: Option<String>, 
            org: Option<String> 
        }

        loop {
            let enabled = cfg_state
                .lock().ok()
                .and_then(|c| c.public_net_enabled)
                .unwrap_or(true);
            if !enabled {
                std::thread::sleep(Duration::from_secs(60));
                continue;
            }

            let mut ok = false;
            // 1) ip-api.com
            let try1 = agent.get("https://ip-api.com/json/?fields=status,query,isp,message").call();
            if let Ok(resp) = try1 {
                if let Ok(data) = resp.into_json::<IpApiResp>() {
                    if data.status.as_deref() == Some("success") {
                        if let Ok(mut g) = pub_net.lock() {
                            g.ip = data.query;
                            g.isp = data.isp;
                            g.last_updated_ms = Some(chrono::Local::now().timestamp_millis());
                            g.last_error = None;
                        }
                        ok = true;
                    } else if let Ok(mut g) = pub_net.lock() {
                        g.last_error = data.message.or(Some("ip-api.com failed".to_string()));
                    }
                }
            }

            // 2) fallback ipinfo.io
            if !ok {
                let try2 = agent.get("https://ipinfo.io/json").call();
                if let Ok(resp) = try2 {
                    if let Ok(data) = resp.into_json::<IpInfoResp>() {
                        if let Ok(mut g) = pub_net.lock() {
                            g.ip = data.ip;
                            g.isp = data.org; // org 常含 ASN+ISP 名称
                            g.last_updated_ms = Some(chrono::Local::now().timestamp_millis());
                            g.last_error = None;
                        }
                        ok = true;
                    }
                }
            }

            // 休眠：成功 30 分钟；失败 60 秒
            std::thread::sleep(if ok { Duration::from_secs(1800) } else { Duration::from_secs(60) });
        }
    });
}
