// SMART 后台 Worker：定期与按需采集磁盘 SMART 健康数据并广播事件
// 事件名："sensor://smart"，负载包含设备列表与时间戳

use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;

#[derive(Clone)]
pub struct SmartWorker {
    tx: Sender<SmartCmd>,
}

impl SmartWorker {
    pub fn request_refresh(&self) -> bool {
        self.tx.send(SmartCmd::Refresh).is_ok()
    }
}

#[derive(Debug)]
enum SmartCmd { Refresh, Shutdown }

// 事件负载改为 serde_json::Value，以满足 tauri::Emitter::emit 的 Clone 约束

pub fn start(app: tauri::AppHandle) -> SmartWorker {
    let (tx, rx): (Sender<SmartCmd>, Receiver<SmartCmd>) = mpsc::channel();

    // 后台采集线程
    let app_handle = app.clone();
    thread::Builder::new()
        .name("smart-worker".into())
        .spawn(move || {
            worker_loop(app_handle, rx);
        })
        .expect("spawn smart-worker");

    SmartWorker { tx }
}

fn now_ts_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn worker_loop(app: tauri::AppHandle, rx: Receiver<SmartCmd>) {
    let next_interval = Duration::from_secs(10); // 默认 10s 采集一次
    loop {
        // 先执行一次采集（冷启动尽快填充）
        collect_and_emit(&app);

        // 间隔期间监听指令，提前刷新或退出
        let mut elapsed = Duration::from_secs(0);
        while elapsed < next_interval {
            let wait = Duration::from_millis(200);
            match rx.recv_timeout(wait) {
                Ok(SmartCmd::Refresh) => {
                    collect_and_emit(&app);
                    // 刷新后重置间隔累计
                    elapsed = Duration::from_secs(0);
                    continue;
                }
                Ok(SmartCmd::Shutdown) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    elapsed += wait;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    }
}

fn collect_and_emit(app: &tauri::AppHandle) {
    // 建立 ROOT\\CIMV2 连接用于盘符映射与回退查询
    let data = (|| {
        if let Ok(com) = wmi::COMLibrary::new() {
            if let Ok(conn) = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", com) {
                return crate::smart_utils::wmi_list_smart_status(&conn);
            }
        }
        None
    })();

    let payload = serde_json::json!({
        "smart": data,
        "ts_ms": now_ts_ms(),
    });
    let _ = app.emit("sensor://smart", payload);
}
