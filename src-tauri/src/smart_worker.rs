// SMART 后台 Worker：定期与按需采集磁盘 SMART 健康数据并广播事件
// 事件名："sensor://smart"，负载包含设备列表与时间戳

use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use std::sync::{OnceLock, Mutex};

#[derive(Clone)]
pub struct SmartWorker {
    tx: Sender<SmartCmd>,
}

fn update_caches(payload: &serde_json::Value, had_error: bool) {
    let pcell = LAST_PAYLOAD.get_or_init(|| Mutex::new(None));
    if let Ok(mut g) = pcell.lock() { *g = Some(payload.clone()); }
    let ecell = LAST_ERROR.get_or_init(|| Mutex::new(None));
    if let Ok(mut g) = ecell.lock() {
        if had_error { *g = Some("smart collect failed".to_string()); } else { *g = None; }
    }
}

fn collect_and_emit(app: &tauri::AppHandle) {
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
    // 更新缓存与错误状态（data 为 None 视为错误）
    update_caches(&payload, data.is_none());
    let _ = app.emit("sensor://smart", payload);
}

pub fn get_last_snapshot() -> serde_json::Value {
    let p = LAST_PAYLOAD.get_or_init(|| Mutex::new(None));
    let e = LAST_ERROR.get_or_init(|| Mutex::new(None));
    let (payload, last_error) = (
        p.lock().ok().and_then(|g| g.clone()),
        e.lock().ok().and_then(|g| g.clone()),
    );
    if let Some(mut obj) = payload {
        if let Some(err) = last_error { obj["last_error"] = serde_json::Value::String(err); }
        return obj;
    }
    serde_json::json!({
        "smart": serde_json::Value::Null,
        "ts_ms": 0,
        "last_error": last_error,
    })
}

impl SmartWorker {
    pub fn request_refresh(&self) -> bool {
        self.tx.send(SmartCmd::Refresh).is_ok()
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(SmartCmd::Shutdown);
        // 尝试回收线程
        if let Some(lock) = WORKER_HANDLE.get() {
            if let Ok(mut guard) = lock.lock() {
                if let Some(handle) = guard.take() {
                    let _ = handle.join();
                }
            }
        }
    }
}

#[derive(Debug)]
enum SmartCmd { Refresh, Shutdown }

// 事件负载改为 serde_json::Value，以满足 tauri::Emitter::emit 的 Clone 约束

// 全局保存后台线程句柄，便于退出时回收
static WORKER_HANDLE: OnceLock<Mutex<Option<std::thread::JoinHandle<()>>>> = OnceLock::new();

// 最近一次事件负载与错误信息缓存（供前端启动时拉取）
static LAST_PAYLOAD: OnceLock<Mutex<Option<serde_json::Value>>> = OnceLock::new();
static LAST_ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();

pub fn start(app: tauri::AppHandle) -> SmartWorker {
    let (tx, rx): (Sender<SmartCmd>, Receiver<SmartCmd>) = mpsc::channel();

    // 后台采集线程
    let app_handle = app.clone();
    let handle = thread::Builder::new()
        .name("smart-worker".into())
        .spawn(move || {
            worker_loop(app_handle, rx);
        })
        .expect("spawn smart-worker");

    // 存储句柄
    let cell = WORKER_HANDLE.get_or_init(|| Mutex::new(None));
    if let Ok(mut g) = cell.lock() {
        *g = Some(handle);
    }

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

