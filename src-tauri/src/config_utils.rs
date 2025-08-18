// 配置管理工具模块
// 包含应用配置的加载、保存和Tauri命令处理

use serde::{Deserialize, Serialize};
use crate::scheduler::SchedulerState;
use crate::state_store::{StateStore, TickTelemetry, Aggregated};
use tauri::{AppHandle, Manager};
use std::path::PathBuf;
// use crate::test_runner::{TestRunner, TestSummary};

/// 应用配置结构体
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AppConfig {
    // 托盘第二行显示模式："cpu" | "mem" | "fan"
    // 兼容旧字段 tray_show_mem：若为 true 则等价于 "mem"，否则为 "cpu"
    pub tray_bottom_mode: Option<String>,
    // 兼容保留（已弃用）：托盘第二行 true=显示内存%，false=显示CPU%
    pub tray_show_mem: bool,
    // 网络接口白名单：为空或缺省表示聚合全部
    pub net_interfaces: Option<Vec<String>>,
    // 公网查询开关（默认启用）。false 可关闭公网 IP/ISP 拉取
    pub public_net_enabled: Option<bool>,
    // 公网查询 API（可空使用内置：优先 ip-api.com，失败回退 ipinfo.io）
    pub public_net_api: Option<String>,
    // 多目标 RTT 配置
    pub rtt_targets: Option<Vec<String>>,   // 形如 "1.1.1.1:443"
    pub rtt_timeout_ms: Option<u64>,        // 默认 300ms
    // 集中调度：基础节拍（毫秒）。未设置默认 1000ms
    pub interval_ms: Option<u64>,
    // 集中调度：任务分频（每N个tick执行一次）
    // 多目标 RTT（默认每3tick）
    pub pace_rtt_multi_every: Option<u64>,
    // 网卡/逻辑磁盘枚举（默认每5tick）
    pub pace_net_if_every: Option<u64>,
    pub pace_logical_disk_every: Option<u64>,
    // SMART 健康（默认每10tick）
    pub pace_smart_every: Option<u64>,
    // Top 进程数量（默认 5）
    pub top_n: Option<usize>,
    // 是否启用 SMART 后台 Worker（默认启用）。false 则不启动
    pub smart_enabled: Option<bool>,
}

/// Tauri命令：获取调度器状态
#[tauri::command]
pub fn get_scheduler_state(state: tauri::State<AppState>) -> Result<SchedulerState, String> {
    state.scheduler
        .lock()
        .map(|guard| guard.clone())
        .map_err(|_| "获取调度器状态失败".to_string())
}

/// Tauri命令：获取 StateStore 的 TickTelemetry
#[tauri::command]
pub fn get_state_store_tick(state: tauri::State<AppState>) -> Result<TickTelemetry, String> {
    state.state_store
        .lock()
        .map(|guard| guard.get_tick())
        .map_err(|_| "获取 StateStore Tick 失败".to_string())
}

/// Tauri命令：获取 StateStore 的 Aggregated 聚合数据
#[tauri::command]
pub fn get_state_store_agg(state: tauri::State<AppState>) -> Result<Aggregated, String> {
    state.state_store
        .lock()
        .map(|guard| guard.get_agg())
        .map_err(|_| "获取 StateStore Aggregated 失败".to_string())
}

/// 公网信息结构体
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct PublicNetInfo {
    pub ip: Option<String>,
    pub isp: Option<String>,
    pub last_updated_ms: Option<i64>,
    pub last_error: Option<String>,
}

/// 应用状态结构体
#[derive(Clone)]
pub struct AppState {
    pub config: std::sync::Arc<std::sync::Mutex<AppConfig>>,
    #[allow(dead_code)]
    pub public_net: std::sync::Arc<std::sync::Mutex<PublicNetInfo>>,
    pub scheduler: std::sync::Arc<std::sync::Mutex<SchedulerState>>,
    pub state_store: std::sync::Arc<std::sync::Mutex<StateStore>>,
    // 新增：SMART 后台 Worker（可选）
    pub smart: Option<crate::smart_worker::SmartWorker>,
}

/// 加载应用配置
pub fn load_config(app_handle: &AppHandle) -> AppConfig {
    let config_path = get_config_path(app_handle);
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }
    AppConfig::default()
}

/// 保存应用配置
pub fn save_config(app_handle: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let config_path = get_config_path(app_handle);
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
        }
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&config_path, content).map_err(|e| format!("写入配置文件失败: {}", e))?;
    Ok(())
}

/// 获取配置文件路径
fn get_config_path(app_handle: &AppHandle) -> PathBuf {
    app_handle
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("config.json")
}

/// Tauri命令：获取配置
#[tauri::command]
pub fn get_config(state: tauri::State<AppState>) -> Result<AppConfig, String> {
    state.config
        .lock()
        .map(|guard| guard.clone())
        .map_err(|_| "获取配置失败".to_string())
}

/// Tauri命令：设置配置
#[tauri::command]
pub fn set_config(
    new_cfg: AppConfig,
    state: tauri::State<AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    // 更新内存中的配置
    if let Ok(mut guard) = state.config.lock() {
        *guard = new_cfg.clone();
    }
    // 持久化到文件
    save_config(&app_handle, &new_cfg)
}

