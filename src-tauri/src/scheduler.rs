use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct TaskTable {
  pub tick: u64,
  pub pace_rtt_multi_every: u64,
  pub pace_net_if_every: u64,
  pub pace_logical_disk_every: u64,
  pub pace_smart_every: u64,
}

impl TaskTable {
  pub fn new(pace_rtt_multi_every: u64, pace_net_if_every: u64, pace_logical_disk_every: u64, pace_smart_every: u64) -> Self {
    Self { tick: 0, pace_rtt_multi_every, pace_net_if_every, pace_logical_disk_every, pace_smart_every }
  }
  #[inline] pub fn inc(&mut self) { self.tick = self.tick.wrapping_add(1); }
  #[inline] pub fn due_every(&self, n: u64) -> bool { n > 0 && (self.tick % n == 0) }
}
