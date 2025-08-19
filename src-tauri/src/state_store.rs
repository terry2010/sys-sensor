// 统一 State Store（骨架）
// 目标：集中维护各领域最新状态，并在每个 tick 聚合构建对外快照。
// 当前阶段：先接入 tick 级监控指标，为后续迁移 CPU/内存/网络/磁盘/SMART 等做铺垫。

use std::time::SystemTime;
use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TickTelemetry {
    pub tick: u64,
    pub timestamp_ms: i64,
    pub tick_cost_ms: Option<u64>,
    pub frame_skipped: bool,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Aggregated {
    pub timestamp_ms: i64,
    pub cpu_usage: Option<f32>,
    pub mem_pct: Option<f32>,
    pub net_rx_bps: Option<f64>,
    pub net_tx_bps: Option<f64>,
    pub disk_r_bps: Option<f64>,
    pub disk_w_bps: Option<f64>,
    // LDisk IOPS（逻辑盘）
    pub disk_r_iops: Option<f64>,
    pub disk_w_iops: Option<f64>,
    pub ping_rtt_ms: Option<f32>,
    pub battery_percent: Option<f32>,
    // 扩展字段：磁盘/网络/GPU
    pub disk_queue_len: Option<f64>,
    // 网络错误/丢弃/丢包统计（每秒或估算百分比）
    pub net_rx_err_ps: Option<f64>,
    pub net_tx_err_ps: Option<f64>,
    pub packet_loss_pct: Option<f64>,
    pub discarded_recv: Option<u32>,
    pub discarded_sent: Option<u32>,
    pub active_connections: Option<u32>,
    // GPU 概览
    pub gpu_count: Option<u32>,
    // RTT 聚合（多目标）
    pub rtt_avg_ms: Option<f32>,
    pub rtt_min_ms: Option<f32>,
    pub rtt_max_ms: Option<f32>,
    pub rtt_success_ratio: Option<f32>,
    pub rtt_success_count: Option<u32>,
    pub rtt_total_count: Option<u32>,
    // SMART 聚合统计
    pub smart_ok_count: Option<u64>,
    pub smart_fail_count: Option<u64>,
    pub smart_consecutive_failures: Option<u64>,
    pub smart_last_ok_ms: Option<i64>,
    pub smart_last_fail_ms: Option<i64>,
}

#[derive(Default, Debug)]
pub struct StateStore {
    tick: TickTelemetry,
    agg: Aggregated,
}

impl StateStore {
    pub fn new() -> Self { Self::default() }

    pub fn update_tick(&mut self, tick: u64, timestamp_ms: i64, tick_cost_ms: Option<u64>, frame_skipped: bool) {
        self.tick = TickTelemetry { tick, timestamp_ms, tick_cost_ms, frame_skipped };
    }

    pub fn get_tick(&self) -> TickTelemetry { self.tick.clone() }

    pub fn update_agg(&mut self, agg: Aggregated) {
        self.agg = agg;
    }

    pub fn get_agg(&self) -> Aggregated { self.agg.clone() }

    #[allow(dead_code)]
    pub fn now_ts_ms() -> i64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

