// 简易调度器工具：基于 tick 的节拍闸门
// 设计目标：
// - 允许在运行时调整分频（every）而不引发相位抖动
// - 与现有主循环的 sched_tick 配合：当 (tick - last) >= every 时返回 true
// - 首次可视为到期（若需要可选项控制，这里默认 tick==0 时触发一次）

#[derive(Debug, Clone)]
pub struct PacedGate {
    every: u64,
    last_tick_run: Option<u64>,
}

impl PacedGate {
    pub fn new(every: u64) -> Self {
        Self { every: every.max(1), last_tick_run: None }
    }

    pub fn set_every(&mut self, every: u64) {
        self.every = every.max(1);
    }

    // 返回本 tick 是否到期。若到期则更新 last_tick_run。
    pub fn check(&mut self, tick: u64) -> bool {
        match self.last_tick_run {
            None => {
                // 第一次调用视作到期，让上游尽快填充缓存
                self.last_tick_run = Some(tick);
                true
            }
            Some(last) => {
                if tick.saturating_sub(last) >= self.every {
                    self.last_tick_run = Some(tick);
                    true
                } else {
                    false
                }
            }
        }
    }

    // 读取当前分频
    pub fn every(&self) -> u64 { self.every }
    // 读取上次触发的 tick
    pub fn last_tick(&self) -> Option<u64> { self.last_tick_run }
}

