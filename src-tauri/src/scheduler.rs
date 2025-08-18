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
    pub rtt_every: u64,
    pub rtt_last: Option<u64>,
    pub netif_every: u64,
    pub netif_last: Option<u64>,
    pub ldisk_every: u64,
    pub ldisk_last: Option<u64>,
    pub smart_every: u64,
    pub smart_last: Option<u64>,
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self {
            tick: 0,
            rtt_every: 3,
            rtt_last: None,
            netif_every: 5,
            netif_last: None,
            ldisk_every: 5,
            ldisk_last: None,
            smart_every: 10,
            smart_last: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TaskKind { Rtt, NetIf, LDisk, Smart }

#[derive(Debug, Clone)]
pub struct TaskTable {
    rtt: PacedGate,
    netif: PacedGate,
    ldisk: PacedGate,
    smart: PacedGate,
}

impl Default for TaskTable {
    fn default() -> Self {
        Self {
            rtt: PacedGate::new(3),
            netif: PacedGate::new(5),
            ldisk: PacedGate::new(5),
            smart: PacedGate::new(10),
        }
    }
}

impl TaskTable {
    pub fn set_every(&mut self, kind: TaskKind, every: u64) {
        match kind {
            TaskKind::Rtt => self.rtt.set_every(every),
            TaskKind::NetIf => self.netif.set_every(every),
            TaskKind::LDisk => self.ldisk.set_every(every),
            TaskKind::Smart => self.smart.set_every(every),
        }
    }

    pub fn should_run(&mut self, kind: TaskKind, tick: u64) -> bool {
        match kind {
            TaskKind::Rtt => self.rtt.check(tick),
            TaskKind::NetIf => self.netif.check(tick),
            TaskKind::LDisk => self.ldisk.check(tick),
            TaskKind::Smart => self.smart.check(tick),
        }
    }

    pub fn fill_state(&self, st: &mut SchedulerState, tick: u64) {
        st.tick = tick;
        st.rtt_every = self.rtt.every();
        st.rtt_last = self.rtt.last_tick();
        st.netif_every = self.netif.every();
        st.netif_last = self.netif.last_tick();
        st.ldisk_every = self.ldisk.every();
        st.ldisk_last = self.ldisk.last_tick();
        st.smart_every = self.smart.every();
        st.smart_last = self.smart.last_tick();
    }
}
