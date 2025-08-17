// 配置管理工具模块
// 包含应用配置的加载、保存和Tauri命令处理

use serde::{Deserialize, Serialize};
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
    #[allow(dead_code)]
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
