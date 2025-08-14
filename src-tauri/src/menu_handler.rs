// 菜单事件处理模块
// 包含托盘菜单事件的处理逻辑

use tauri::{AppHandle, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, CustomMenuItem};
use std::sync::{Arc, Mutex, atomic::AtomicBool};

/// 创建系统托盘菜单
pub fn create_system_tray() -> SystemTray {
    let quit = CustomMenuItem::new("quit".to_string(), "退出");
    let show = CustomMenuItem::new("show".to_string(), "显示主界面");
    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(quit);
    
    SystemTray::new().with_menu(tray_menu)
}

/// 处理系统托盘事件
pub fn handle_system_tray_event(
    app: &AppHandle,
    event: SystemTrayEvent,
    shutdown_flag: Arc<AtomicBool>,
) {
    match event {
        SystemTrayEvent::LeftClick { position: _, size: _, .. } => {
            // 左键点击显示主窗口
            if let Some(window) = app.get_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        SystemTrayEvent::MenuItemClick { id, .. } => {
            match id.as_str() {
                "quit" => {
                    // 设置关闭标志
                    shutdown_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    
                    // 退出应用
                    app.exit(0);
                }
                "show" => {
                    // 显示主窗口
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}
