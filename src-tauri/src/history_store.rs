use std::fs::{OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Local;
use tauri::{AppHandle, Manager};

use crate::types::SensorSnapshot;

#[derive(Clone)]
pub struct HistoryStore {
  inner: Arc<Mutex<Vec<SensorSnapshot>>>,
  approx_bytes: Arc<Mutex<usize>>, // 近似内存占用（序列化长度累计）
  base_dir: Arc<Mutex<PathBuf>>,   // 历史落盘目录
  threshold_bytes: usize,          // 默认 50MB
}

impl HistoryStore {
  pub fn new(app: &AppHandle) -> Self {
    let dir = app
      .path()
      .app_data_dir()
      .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
      .join("history");
    if !dir.exists() { let _ = std::fs::create_dir_all(&dir); }
    Self {
      inner: Arc::new(Mutex::new(Vec::with_capacity(4096))),
      approx_bytes: Arc::new(Mutex::new(0)),
      base_dir: Arc::new(Mutex::new(dir)),
      threshold_bytes: 50 * 1024 * 1024,
    }
  }

  pub fn push(&self, snap: SensorSnapshot) {
    // 估算新增大小
    let json = match serde_json::to_string(&snap) { Ok(s) => s, Err(_) => return };
    let add = json.len() + 1; // 包含换行
    if let Ok(mut v) = self.inner.lock() {
      v.push(snap);
    }
    if let Ok(mut b) = self.approx_bytes.lock() { *b += add; }
    // 阈值检查
    if self.bytes() >= self.threshold_bytes { let _ = self.flush_to_disk(json.into_bytes()); } // 先写本条，避免丢失
  }

  fn bytes(&self) -> usize { self.approx_bytes.lock().ok().map(|g| *g).unwrap_or(0) }

  fn today_file(&self) -> PathBuf {
    let ymd = Local::now().format("%Y%m%d").to_string();
    self.base_dir.lock().unwrap().join(format!("{}.jsonl", ymd))
  }

  fn flush_to_disk(&self, last_line: Vec<u8>) -> Result<(), String> {
    // 取出当前缓冲并清零计数
    let mut lines: Vec<String> = Vec::new();
    if let Ok(mut g) = self.inner.lock() {
      lines.reserve(g.len());
      for s in g.drain(..) {
        match serde_json::to_string(&s) { Ok(j) => lines.push(j), Err(_) => {} }
      }
    }
    // 将触发 push 的最后一条也写入
    let mut f = OpenOptions::new().create(true).append(true).open(self.today_file())
      .map_err(|e| format!("打开历史文件失败: {}", e))?;
    for l in lines { let _ = f.write_all(l.as_bytes()); let _ = f.write_all(b"\n"); }
    let _ = f.write_all(&last_line); let _ = f.write_all(b"\n");
    if let Ok(mut b) = self.approx_bytes.lock() { *b = 0; }
    Ok(())
  }

  pub fn query_memory(&self, from_ts: i64, to_ts: i64, limit: usize) -> Vec<SensorSnapshot> {
    let mut out = Vec::new();
    if let Ok(g) = self.inner.lock() {
      for s in g.iter().rev() { // 逆序以尽快达到 limit
        if s.timestamp_ms >= from_ts && s.timestamp_ms <= to_ts { out.push(s.clone()); }
        if out.len() >= limit { break; }
      }
    }
    out.reverse();
    out
  }

  pub fn query_today_file(&self, from_ts: i64, to_ts: i64, limit: usize) -> Vec<SensorSnapshot> {
    let mut out = Vec::new();
    let path = self.today_file();
    if !path.exists() { return out; }
    if let Ok(txt) = std::fs::read_to_string(path) {
      for line in txt.lines().rev() { // 逆序
        if let Ok(s) = serde_json::from_str::<SensorSnapshot>(line) {
          if s.timestamp_ms >= from_ts && s.timestamp_ms <= to_ts { out.push(s); }
          if out.len() >= limit { break; }
        }
      }
    }
    out.reverse();
    out
  }
}

#[tauri::command]
pub fn cmd_history_query(
  from_ts: i64,
  to_ts: i64,
  limit: Option<usize>,
  state: tauri::State<HistoryStore>,
) -> Result<Vec<SensorSnapshot>, String> {
  let lim = limit.unwrap_or(2000).min(50_000);
  let mut v = state.query_memory(from_ts, to_ts, lim);
  if v.len() < lim {
    let remain = lim - v.len();
    let mut f = state.query_today_file(from_ts, to_ts, remain);
    v.append(&mut f);
  }
  Ok(v)
}
