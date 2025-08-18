// Runner 抽象：统一的任务执行接口与并发防重入基元
// 说明：当前仅定义接口与基础工具，不引入任何上层依赖，便于各 Runner 复用。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// 任务运行时状态（可选共享）
#[derive(Default, Debug, Clone)]
pub struct RunMeta {
    pub last_ok_ms: Option<i64>,
    pub is_running: bool,
}

/// 通用 Runner 接口
/// - name: 固定名称，便于日志/调试
/// - trigger: 由调度器在到期 tick 触发，内部需自行做防重入
/// - is_running/last_ok_ms: 供 `TaskTable.fill_state()` 或其它观测使用
/// - snapshot_json: 提供轻量快照（可用于事件或聚合写入）
pub trait Runner: Send + Sync {
    fn name(&self) -> &'static str;
    fn trigger(&self, now_ms: i64);
    fn is_running(&self) -> bool;
    fn last_ok_ms(&self) -> Option<i64>;
    fn snapshot_json(&self) -> serde_json::Value;
}

/// 基础并发防重入与时间标记工具
#[derive(Debug)]
pub struct BaseGate {
    running: AtomicBool,
    meta: Mutex<RunMeta>,
}

impl BaseGate {
    pub fn new() -> Self {
        Self { running: AtomicBool::new(false), meta: Mutex::new(RunMeta::default()) }
    }

    /// 尝试进入运行态，如果已在运行返回 false
    pub fn try_enter(&self) -> bool {
        !self.running.swap(true, Ordering::SeqCst)
    }

    /// 标记成功并退出运行态
    pub fn mark_ok_and_exit(&self, now_ms: i64) {
        if let Ok(mut m) = self.meta.lock() { m.last_ok_ms = Some(now_ms); m.is_running = false; }
        self.running.store(false, Ordering::SeqCst);
    }

    /// 仅退出运行态（失败或早退）
    pub fn exit(&self) {
        if let Ok(mut m) = self.meta.lock() { m.is_running = false; }
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn set_running(&self) {
        if let Ok(mut m) = self.meta.lock() { m.is_running = true; }
    }

    pub fn is_running(&self) -> bool { self.running.load(Ordering::SeqCst) }
    pub fn last_ok_ms(&self) -> Option<i64> { self.meta.lock().ok().and_then(|m| m.last_ok_ms) }
}
