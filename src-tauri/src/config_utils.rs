// 配置管理工具模块
// 包含应用配置的加载、保存和Tauri命令处理

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use std::path::PathBuf;

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
    // Top 进程数量（默认 5）
    pub top_n: Option<usize>,
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
    pub public_net: std::sync::Arc<std::sync::Mutex<PublicNetInfo>>,
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

/// Tauri命令：问候语（示例命令）
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// 列出网络接口
#[tauri::command]
pub fn list_net_interfaces() -> Vec<String> {
    // 返回空列表，实际实现可以根据需要添加
    vec![]
}