/// 将 JSON 补丁增量合并到配置
fn apply_patch(cfg: &mut AppConfig, patch: &serde_json::Value) {
    let obj = match patch.as_object() { Some(m) => m, None => return };
    // 字符串与布尔/数值字段
    if let Some(v) = obj.get("tray_bottom_mode") { cfg.tray_bottom_mode = v.as_str().map(|s| s.to_string()); }
    if let Some(v) = obj.get("tray_show_mem") { cfg.tray_show_mem = v.as_bool().unwrap_or(cfg.tray_show_mem); }
    if let Some(v) = obj.get("public_net_enabled") { cfg.public_net_enabled = v.as_bool(); }
    if let Some(v) = obj.get("public_net_api") { cfg.public_net_api = v.as_str().map(|s| s.to_string()); }
    if let Some(v) = obj.get("rtt_timeout_ms") { cfg.rtt_timeout_ms = v.as_u64(); }
    if let Some(v) = obj.get("interval_ms") { cfg.interval_ms = v.as_u64(); }
    if let Some(v) = obj.get("pace_rtt_multi_every") { cfg.pace_rtt_multi_every = v.as_u64(); }
    if let Some(v) = obj.get("pace_net_if_every") { cfg.pace_net_if_every = v.as_u64(); }
    if let Some(v) = obj.get("pace_logical_disk_every") { cfg.pace_logical_disk_every = v.as_u64(); }
    if let Some(v) = obj.get("pace_smart_every") { cfg.pace_smart_every = v.as_u64(); }
    if let Some(v) = obj.get("top_n") { cfg.top_n = v.as_u64().map(|x| x as usize); }
    if let Some(v) = obj.get("smart_enabled") { cfg.smart_enabled = v.as_bool(); }

    // 列表字段
    if let Some(v) = obj.get("net_interfaces") {
        if v.is_null() { cfg.net_interfaces = None; }
        else if let Some(arr) = v.as_array() {
            let list: Vec<String> = arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
            cfg.net_interfaces = Some(list);
        }
    }
    if let Some(v) = obj.get("rtt_targets") {
        if v.is_null() { cfg.rtt_targets = None; }
        else if let Some(arr) = v.as_array() {
            let list: Vec<String> = arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
            cfg.rtt_targets = Some(list);
        }
    }
}

/// Tauri命令：增量热更新配置（只变更传入字段）
#[tauri::command]
pub fn cmd_cfg_update(
    patch: serde_json::Value,
    state: tauri::State<AppState>,
    app_handle: AppHandle,
) -> Result<AppConfig, String> {
    // 合并内存中的配置
    let mut merged: Option<AppConfig> = None;
    if let Ok(mut guard) = state.config.lock() {
        let mut cfg = guard.clone();
        apply_patch(&mut cfg, &patch);
        *guard = cfg.clone();
        merged = Some(cfg);
    }
    let cfg = merged.ok_or_else(|| "更新配置失败".to_string())?;
    // 持久化到文件
    save_config(&app_handle, &cfg)?;
    Ok(cfg)
}

/// Tauri命令：问候语（示例命令）
#[tauri::command]
#[allow(dead_code)]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// 列出网络接口
#[tauri::command]
pub fn list_net_interfaces() -> Vec<String> {
    // 返回空列表，实际实现可以根据需要添加
    vec![]
}

/// Tauri命令：立即触发 SMART 刷新
#[tauri::command]
pub fn smart_refresh(state: tauri::State<AppState>) -> Result<bool, String> {
    if let Some(w) = state.smart.as_ref() {
        Ok(w.request_refresh())
    } else {
        Err("SMART Worker 未初始化".to_string())
    }
}

/// Tauri命令：获取最近一次 SMART 快照（含 last_error）
#[tauri::command]
pub fn smart_get_last() -> serde_json::Value {
    crate::smart_worker::get_last_snapshot()
}

// Tauri 测试功能暂时禁用
// #[tauri::command]
// pub async fn run_tauri_tests() -> Result<TestSummary, String> {
//     let mut test_runner = TestRunner::new();
//     match test_runner.run_all_tests().await {
//         Ok(summary) => Ok(summary),
//         Err(e) => Err(format!("Tauri tests failed: {}", e)),
//     }
// }

// #[tauri::command]
// pub async fn run_bridge_tests() -> Result<String, String> {
//     // This would typically call the C# bridge test runner
//     // For now, return a placeholder
//     Ok("Bridge tests not implemented yet".to_string())
// }
/// 运行 C# 桥接层测试
#[tauri::command]
pub async fn run_bridge_tests() -> Result<serde_json::Value, String> {
    use std::process::Command;
    
    // 查找 C# 测试程序
    let bridge_dir = std::path::Path::new("sensor-bridge");
    let test_exe = bridge_dir.join("bin/Release/win-x64/publish/TestProgram.exe");
    
    if !test_exe.exists() {
        return Err("C# 测试程序不存在，请先编译 sensor-bridge 项目".to_string());
    }
    
    // 运行 C# 测试
    let output = Command::new(&test_exe)
        .current_dir(bridge_dir)
        .output()
        .map_err(|e| format!("运行C#测试失败: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("C#测试执行失败: {}", stderr));
    }
    
    // 查找最新的测试报告
    let reports = std::fs::read_dir(bridge_dir)
        .map_err(|e| format!("读取目录失败: {}", e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if name.starts_with("bridge-test-report-") && name.ends_with(".json") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    
    if let Some(latest_report) = reports.last() {
        let content = std::fs::read_to_string(latest_report)
            .map_err(|e| format!("读取测试报告失败: {}", e))?;
        
        let report: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("解析测试报告失败: {}", e))?;
        
        Ok(report)
    } else {
        Err("未找到测试报告文件".to_string())
    }
}
