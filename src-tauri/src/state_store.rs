use std::sync::{Arc, Mutex};
use crate::types::SensorSnapshot;

#[derive(Clone)]
pub struct StateStore {
  pub inner: Arc<Mutex<Option<SensorSnapshot>>>,
}

impl Default for StateStore {
  fn default() -> Self {
    Self { inner: Arc::new(Mutex::new(None)) }
  }
}

impl StateStore {
  pub fn new() -> Self { Self::default() }

  pub fn set_latest(&self, snap: SensorSnapshot) {
    if let Ok(mut g) = self.inner.lock() {
      *g = Some(snap);
    }
  }

  pub fn get_latest(&self) -> Option<SensorSnapshot> {
    self.inner.lock().ok().and_then(|g| g.clone())
  }
}

#[tauri::command]
pub fn cmd_state_get_latest(state: tauri::State<StateStore>) -> Result<Option<SensorSnapshot>, String> {
  Ok(state.get_latest())
}