// 对外可视化的调度状态（便于前端/调试页面查看）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SchedulerState {
    pub tick: u64,
    pub tick_cost_ms: Option<u64>,
    pub frame_skipped: bool,
    pub rtt_every: u64,
    pub rtt_last: Option<u64>,
    pub rtt_enabled: bool,
    pub rtt_is_running: bool,
    pub rtt_last_ok_ms: Option<i64>,
    pub rtt_age_ms: Option<i64>,
    pub netif_every: u64,
    pub netif_last: Option<u64>,
    pub netif_enabled: bool,
    pub netif_is_running: bool,
    pub netif_last_ok_ms: Option<i64>,
    pub netif_age_ms: Option<i64>,
    pub ldisk_every: u64,
    pub ldisk_last: Option<u64>,
    pub ldisk_enabled: bool,
    pub ldisk_is_running: bool,
    pub ldisk_last_ok_ms: Option<i64>,
    pub ldisk_age_ms: Option<i64>,
    pub smart_every: u64,
    pub smart_last: Option<u64>,
    pub smart_enabled: bool,
    pub smart_is_running: bool,
    pub smart_last_ok_ms: Option<i64>,
    pub smart_age_ms: Option<i64>,
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self {
            tick: 0,
            tick_cost_ms: None,
            frame_skipped: false,
            rtt_every: 3,
            rtt_last: None,
            rtt_enabled: true,
            rtt_is_running: false,
            rtt_last_ok_ms: None,
            rtt_age_ms: None,
            netif_every: 5,
            netif_last: None,
            netif_enabled: true,
            netif_is_running: false,
            netif_last_ok_ms: None,
            netif_age_ms: None,
            ldisk_every: 5,
            ldisk_last: None,
            ldisk_enabled: true,
            ldisk_is_running: false,
            ldisk_last_ok_ms: None,
            ldisk_age_ms: None,
            smart_every: 10,
            smart_last: None,
            smart_enabled: true,
            smart_is_running: false,
            smart_last_ok_ms: None,
            smart_age_ms: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TaskKind { Rtt, NetIf, LDisk, Smart }

#[derive(Debug, Clone)]
struct TaskEntry {
    gate: PacedGate,
    enabled: bool,
    trigger_once: bool,
    // Runner元数据
    is_running: bool,
    last_ok_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct TaskTable {
    rtt: TaskEntry,
    netif: TaskEntry,
    ldisk: TaskEntry,
    smart: TaskEntry,
}

impl Default for TaskTable {
    fn default() -> Self {
        Self {
            rtt: TaskEntry { gate: PacedGate::new(3), enabled: true, trigger_once: false, is_running: false, last_ok_ms: None },
            netif: TaskEntry { gate: PacedGate::new(5), enabled: true, trigger_once: false, is_running: false, last_ok_ms: None },
            ldisk: TaskEntry { gate: PacedGate::new(5), enabled: true, trigger_once: false, is_running: false, last_ok_ms: None },
            smart: TaskEntry { gate: PacedGate::new(10), enabled: true, trigger_once: false, is_running: false, last_ok_ms: None },
        }
    }
}

impl TaskTable {
    pub fn set_every(&mut self, kind: TaskKind, every: u64) {
        match kind {
            TaskKind::Rtt => self.rtt.gate.set_every(every),
            TaskKind::NetIf => self.netif.gate.set_every(every),
            TaskKind::LDisk => self.ldisk.gate.set_every(every),
            TaskKind::Smart => self.smart.gate.set_every(every),
        }
    }

    // 开关任务：禁用后 should_run 恒为 false
    pub fn set_enabled(&mut self, kind: TaskKind, enabled: bool) {
        match kind {
            TaskKind::Rtt => self.rtt.enabled = enabled,
            TaskKind::NetIf => self.netif.enabled = enabled,
            TaskKind::LDisk => self.ldisk.enabled = enabled,
            TaskKind::Smart => self.smart.enabled = enabled,
        }
    }

    // 一次性触发：下一个 should_run 会返回 true 并消费该标记
    pub fn trigger_once(&mut self, kind: TaskKind) {
        match kind {
            TaskKind::Rtt => self.rtt.trigger_once = true,
            TaskKind::NetIf => self.netif.trigger_once = true,
            TaskKind::LDisk => self.ldisk.trigger_once = true,
            TaskKind::Smart => self.smart.trigger_once = true,
        }
    }

    pub fn should_run(&mut self, kind: TaskKind, tick: u64) -> bool {
        let entry = match kind {
            TaskKind::Rtt => &mut self.rtt,
            TaskKind::NetIf => &mut self.netif,
            TaskKind::LDisk => &mut self.ldisk,
            TaskKind::Smart => &mut self.smart,
        };
        if !entry.enabled { return false; }
        if entry.trigger_once {
            entry.trigger_once = false;
            // 维持与正常触发一致的相位：更新 last_tick_run
            entry.gate.check(tick); // 将 last_tick_run 推进到当前 tick
            return true;
        }
        entry.gate.check(tick)
    }

    pub fn fill_state(&self, st: &mut SchedulerState, tick: u64, now_ms: i64) {
        st.tick = tick;
        // RTT
        st.rtt_every = self.rtt.gate.every();
        st.rtt_last = self.rtt.gate.last_tick();
        st.rtt_enabled = self.rtt.enabled;
        st.rtt_is_running = self.rtt.is_running;
        st.rtt_last_ok_ms = self.rtt.last_ok_ms;
        st.rtt_age_ms = self.rtt.last_ok_ms.map(|t| now_ms.saturating_sub(t));
        // NetIf
        st.netif_every = self.netif.gate.every();
        st.netif_last = self.netif.gate.last_tick();
        st.netif_enabled = self.netif.enabled;
        st.netif_is_running = self.netif.is_running;
        st.netif_last_ok_ms = self.netif.last_ok_ms;
        st.netif_age_ms = self.netif.last_ok_ms.map(|t| now_ms.saturating_sub(t));
        // LDisk
        st.ldisk_every = self.ldisk.gate.every();
        st.ldisk_last = self.ldisk.gate.last_tick();
        st.ldisk_enabled = self.ldisk.enabled;
        st.ldisk_is_running = self.ldisk.is_running;
        st.ldisk_last_ok_ms = self.ldisk.last_ok_ms;
        st.ldisk_age_ms = self.ldisk.last_ok_ms.map(|t| now_ms.saturating_sub(t));
        // Smart
        st.smart_every = self.smart.gate.every();
        st.smart_last = self.smart.gate.last_tick();
        st.smart_enabled = self.smart.enabled;
        st.smart_is_running = self.smart.is_running;
        st.smart_last_ok_ms = self.smart.last_ok_ms;
        st.smart_age_ms = self.smart.last_ok_ms.map(|t| now_ms.saturating_sub(t));
    }

    // Runner标记：开始、成功、结束
    pub fn mark_start(&mut self, kind: TaskKind) {
        let entry = match kind {
            TaskKind::Rtt => &mut self.rtt,
            TaskKind::NetIf => &mut self.netif,
            TaskKind::LDisk => &mut self.ldisk,
            TaskKind::Smart => &mut self.smart,
        };
        entry.is_running = true;
    }

    pub fn mark_ok(&mut self, kind: TaskKind, now_ms: i64) {
        let entry = match kind {
            TaskKind::Rtt => &mut self.rtt,
            TaskKind::NetIf => &mut self.netif,
            TaskKind::LDisk => &mut self.ldisk,
            TaskKind::Smart => &mut self.smart,
        };
        entry.last_ok_ms = Some(now_ms);
    }

    pub fn mark_finish(&mut self, kind: TaskKind) {
        let entry = match kind {
            TaskKind::Rtt => &mut self.rtt,
            TaskKind::NetIf => &mut self.netif,
            TaskKind::LDisk => &mut self.ldisk,
            TaskKind::Smart => &mut self.smart,
        };
        entry.is_running = false;
    }
}
