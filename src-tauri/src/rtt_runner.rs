// RTT Runner：多目标 RTT 采集的最小实现
// 说明：
// - 依赖 BaseGate 做并发防重入
// - trigger() 异步执行测量，完成后更新内部快照
// - 提供 snapshot_json() 供外部拉取或事件上报

use std::sync::{Arc, Mutex};
use crate::runner::{Runner, BaseGate};
use crate::ping_utils::measure_multi_rtt;

#[derive(Clone)]
pub struct RttRunner {
    gate: Arc<BaseGate>,
    // 配置提供器：返回 (targets, timeout_ms)
    cfg_provider: Arc<dyn Fn() -> (Vec<String>, u64) + Send + Sync>,
    // 最近一次结果快照
    last_snapshot: Arc<Mutex<serde_json::Value>>,
}

impl RttRunner {
    pub fn new<F>(cfg_provider: F) -> Self
    where
        F: Fn() -> (Vec<String>, u64) + Send + Sync + 'static,
    {
        Self {
            gate: Arc::new(BaseGate::new()),
            cfg_provider: Arc::new(cfg_provider),
            last_snapshot: Arc::new(Mutex::new(serde_json::json!({}))),
        }
    }
}

impl Runner for RttRunner {
    fn name(&self) -> &'static str { "rtt_runner" }

    fn trigger(&self, now_ms: i64) {
        // 防重入
        if !self.gate.try_enter() { return; }
        self.gate.set_running();
        let gate_c = self.gate.clone();
        let snap_c = self.last_snapshot.clone();
        let cfg_c = self.cfg_provider.clone();
        std::thread::spawn(move || {
            let (targets, timeout_ms) = (cfg_c)();
            let results = measure_multi_rtt(&targets, timeout_ms);
            // 简单聚合：min/avg
            let lats: Vec<f64> = results.iter().filter_map(|r| r.rtt_ms).collect();
            let min_ms = lats.iter().cloned().fold(f64::INFINITY, f64::min);
            let avg_ms = if lats.is_empty() { 0.0 } else { lats.iter().sum::<f64>() / lats.len() as f64 };
            let min_opt = if lats.is_empty() { None } else { Some(min_ms) };
            let avg_opt = if lats.is_empty() { None } else { Some(avg_ms) };

            let payload = serde_json::json!({
                "timestamp_ms": now_ms,
                "targets": targets,
                "timeout_ms": timeout_ms,
                "results": results,
                "summary": {
                    "min_ms": min_opt,
                    "avg_ms": avg_opt,
                }
            });
            if let Ok(mut g) = snap_c.lock() { *g = payload; }
            // 标记成功并退出
            gate_c.mark_ok_and_exit(now_ms);
        });
    }

    fn is_running(&self) -> bool { self.gate.is_running() }
    fn last_ok_ms(&self) -> Option<i64> { self.gate.last_ok_ms() }

    fn snapshot_json(&self) -> serde_json::Value {
        match self.last_snapshot.lock() {
            Ok(g) => (*g).clone(),
            Err(_) => serde_json::json!({}),
        }
    }
}
