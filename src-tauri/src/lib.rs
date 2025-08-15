// ================================================================================
// 系统传感器监控应用 - 主模块
// ================================================================================
// 
// 本文件包含以下功能区域：
// 1. Tauri 命令函数
// 2. 前端数据结构定义 (Payload 结构体)
// 3. WMI 查询结构体定义
// 4. WMI 查询函数实现
// 5. 网络工具函数
// 6. SMART 硬盘监控函数
// 7. 托盘图标渲染函数
// 8. 主程序逻辑和数据采集循环
//
// ================================================================================

// 模块导入
mod battery_utils;
mod thermal_utils;
mod network_disk_utils;
mod gpu_utils;
mod smart_utils;
mod process_utils;
mod wifi_utils;
mod nvme_smart_utils;
mod tray_graphics_utils;
mod config_utils;
mod types;
mod wmi_utils;
mod power_utils;
mod public_net_utils;
mod bridge_manager;
mod menu_handler;
mod nvme_ioctl_utils;
mod powershell_utils;
mod smartctl_utils;
mod bridge_types;

// 导入各模块的公共类型和函数
use smart_utils::{wmi_list_smart_status, wmi_fallback_disk_status};
use process_utils::*;
use wifi_utils::*;
use types::*;
use config_utils::*;
use wmi_utils::*;
use bridge_types::{SensorSnapshot, read_power_status};
use gpu_utils::wmi_query_gpu_vram;
use powershell_utils::nvme_storage_reliability_ps;

// ================================================================================
// 1. TAURI 命令函数
// ================================================================================

// greet 命令已移至 config_utils 模块

// ================================================================================
// 2. 前端数据结构定义 (PAYLOAD 结构体)
// ================================================================================
// 所有 Payload 结构体已移至 types.rs 模块

// ================================================================================
// 3. WMI 查询结构体定义
// ================================================================================
// 所有 WMI 查询结构体已移至 types.rs 模块

// ================================================================================
// 4. WMI 查询函数实现
// ================================================================================
// 所有 WMI 查询函数已移至 wmi_utils.rs 模块

// tcp_rtt_ms 函数已移至 process_utils 模块

// decode_console_bytes 函数已移至 wmi_utils 模块

// Wi-Fi相关函数已移至 wifi_utils 模块
// 温度和风扇相关结构体已移至 thermal_utils 模块

// PerfTcpipNic 结构体已移至 types.rs 模块

// PerfOsMemory 结构体已移至 types.rs 模块

// GPU WMI 查询结构体已移至 gpu_utils 模块

// 电池相关结构体和函数已移至 battery_utils 模块

// SMART相关结构体已移至 smart_utils 模块

// 电池相关函数已移至 battery_utils 模块

// 温度和风扇相关函数已移至 thermal_utils 模块

// ---- WMI helpers: network interfaces, logical disks, SMART status ----

// SMART属性解析函数已移至 smart_utils 模块

// wmi_list_net_ifs 函数已移至 network_disk_utils 模块

// wmi_list_logical_disks 函数已移至 network_disk_utils 模块

// wmi_list_smart_status 函数已移至 smart_utils 模块

// nvme_smart_via_ioctl 函数已移至 nvme_smart_utils 模块

// nvme_get_health_via_protocol_command 函数已移至 nvme_ioctl_utils 模块
fn nvme_get_health_via_protocol_command(handle: windows::Win32::Foundation::HANDLE, path: &str) -> Option<SmartHealthPayload> {
    nvme_ioctl_utils::nvme_get_health_via_protocol_command(handle, path)
}

#[cfg(not(windows))]
fn nvme_smart_via_ioctl() -> Option<Vec<SmartHealthPayload>> { None }


// nvme_storage_reliability_ps 函数已移至 powershell_utils 模块

#[cfg(not(windows))]
fn nvme_storage_reliability_ps() -> Option<Vec<SmartHealthPayload>> { None }
// smartctl_collect 函数已移至 smartctl_utils 模块
#[cfg(windows)]
fn smartctl_collect() -> Option<Vec<SmartHealthPayload>> {
    smartctl_utils::smartctl_collect()
}

#[cfg(not(windows))]
fn smartctl_collect() -> Option<Vec<SmartHealthPayload>> { None }

// wmi_query_gpu_vram 函数已移至 gpu_utils 模块

// ---- Realtime snapshot payload for frontend ----
// read_power_status 函数已移至 bridge_types 模块

// SensorSnapshot 结构体已移至 bridge_types 模块

// RttResultPayload、TopProcessPayload、BridgeFan、BridgeVoltage 等结构体已移至对应模块





#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::thread;
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        image::Image,
        Emitter,
        tray::TrayIconBuilder,

        Manager,
    };

    use tauri::path::BaseDirectory;

    // 使用模块中的类型定义
    use crate::types::BridgeOut;
    use crate::config_utils::{AppConfig, PublicNetInfo, AppState};

    // 使用模块中的配置相关函数
    use crate::config_utils::{load_config, get_config, set_config};
    use crate::menu_handler::setup_menu_handlers;
    use crate::bridge_manager::start_bridge_manager;
    use crate::public_net_utils::start_public_net_polling;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_config, set_config, list_net_interfaces])
        .setup(|app| {
            use tauri::WindowEvent;
            // Windows 下：启动时自动检测管理员权限，若非管理员则尝试以管理员身份重启并退出当前进程
            // 但在开发模式（debug 或存在 TAURI_DEV_SERVER_URL）下禁用自动提权，避免断开 tauri dev server 导致 localhost 拒绝连接。
            #[cfg(windows)]
            {
                let is_dev_mode = cfg!(debug_assertions) || std::env::var("TAURI_DEV_SERVER_URL").is_ok();
                if !is_dev_mode {
                    let is_admin = {
                        let mut cmd = std::process::Command::new("powershell");
                        cmd.args([
                            "-NoProfile",
                            "-Command",
                            "([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
                        ]);
                        #[cfg(windows)]
                        {
                            use std::os::windows::process::CommandExt;
                            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                        }
                        cmd.output()
                    }
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|s| s.trim().eq_ignore_ascii_case("True"))
                        .unwrap_or(false);
                    if !is_admin {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = {
                                let mut cmd = std::process::Command::new("powershell");
                                cmd.args([
                                    "-NoProfile",
                                    "-Command",
                                    &format!("Start-Process -FilePath '{}' -Verb runas", exe.display()),
                                ]);
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                                }
                                cmd.spawn()
                            };
                        }
                        eprintln!("[sys-sensor] 正在请求管理员权限运行，请在UAC中确认...");
                        std::process::exit(0);
                    }
                }
            }
            // 为已存在的主窗口（label: "main"）注册关闭->隐藏处理
            if let Some(main_win) = app.get_webview_window("main") {
                let main_win_c = main_win.clone();
                let _ = main_win.on_window_event(move |e| {
                    if let WindowEvent::CloseRequested { api, .. } = e {
                        let _ = main_win_c.hide();
                        api.prevent_close();
                    }
                });
            }

            use std::sync::{Arc, Mutex};
            use std::time::Instant as StdInstant;
            // --- Build non-clickable info area as disabled menu items ---
            let info_cpu = MenuItem::with_id(app, "info_cpu", "CPU: —", false, None::<&str>)?;
            let info_mem = MenuItem::with_id(app, "info_mem", "内存: —", false, None::<&str>)?;
            let info_temp = MenuItem::with_id(app, "info_temp", "温度: —", false, None::<&str>)?;
            let info_fan = MenuItem::with_id(app, "info_fan", "风扇: —", false, None::<&str>)?;
            let info_net = MenuItem::with_id(app, "info_net", "网络: —", false, None::<&str>)?;
            let info_public = MenuItem::with_id(app, "info_public", "公网: —", false, None::<&str>)?;
            let info_disk = MenuItem::with_id(app, "info_disk", "磁盘: —", false, None::<&str>)?;
            let info_store = MenuItem::with_id(app, "info_store", "存储: —", false, None::<&str>)?;
            let info_gpu = MenuItem::with_id(app, "info_gpu", "GPU: —", false, None::<&str>)?;
            let info_bridge = MenuItem::with_id(app, "info_bridge", "桥接: —", false, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;

            // --- Clickable action items ---
            let show_details = MenuItem::with_id(app, "show_details", "显示详情", true, None::<&str>)?;
            let quick_settings = MenuItem::with_id(app, "quick_settings", "快速设置", true, None::<&str>)?;
            let about = MenuItem::with_id(app, "about", "关于我们", true, None::<&str>)?;
            // 调试：复制全部托盘数据到剪贴板
            let debug_copy = MenuItem::with_id(app, "debug_copy_all", "[debug] 复制全部数据", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?;

            // 初始化配置与公网缓存，并注入状态
            let cfg_arc: Arc<Mutex<AppConfig>> = Arc::new(Mutex::new(load_config(&app.handle())));
            let pub_net_arc: Arc<Mutex<PublicNetInfo>> = Arc::new(Mutex::new(PublicNetInfo::default()));
            app.manage(AppState { config: cfg_arc.clone(), public_net: pub_net_arc.clone() });

            let menu = Menu::with_items(
                app,
                &[
                    &info_cpu,
                    &info_mem,
                    &info_temp,
                    &info_fan,
                    &info_net,
                    &info_public,
                    &info_disk,
                    &info_gpu,
                    &info_store,
                    &info_bridge,
                    &sep,
                    &show_details,
                    &quick_settings,
                    &about,
                    &debug_copy,
                    &exit,
                ],
            )?;

            // --- Create tray icon ---
            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("系统监控 - 初始化中...");

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let tray = tray_builder.build(app)?;
            let app_handle = app.handle();
            // 预计算打包资源中的桥接可执行文件路径（如存在，优先使用）
            let packaged_bridge_exe = app_handle
                .path()
                .resolve("sensor-bridge/sensor-bridge.exe", BaseDirectory::Resource)
                .ok();

            // 退出控制与子进程 PID 记录（用于退出时清理）
            let shutdown_flag: Arc<std::sync::atomic::AtomicBool> = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let bridge_pid: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
            // 最近一次的汇总文本（用于 [debug] 复制）
            let last_info_text: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

            // --- Spawn sensor-bridge (.NET) and share latest output ---
            let bridge_data: Arc<Mutex<(Option<BridgeOut>, StdInstant)>> = Arc::new(Mutex::new((None, StdInstant::now())));
            start_bridge_manager(bridge_data.clone(), packaged_bridge_exe, shutdown_flag.clone(), bridge_pid.clone());

            // --- 公网 IP/ISP 后台轮询线程 ---
            start_public_net_polling(cfg_arc.clone(), pub_net_arc.clone());

            // --- Handle menu events ---
            let app_handle_menu = app_handle.clone();
            let last_info_text_menu = last_info_text.clone();
            let shutdown_flag_menu = shutdown_flag.clone();
            tray.on_menu_event(move |app, event| {
                match event.id.as_ref() {
                    "show_details" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            // 发送事件到前端导航到主页
                            let _ = window.emit("navigate-to-home", ());
                        }
                    }
                    "quick_settings" => {
                        // 打开快速设置对话框或页面
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            // 可以发送事件到前端切换到设置页面
                            let _ = window.emit("navigate-to-settings", ());
                        }
                    }
                    "about" => {
                        // 显示关于对话框
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("show-about", ());
                        }
                    }
                    "debug_copy_all" => {
                        // 复制调试信息到剪贴板
                        if let Ok(info_text) = last_info_text_menu.lock() {
                            let clipboard_text = format!("系统监控调试信息:\n{}", info_text.as_str());
                            // 尝试复制到剪贴板
                            #[cfg(windows)]
                            {
                                use std::process::Command;
                                let mut cmd = Command::new("powershell");
                                cmd.args([
                                    "-NoProfile", 
                                    "-Command", 
                                    &format!("Set-Clipboard -Value '{}'", clipboard_text.replace("'", "''"))
                                ]);
                                let _ = cmd.output();
                            }
                        }
                    }
                    "exit" => {
                        shutdown_flag_menu.store(true, std::sync::atomic::Ordering::Relaxed);
                        std::process::exit(0);
                    }
                    _ => {}
                }
            });

            // --- Spawn background refresh thread (1s) ---
            let info_cpu_c = info_cpu.clone();
            let info_mem_c = info_mem.clone();
            let info_temp_c = info_temp.clone();
            let info_fan_c = info_fan.clone();
            let info_net_c = info_net.clone();
            let info_disk_c = info_disk.clone();
            let info_store_c = info_store.clone();
            let info_gpu_c = info_gpu.clone();
            let info_bridge_c = info_bridge.clone();
            let info_public_c = info_public.clone();
            let tray_c = tray.clone();
            let app_handle_c = app_handle.clone();
            let bridge_data_sampling = bridge_data.clone();
            let cfg_state_c = cfg_arc.clone();
            let pub_net_c = pub_net_arc.clone();
            let last_info_text_c = last_info_text.clone();

            thread::spawn(move || {
                use std::time::{Duration, Instant};
                use sysinfo::{Networks, System};

                // 初始化 WMI 连接（在后台线程中初始化 COM）
                let mut wmi_temp_conn: Option<wmi::WMIConnection> = {
                    if let Ok(com) = wmi::COMLibrary::new() {
                        wmi::WMIConnection::with_namespace_path("ROOT\\WMI", com.into()).ok()
                    } else { None }
                };
                let mut wmi_fan_conn: Option<wmi::WMIConnection> = {
                    if let Ok(com) = wmi::COMLibrary::new() {
                        wmi::WMIConnection::new(com).ok() // 默认 ROOT\CIMV2
                    } else { None }
                };
                let mut wmi_perf_conn: Option<wmi::WMIConnection> = {
                    if let Ok(com) = wmi::COMLibrary::new() {
                        wmi::WMIConnection::new(com).ok() // ROOT\CIMV2: PerfFormattedData
                    } else { None }
                };

                // --- sysinfo contexts ---
                let mut sys = System::new_all();
                let mut networks = Networks::new_with_refreshed_list();

                // 初次刷新以建立基线
                sys.refresh_cpu_usage();
                sys.refresh_memory();

                // 累计计数与 EMA
                let mut last_net_rx: u64 = 0;
                let mut last_net_tx: u64 = 0;
                let mut last_disk_r: u64 = 0;
                let mut last_disk_w: u64 = 0;
                let mut last_t = Instant::now();
                let alpha = 0.3f64;
                let mut ema_net_rx: f64 = 0.0;
                let mut ema_net_tx: f64 = 0.0;
                let mut ema_disk_r: f64 = 0.0;
                let mut ema_disk_w: f64 = 0.0;
                let mut has_prev = false;
                let mut last_bridge_fresh: Option<bool> = None;
                // WMI 健壮性：失败计数与周期重开
                let mut wmi_fail_perf: u32 = 0;
                let mut last_wmi_reopen = Instant::now();

                // 单位格式化（bytes/s -> KB/s 或 MB/s）
                let fmt_bps = |bps: f64| -> String {
                    let kbps = bps / 1024.0;
                    if kbps < 1024.0 {
                        format!("{:.1} KB/s", kbps)
                    } else {
                        format!("{:.1} MB/s", kbps / 1024.0)
                    }
                };

                loop {
                    // 刷新数据
                    sys.refresh_cpu_usage();
                    sys.refresh_memory();
                    let _ = networks.refresh();
                    sys.refresh_processes();

                    // CPU 使用率（0~100）
                    let cpu_usage = sys.global_cpu_info().cpu_usage();
                    // 内存（以字节为单位读取后格式化为 GB）
                    let used = sys.used_memory() as f64;
                    let total = sys.total_memory() as f64;
                    let mem_pct = if total > 0.0 { (used / total) * 100.0 } else { 0.0 };
                    let used_gb = used / 1073741824.0; // 1024^3
                    let total_gb = total / 1073741824.0;
                    let avail = sys.available_memory() as f64;
                    let avail_gb = avail / 1073741824.0;
                    let swap_total = sys.total_swap() as f64;
                    let swap_used = sys.used_swap() as f64;
                    let swap_total_gb = swap_total / 1073741824.0;
                    let swap_used_gb = swap_used / 1073741824.0;

                    // --- 网络累计字节合计（可按配置过滤接口）---
                    let (net_rx_total, net_tx_total): (u64, u64) = {
                        let selected: Option<Vec<String>> = cfg_state_c
                            .lock().ok()
                            .and_then(|c| c.net_interfaces.clone())
                            .filter(|v| !v.is_empty());
                        if let Some(allow) = selected {
                            let mut rx = 0u64; let mut tx = 0u64;
                            for (name, data) in &networks {
                                if allow.iter().any(|n| n == name) {
                                    rx = rx.saturating_add(data.total_received());
                                    tx = tx.saturating_add(data.total_transmitted());
                                }
                            }
                            (rx, tx)
                        } else {
                            let mut rx = 0u64; let mut tx = 0u64;
                            for (_, data) in &networks {
                                rx = rx.saturating_add(data.total_received());
                                tx = tx.saturating_add(data.total_transmitted());
                            }
                            (rx, tx)
                        }
                    };

                    // --- 磁盘累计字节合计（按进程聚合）---
                    let (disk_r_total, disk_w_total) = calculate_disk_totals(&sys);

                    // 计算速率（bytes/s）
                    let now = Instant::now();
                    let dt = now.duration_since(last_t).as_secs_f64().max(1e-6);
                    // 若系统经历了睡眠/长间隔（>5s），重置速率基线并尝试重建 WMI 连接
                    let slept = dt > 5.0;
                    if slept {
                        // 重置 EMA 基线：跳过本次差分，下一轮重新建立基线
                        has_prev = false;
                        // 重建 WMI 连接（分别初始化，避免单次失败影响全部）
                        if let Ok(com) = wmi::COMLibrary::new() {
                            wmi_temp_conn = wmi::WMIConnection::with_namespace_path("ROOT\\WMI", com.into()).ok();
                        }
                        if let Ok(com) = wmi::COMLibrary::new() {
                            wmi_fan_conn = wmi::WMIConnection::new(com).ok();
                        }
                        if let Ok(com) = wmi::COMLibrary::new() {
                            wmi_perf_conn = wmi::WMIConnection::new(com).ok();
                        }
                        last_wmi_reopen = Instant::now();
                        eprintln!("[wmi][reopen] due to long gap {:.1}s (sleep/resume?)", dt);
                    }
                    let mut net_rx_rate = 0.0;
                    let mut net_tx_rate = 0.0;
                    let mut disk_r_rate = 0.0;
                    let mut disk_w_rate = 0.0;
                    if has_prev {
                        net_rx_rate = (net_rx_total.saturating_sub(last_net_rx)) as f64 / dt;
                        net_tx_rate = (net_tx_total.saturating_sub(last_net_tx)) as f64 / dt;
                        disk_r_rate = (disk_r_total.saturating_sub(last_disk_r)) as f64 / dt;
                        disk_w_rate = (disk_w_total.saturating_sub(last_disk_w)) as f64 / dt;
                    }

                    // EMA 平滑
                    if !has_prev {
                        ema_net_rx = net_rx_rate;
                        ema_net_tx = net_tx_rate;
                        ema_disk_r = disk_r_rate;
                        ema_disk_w = disk_w_rate;
                        has_prev = true;
                    } else {
                        ema_net_rx = alpha * net_rx_rate + (1.0 - alpha) * ema_net_rx;
                        ema_net_tx = alpha * net_tx_rate + (1.0 - alpha) * ema_net_tx;
                        ema_disk_r = alpha * disk_r_rate + (1.0 - alpha) * ema_disk_r;
                        ema_disk_w = alpha * disk_w_rate + (1.0 - alpha) * ema_disk_w;
                    }

                    // 保存本次累计与时间
                    last_net_rx = net_rx_total;
                    last_net_tx = net_tx_total;
                    last_disk_r = disk_r_total;
                    last_disk_w = disk_w_total;
                    last_t = now;

                    // 读取第二梯队：磁盘 IOPS/队列、网络错误、RTT
                    let (disk_r_iops, disk_w_iops, disk_queue_len) = match &wmi_perf_conn {
                        Some(c) => wmi_perf_disk(c),
                        None => (None, None, None),
                    };
                    let (net_rx_err_ps, net_tx_err_ps, packet_loss_pct, discarded_recv, discarded_sent) = match &wmi_perf_conn {
                        Some(c) => wmi_perf_net_err(c),
                        None => (None, None, None, None, None),
                    };
                    
                    // 获取活动网络连接数
                    let active_connections = wmi_utils::get_active_connections();
                    let (mem_cache_gb, mem_committed_gb, mem_commit_limit_gb, mem_pool_paged_gb, mem_pool_nonpaged_gb, 
                         mem_pages_per_sec, mem_page_reads_per_sec, mem_page_writes_per_sec, mem_page_faults_per_sec) = match &wmi_perf_conn {
                        Some(c) => wmi_perf_memory(c),
                        None => (None, None, None, None, None, None, None, None, None),
                    };
                    let ping_rtt_ms = tcp_rtt_ms("1.1.1.1:443", 300);

                    // 多目标 RTT（顺序串行测量）
                    let rtt_multi: Option<Vec<RttResultPayload>> = {
                        let timeout = cfg_state_c
                            .lock().ok()
                            .and_then(|c| c.rtt_timeout_ms)
                            .unwrap_or(300);
                        let targets = cfg_state_c
                            .lock().ok()
                            .and_then(|c| c.rtt_targets.clone())
                            .unwrap_or_else(|| vec![
                                "1.1.1.1:443".to_string(),
                                "8.8.8.8:443".to_string(),
                                "114.114.114.114:53".to_string(),
                            ]);
                        measure_multi_rtt(&targets, timeout)
                    };

                    // Top 进程（CPU 与内存）
                    let top_n = cfg_state_c
                        .lock().ok()
                        .and_then(|c| c.top_n)
                        .unwrap_or(5);
                    let (top_cpu_procs, top_mem_procs) = get_top_processes(&sys, top_n);
                    // 根据查询结果更新失败计数并在需要时重建 WMI Perf 连接
                    if wmi_perf_conn.is_some()
                        && disk_r_iops.is_none()
                        && disk_w_iops.is_none()
                        && disk_queue_len.is_none()
                        && net_rx_err_ps.is_none()
                        && net_tx_err_ps.is_none() {
                        wmi_fail_perf = wmi_fail_perf.saturating_add(1);
                    } else {
                        wmi_fail_perf = 0;
                    }
                    if wmi_fail_perf >= 3 || last_wmi_reopen.elapsed().as_secs() >= 1800 {
                        if let Ok(com) = wmi::COMLibrary::new() {
                            wmi_perf_conn = wmi::WMIConnection::new(com).ok();
                            eprintln!(
                                "[wmi][reopen] perf conn recreated (fail_cnt={}, periodic={})",
                                wmi_fail_perf,
                                (last_wmi_reopen.elapsed().as_secs() >= 1800)
                            );
                            wmi_fail_perf = 0;
                            last_wmi_reopen = Instant::now();
                        }
                    }

                    // 组织显示文本
                    let cpu_line = format!("CPU: {:.0}%", cpu_usage);
                    let mem_line = format!("内存: {:.1}/{:.1} GB ({:.0}%)", used_gb, total_gb, mem_pct);
                    // 读取温度与风扇（优先桥接数据，其次 WMI）
                    let (
                        bridge_cpu_temp,
                        bridge_mobo_temp,
                        bridge_cpu_fan,
                        case_fan,
                        bridge_cpu_fan_pct,
                        case_fan_pct,
                        is_admin,
                        has_temp,
                        has_temp_value,
                        has_fan,
                        has_fan_value,
                        storage_temps,
                        gpus,
                        mobo_voltages,
                        fans_extra,
                        battery_percent,
                        battery_status,
                        battery_design_capacity,
                        battery_full_charge_capacity,
                        battery_cycle_count,
                        battery_ac_online,
                        battery_time_remaining_sec,
                        battery_time_to_full_sec,
                        hb_tick,
                        idle_sec,
                        exc_count,
                        uptime_sec,
                        cpu_pkg_power_w,
                        cpu_avg_freq_mhz,
                        cpu_throttle_active,
                        cpu_throttle_reasons,
                        since_reopen_sec,
                        cpu_core_loads_pct,
                        cpu_core_clocks_mhz,
                        cpu_core_temps_c,
                    ) = {
                        let mut cpu_t: Option<f32> = None;
                        let mut mobo_t: Option<f32> = None;
                        let mut cpu_fan: Option<u32> = None;
                        let mut case_fan: Option<u32> = None;
                        let mut cpu_fan_pct: Option<u32> = None;
                        let mut case_fan_pct: Option<u32> = None;
                        let mut is_admin: Option<bool> = None;
                        let mut has_temp: Option<bool> = None;
                        let mut has_temp_value: Option<bool> = None;
                        let mut has_fan: Option<bool> = None;
                        let mut has_fan_value: Option<bool> = None;
                        let mut storage_temps: Option<Vec<StorageTempPayload>> = None;
                        let mut gpus: Option<Vec<GpuPayload>> = None;
                        let mut mobo_voltages: Option<Vec<VoltagePayload>> = None;
                        let mut fans_extra: Option<Vec<FanPayload>> = None;
                        let mut battery_percent: Option<i32> = None;
                        let mut battery_status: Option<String> = None;
                        let mut battery_ac_online: Option<bool> = None;
                        let mut battery_time_remaining_sec: Option<i32> = None;
                        let mut battery_time_to_full_sec: Option<i32> = None;
                        let mut hb_tick: Option<i64> = None;
                        let mut idle_sec: Option<i32> = None;
                        let mut exc_count: Option<i32> = None;
                        let mut uptime_sec: Option<i32> = None;
                        let mut cpu_pkg_power_w: Option<f64> = None;
                        let mut cpu_avg_freq_mhz: Option<f64> = None;
                        let mut cpu_throttle_active: Option<bool> = None;
                        let mut cpu_throttle_reasons: Option<Vec<String>> = None;
                        let mut since_reopen_sec: Option<i32> = None;
                        let mut cpu_core_loads_pct: Option<Vec<Option<f32>>> = None;
                        let mut cpu_core_clocks_mhz: Option<Vec<Option<f64>>> = None;
                        let mut cpu_core_temps_c: Option<Vec<Option<f32>>> = None;
                        let mut fresh_now: Option<bool> = None;
                        if let Ok(guard) = bridge_data_sampling.lock() {
                            if let (Some(ref b), ts) = (&guard.0, guard.1) {
                                // 若超过 30s 未更新则视为过期（原为 5s）。
                                // 现场发现：桥接在长时间运行、系统休眠/杀软打扰、或桥接短暂重启期间，输出间隔可能>5s，
                                // 过低阈值会导致误判为过期，从而丢弃桥接温度/风扇数据（WMI 又常无值），UI 显示“—”。
                                if ts.elapsed().as_secs() <= 30 {
                                    fresh_now = Some(true);
                                    cpu_t = b.cpu_temp_c;
                                    mobo_t = b.mobo_temp_c;
                                    is_admin = b.is_admin;
                                    has_temp = b.has_temp;
                                    has_temp_value = b.has_temp_value;
                                    has_fan = b.has_fan;
                                    has_fan_value = b.has_fan_value;
                                    // 存储温度
                                    if let Some(st) = &b.storage_temps {
                                        let mapped: Vec<StorageTempPayload> = st.iter().map(|x| StorageTempPayload {
                                            name: x.name.clone(),
                                            temp_c: x.temp_c,
                                            drive_letter: None, // 初始为空，后续会合并smartctl数据
                                        }).collect();
                                        if !mapped.is_empty() { storage_temps = Some(mapped); }
                                    }

                                    // 合并 smartctl 采集的盘符数据到 storage_temps
                                    if let Some(smartctl_data) = smartctl_utils::smartctl_collect() {
                                        eprintln!("[SMARTCTL_MERGE] 开始合并smartctl盘符数据，共{}条", smartctl_data.len());
                                        
                                        // 如果没有storage_temps，创建新的
                                        if storage_temps.is_none() {
                                            storage_temps = Some(Vec::new());
                                        }
                                        
                                        if let Some(ref mut st_list) = storage_temps {
                                            // 为现有的storage_temps条目匹配盘符
                                            for st_item in st_list.iter_mut() {
                                                if let Some(st_name) = &st_item.name {
                                                    // 尝试匹配smartctl数据中的设备名
                                                    for smart_item in &smartctl_data {
                                                        if let Some(smart_device) = &smart_item.device {
                                                            // 匹配逻辑：名称包含关系或设备路径匹配
                                                            if st_name.contains(smart_device) || smart_device.contains(st_name) {
                                                                st_item.drive_letter = smart_item.drive_letter.clone();
                                                                eprintln!("[SMARTCTL_MERGE] 匹配成功: {} -> {:?}", st_name, smart_item.drive_letter);
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            // 添加smartctl独有的设备（如果storage_temps中没有对应条目）
                                            for smart_item in &smartctl_data {
                                                let device_name = smart_item.device.clone().unwrap_or_else(|| "未知设备".to_string());
                                                let already_exists = st_list.iter().any(|st| {
                                                    if let Some(st_name) = &st.name {
                                                        st_name.contains(&device_name) || device_name.contains(st_name)
                                                    } else {
                                                        false
                                                    }
                                                });
                                                
                                                if !already_exists {
                                                    st_list.push(StorageTempPayload {
                                                        name: Some(device_name.clone()),
                                                        temp_c: smart_item.temp_c,
                                                        drive_letter: smart_item.drive_letter.clone(),
                                                    });
                                                    eprintln!("[SMARTCTL_MERGE] 添加新设备: {} -> {:?}", device_name, smart_item.drive_letter);
                                                }
                                            }
                                        }
                                    } else {
                                        eprintln!("[SMARTCTL_MERGE] smartctl数据采集失败，尝试基于WMI的盘符映射");
                                        
                                        // 如果smartctl失败，使用基于WMI的盘符映射（兼容无smartctl环境）
                                        if let Some(ref mut st_list) = storage_temps {
                                            for (index, st_item) in st_list.iter_mut().enumerate() {
                                                if st_item.drive_letter.is_none() {
                                                    // 基于索引的简单映射
                                                    let drive_letter = match index {
                                                        0 => Some("C:".to_string()),
                                                        1 => Some("D:".to_string()),
                                                        2 => Some("E:".to_string()),
                                                        3 => Some("F:".to_string()),
                                                        _ => Some(format!("磁盘{}", index)),
                                                    };
                                                    st_item.drive_letter = drive_letter.clone();
                                                    eprintln!("[SMARTCTL_MERGE] 默认映射: 索引{} -> {:?}", index, drive_letter);
                                                }
                                            }
                                        }
                                    }

                                    // GPU 列表
                                    if let Some(gg) = &b.gpus {
                                        eprintln!("[BRIDGE_GPU_DEBUG] Received {} GPUs from bridge", gg.len());
                                        for (i, gpu) in gg.iter().enumerate() {
                                            eprintln!("[BRIDGE_GPU_DEBUG] GPU {}: name={:?} vram_used_mb={:?} power_w={:?} temp_c={:?} load_pct={:?}", 
                                                i, gpu.name, gpu.vram_used_mb, gpu.power_w, gpu.temp_c, gpu.load_pct);
                                        }
                                        
                                        // 查询 GPU 显存信息
                                        let gpu_vram_info = match &wmi_perf_conn {
                                            Some(c) => wmi_query_gpu_vram(c),
                                            None => Vec::new(),
                                        };
                                        
                                        let mapped: Vec<GpuPayload> = gg.iter().map(|x| {
                                            // 尝试匹配 GPU 名称获取显存信息
                                            eprintln!("[GPU_MAPPING] Processing GPU from bridge: name={:?}", x.name);
                                            eprintln!("[GPU_MAPPING] Available VRAM info: {:?}", gpu_vram_info);
                                            
                                            let (vram_total_mb, vram_usage_pct) = if let Some(gpu_name) = &x.name {
                                                if let Some((vram_name, vram_bytes)) = gpu_vram_info.iter()
                                                    .find(|(name, _)| name.as_ref().map_or(false, |n| n.contains(gpu_name) || gpu_name.contains(n))) {
                                                    eprintln!("[GPU_MAPPING] Found match: bridge_name='{}' vram_name={:?} vram_bytes={:?}", gpu_name, vram_name, vram_bytes);
                                                    let vram_total_mb = vram_bytes.map(|bytes| (bytes / 1024 / 1024) as f64);
                                                    let vram_usage_pct = if let (Some(used), Some(total)) = (x.vram_used_mb.map(|v| v as f64), vram_total_mb) {
                                                        if total > 0.0 {
                                                            Some((used / total) * 100.0)
                                                        } else {
                                                            None
                                                        }
                                                    } else {
                                                        None
                                                    };
                                                    eprintln!("[GPU_MAPPING] Calculated: vram_total_mb={:?} vram_usage_pct={:?}", vram_total_mb, vram_usage_pct);
                                                    (vram_total_mb, vram_usage_pct)
                                                } else {
                                                    eprintln!("[GPU_MAPPING] No VRAM match found for GPU: {}", gpu_name);
                                                    (None, None)
                                                }
                                            } else {
                                                eprintln!("[GPU_MAPPING] GPU has no name");
                                                (None, None)
                                            };
                                            
                                            // 确保VRAM数据正确传递到前端
                                            let final_vram_used_mb = x.vram_used_mb.or_else(|| {
                                                // 如果桥接数据没有vram_used_mb，但有计算出的使用率，则反推计算
                                                if let (Some(total), Some(pct)) = (vram_total_mb, vram_usage_pct) {
                                                    Some(total * pct / 100.0)
                                                } else {
                                                    None
                                                }
                                            });
                                            
                                            eprintln!("[GPU_FINAL] Creating GpuPayload: name={:?} vram_used_mb={:?} vram_total_mb={:?} vram_usage_pct={:?}", 
                                                x.name, final_vram_used_mb, vram_total_mb, vram_usage_pct);
                                            
                                            // 查询GPU深度监控指标（编码/解码单元使用率、显存带宽使用率、P-State）
                                            let (encode_util, decode_util, vram_bandwidth, p_state) = if let Some(gpu_name) = &x.name {
                                                gpu_utils::query_gpu_advanced_metrics(gpu_name)
                                            } else {
                                                (None, None, None, None)
                                            };
                                            
                                            GpuPayload {
                                                name: x.name.clone(),
                                                temp_c: x.temp_c,
                                                load_pct: x.load_pct,
                                                core_mhz: x.core_mhz,
                                                memory_mhz: x.memory_mhz,
                                                fan_rpm: x.fan_rpm,
                                                fan_duty_pct: x.fan_duty_pct,
                                                vram_used_mb: final_vram_used_mb,
                                                vram_total_mb,
                                                vram_usage_pct,
                                                power_w: x.power_w,
                                                power_limit_w: x.power_limit_w,
                                                voltage_v: x.voltage_v,
                                                hotspot_temp_c: x.hotspot_temp_c,
                                                vram_temp_c: x.vram_temp_c,
                                                // 添加GPU深度监控指标 - 优先使用桥接层数据，其次才是模拟数据
                                                // 详细调试GPU深度指标数据流转
                                                encode_util_pct: {
                                                    let val = x.encode_util_pct.or(encode_util);
                                                    println!("[GPU_DEEP_DEBUG] GPU {} encode_util_pct: bridge={:?}, simulated={:?}, final={:?}", 
                                                        x.name.as_ref().unwrap_or(&"Unknown".to_string()), x.encode_util_pct, encode_util, val);
                                                    val
                                                },
                                                decode_util_pct: {
                                                    let val = x.decode_util_pct.or(decode_util);
                                                    println!("[GPU_DEEP_DEBUG] GPU {} decode_util_pct: bridge={:?}, simulated={:?}, final={:?}", 
                                                        x.name.as_ref().unwrap_or(&"Unknown".to_string()), x.decode_util_pct, decode_util, val);
                                                    val
                                                },
                                                vram_bandwidth_pct: {
                                                    let val = x.vram_bandwidth_pct.or(vram_bandwidth);
                                                    println!("[GPU_DEEP_DEBUG] GPU {} vram_bandwidth_pct: bridge={:?}, simulated={:?}, final={:?}", 
                                                        x.name.as_ref().unwrap_or(&"Unknown".to_string()), x.vram_bandwidth_pct, vram_bandwidth, val);
                                                    val
                                                },
                                                p_state: {
                                                    // 先克隆p_state以避免move语义错误
                                                    let simulated = p_state.clone();
                                                    let val = x.p_state.clone().or_else(|| simulated);
                                                    println!("[GPU_DEEP_DEBUG] GPU {} p_state: bridge={:?}, simulated={:?}, final={:?}", 
                                                        x.name.as_ref().unwrap_or(&"Unknown".to_string()), x.p_state, p_state, val);
                                                    val
                                                },
                                            }
                                        }).collect();
                                        if !mapped.is_empty() { gpus = Some(mapped); }
                                    }
                                    // 主板电压
                                    if let Some(vs) = &b.mobo_voltages {
                                        let mapped: Vec<VoltagePayload> = vs.iter().map(|x| VoltagePayload {
                                            name: x.name.clone(),
                                            volts: x.volts,
                                        }).collect();
                                        if !mapped.is_empty() { mobo_voltages = Some(mapped); }
                                    }
                                    // 多风扇
                                    if let Some(fx) = &b.fans_extra {
                                        let mapped: Vec<FanPayload> = fx.iter().map(|x| FanPayload {
                                            name: x.name.clone(),
                                            rpm: x.rpm,
                                            pct: x.pct,
                                        }).collect();
                                        if !mapped.is_empty() { fans_extra = Some(mapped); }
                                    }
                                    // 健康指标
                                    hb_tick = b.hb_tick;
                                    idle_sec = b.idle_sec;
                                    exc_count = b.exc_count;
                                    uptime_sec = b.uptime_sec;
                                    // 第二梯队：CPU 扩展与重建秒数
                                    cpu_pkg_power_w = b.cpu_pkg_power_w;
                                    cpu_avg_freq_mhz = b.cpu_avg_freq_mhz;
                                    cpu_throttle_active = b.cpu_throttle_active;
                                    cpu_throttle_reasons = b.cpu_throttle_reasons.clone();
                                    since_reopen_sec = b.since_reopen_sec;
                                    // 每核心数组
                                    cpu_core_loads_pct = b.cpu_core_loads_pct.clone();
                                    cpu_core_clocks_mhz = b.cpu_core_clocks_mhz.clone();
                                    cpu_core_temps_c = b.cpu_core_temps_c.clone();
                                    if let Some(fans) = &b.fans {
                                        let mut best_cpu: Option<i32> = None;
                                        let mut best_case: Option<i32> = None;
                                        let mut best_cpu_pct: Option<i32> = None;
                                        let mut best_case_pct: Option<i32> = None;
                                        for f in fans {
                                            if let Some(rpm) = f.rpm {
                                                let name_lc = f.name.as_deref().unwrap_or("").to_ascii_lowercase();
                                                if name_lc.contains("cpu") {
                                                    best_cpu = Some(best_cpu.map_or(rpm, |v| v.max(rpm)));
                                                } else {
                                                    best_case = Some(best_case.map_or(rpm, |v| v.max(rpm)));
                                                }
                                            }
                                            if let Some(p) = f.pct {
                                                let name_lc = f.name.as_deref().unwrap_or("").to_ascii_lowercase();
                                                if name_lc.contains("cpu") {
                                                    best_cpu_pct = Some(best_cpu_pct.map_or(p, |v| v.max(p)));
                                                } else {
                                                    best_case_pct = Some(best_case_pct.map_or(p, |v| v.max(p)));
                                                }
                                            }
                                        }
                                        cpu_fan = best_cpu.map(|v| v.max(0) as u32);
                                        case_fan = best_case.map(|v| v.max(0) as u32);
                                        cpu_fan_pct = best_cpu_pct.map(|v| v.clamp(0, 100) as u32);
                                        case_fan_pct = best_case_pct.map(|v| v.clamp(0, 100) as u32);
                                    }
                                } else {
                                    fresh_now = Some(false);
                                }
                            }
                        }
                        if let Some(f) = fresh_now {
                            if last_bridge_fresh.map(|x| x != f).unwrap_or(true) {
                                if f { eprintln!("[bridge][status] data became FRESH"); } else { eprintln!("[bridge][status] data became STALE"); }
                            }
                            last_bridge_fresh = Some(f);
                        }
                        // 电池信息（WMI + WinAPI）
                        let mut wmi_remain: Option<i32> = None;
                        let mut wmi_to_full: Option<i32> = None;
                        if let Some(c) = &wmi_fan_conn {
                            let (bp, bs) = battery_utils::wmi_read_battery(c);
                            battery_percent = bp;
                            battery_status = bs;
                            let (r_sec, tf_sec) = battery_utils::wmi_read_battery_time(c);
                            wmi_remain = r_sec;
                            wmi_to_full = tf_sec;
                        }
                        let (ac, remain_win, to_full_win) = read_power_status();
                        battery_ac_online = ac;
                        battery_time_remaining_sec = wmi_remain.or(remain_win);
                        battery_time_to_full_sec = wmi_to_full.or(to_full_win);
                        // 将电池健康变量注入返回元组（通过重新查询一次以确保作用域内可读）
                        let (design_cap_ret, full_cap_ret, cycle_cnt_ret) = if let Some(c) = &wmi_fan_conn { battery_utils::wmi_read_battery_health(c) } else { (None, None, None) };
                        (
                            cpu_t,
                            mobo_t,
                            cpu_fan,
                            case_fan,
                            cpu_fan_pct,
                            case_fan_pct,
                            is_admin,
                            has_temp,
                            has_temp_value,
                            has_fan,
                            has_fan_value,
                            storage_temps,
                            gpus,
                            mobo_voltages,
                            fans_extra,
                            battery_percent,
                            battery_status,
                            design_cap_ret,
                            full_cap_ret,
                            cycle_cnt_ret,
                            battery_ac_online,
                            battery_time_remaining_sec,
                            battery_time_to_full_sec,
                            hb_tick,
                            idle_sec,
                            exc_count,
                            uptime_sec,
                            cpu_pkg_power_w,
                            cpu_avg_freq_mhz,
                            cpu_throttle_active,
                            cpu_throttle_reasons,
                            since_reopen_sec,
                            cpu_core_loads_pct,
                            cpu_core_clocks_mhz,
                            cpu_core_temps_c,
                        )
                    };

                    let temp_opt = bridge_cpu_temp.or_else(|| wmi_temp_conn.as_ref().and_then(|c| thermal_utils::wmi_read_cpu_temp_c(c)));
                    let fan_opt = bridge_cpu_fan.or_else(|| wmi_fan_conn.as_ref().and_then(|c| thermal_utils::wmi_read_fan_rpm(c)));

                    let temp_line = if let Some(t) = temp_opt {
                        match bridge_mobo_temp {
                            Some(mb) => format!("温度: {:.1}°C  主板: {:.1}°C", t, mb),
                            None => format!("温度: {:.1}°C", t),
                        }
                    } else if let Some(mb) = bridge_mobo_temp {
                        format!("温度: —  主板: {:.1}°C", mb)
                    } else {
                        let mut s = "温度: —".to_string();
                        if has_temp == Some(true) && has_temp_value == Some(false) {
                            if is_admin == Some(false) { s.push_str(" (需管理员)"); }
                            else { s.push_str(" (无读数)"); }
                        }
                        s
                    };

                    // 风扇行：优先 RPM，否则占空比
                    let fan_line = {
                        if fan_opt.is_some() || case_fan.is_some() {
                            match (fan_opt, case_fan) {
                                (Some(c), Some(k)) => format!("风扇: CPU {} RPM / {} RPM", c, k),
                                (Some(c), None) => format!("风扇: CPU {} RPM", c),
                                (None, Some(k)) => format!("风扇: {} RPM", k),
                                _ => unreachable!(),
                            }
                        } else if bridge_cpu_fan_pct.is_some() || case_fan_pct.is_some() {
                            match (bridge_cpu_fan_pct, case_fan_pct) {
                                (Some(c), Some(k)) => format!("风扇: CPU {}% / {}%", c, k),
                                (Some(c), None) => format!("风扇: CPU {}%", c),
                                (None, Some(k)) => format!("风扇: {}%", k),
                                _ => unreachable!(),
                            }
                        } else {
                            let mut s = "风扇: —".to_string();
                            if has_fan == Some(true) && has_fan_value == Some(false) {
                                if is_admin == Some(false) { s.push_str(" (需管理员)"); }
                                else { s.push_str(" (无读数)"); }
                            }
                            s
                        }
                    };

                    // 网络/磁盘行
                    let net_line = format!(
                        "网络: 下行 {} 上行 {}",
                        fmt_bps(ema_net_rx),
                        fmt_bps(ema_net_tx)
                    );
                    let disk_line = format!(
                        "磁盘: 读 {} 写 {}",
                        fmt_bps(ema_disk_r),
                        fmt_bps(ema_disk_w)
                    );

                    // GPU 汇总行（最多展示 2 个，多余以 +N 表示）
                    let gpu_line: String = match &gpus {
                        Some(gs) if !gs.is_empty() => {
                            let mut parts: Vec<String> = Vec::new();
                            for (i, g) in gs.iter().enumerate().take(2) {
                                let label = g.name.clone().unwrap_or_else(|| format!("GPU{}", i + 1));
                                let vram = g
                                    .vram_used_mb
                                    .map(|v| format!("{:.0} MB", v))
                                    .unwrap_or_else(|| "—".to_string());
                                let pwr = g
                                    .power_w
                                    .map(|w| format!("{:.1} W", w))
                                    .unwrap_or_else(|| "—".to_string());
                                parts.push(format!("{} VRAM {} PWR {}", label, vram, pwr));
                            }
                            let mut s = format!("GPU: {}", parts.join(", "));
                            if gs.len() > 2 { s.push_str(&format!(" +{}", gs.len() - 2)); }
                            s
                        }
                        _ => "GPU: —".to_string(),
                    };

                    // 存储温度行（最多显示 3 个，余量以 +N 表示）
                    let storage_line: String = match &storage_temps {
                        Some(sts) if !sts.is_empty() => {
                            let mut parts: Vec<String> = Vec::new();
                            for (i, st) in sts.iter().enumerate().take(3) {
                                let label = st.name.clone().unwrap_or_else(|| format!("驱动{}", i + 1));
                                let val = st.temp_c.map(|t| format!("{:.1}°C", t)).unwrap_or_else(|| "—".to_string());
                                parts.push(format!("{} {}", label, val));
                            }
                            let mut s = format!("存储: {}", parts.join(", "));
                            if sts.len() > 3 { s.push_str(&format!(" +{}", sts.len() - 3)); }
                            s
                        }
                        _ => "存储: —".to_string(),
                    };

                    // 桥接健康行
                    let bridge_line: String = {
                        let mut parts: Vec<String> = Vec::new();
                        if let Some(t) = hb_tick { parts.push(format!("hb {}", t)); }
                        if let Some(idle) = idle_sec { parts.push(format!("idle {}s", idle)); }
                        if let Some(ex) = exc_count { parts.push(format!("exc {}", ex)); }
                        if let Some(up) = uptime_sec {
                            let h = up / 3600; let m = (up % 3600) / 60; let s = up % 60;
                            if h > 0 { parts.push(format!("up {}h{}m", h, m)); }
                            else if m > 0 { parts.push(format!("up {}m{}s", m, s)); }
                            else { parts.push(format!("up {}s", s)); }
                        }
                        if let Some(sr) = since_reopen_sec { parts.push(format!("reopen {}s", sr)); }
                        if parts.is_empty() { "桥接: —".to_string() } else { format!("桥接: {}", parts.join(" ")) }
                    };

                    // 供托盘与前端使用的最佳风扇 RPM（优先 CPU 再机箱）
                    let fan_best = fan_opt.or(case_fan);

                    // 公网行
                    let (pub_ip_opt, pub_isp_opt) = match pub_net_c.lock() {
                        Ok(g) => (g.ip.clone(), g.isp.clone()),
                        Err(_) => (None, None),
                    };
                    let public_line: String = match (pub_ip_opt.as_ref(), pub_isp_opt.as_ref()) {
                        (Some(ip), Some(isp)) => format!("公网: {} {}", ip, isp),
                        (Some(ip), None) => format!("公网: {}", ip),
                        _ => "公网: —".to_string(),
                    };

                    // 更新菜单只读信息（忽略错误）
                    let _ = info_cpu_c.set_text(&cpu_line);
                    let _ = info_mem_c.set_text(&mem_line);
                    let _ = info_temp_c.set_text(&temp_line);
                    let _ = info_fan_c.set_text(&fan_line);
                    let _ = info_net_c.set_text(&net_line);
                    let _ = info_public_c.set_text(&public_line);
                    let _ = info_disk_c.set_text(&disk_line);
                    let _ = info_gpu_c.set_text(&gpu_line);
                    let _ = info_store_c.set_text(&storage_line);
                    let _ = info_bridge_c.set_text(&bridge_line);

                    // 更新托盘 tooltip，避免一直停留在“初始化中”
                    let tooltip = format!(
                        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                        cpu_line, mem_line, temp_line, fan_line, net_line, public_line, disk_line, gpu_line, storage_line, bridge_line
                    );
                    let _ = tray_c.set_tooltip(Some(&tooltip));
                    // 保存以供 [debug] 复制
                    if let Ok(mut g) = last_info_text_c.lock() { *g = tooltip.clone(); }

                    // 托盘顶部文本：优先温度整数（如 65C），否则 CPU%
                    let top_text = if let Some(t) = temp_opt.map(|v| v.round() as i32) {
                        format!("{}C", t)
                    } else {
                        format!("{}%", cpu_usage.round() as u32)
                    };

                    // 读取配置决定底部文本：cpu% | mem% | fanRPM（无读数则回退 CPU%）
                    let mode = cfg_state_c
                        .lock().ok()
                        .and_then(|c| c.tray_bottom_mode.clone())
                        .unwrap_or_else(|| if cfg_state_c.lock().ok().map(|c| c.tray_show_mem).unwrap_or(false) { "mem".to_string() } else { "cpu".to_string() });
                    let bottom_text = match mode.as_str() {
                        "mem" => format!("{}%", mem_pct.round() as u32),
                        "fan" => match fan_best {
                            Some(rpm) if rpm > 0 => format!("{}", rpm), // 仅数字，节省宽度
                            _ => format!("{}%", cpu_usage.round() as u32), // 回退
                        },
                        _ => format!("{}%", cpu_usage.round() as u32),
                    };

                    let icon_img: Image = tray_graphics_utils::make_tray_icon(&top_text, &bottom_text);
                    let _ = tray_c.set_icon(Some(icon_img));

                    // 广播到前端
                    // 读取 Wi‑Fi 信息（Windows）
                    let wi = read_wifi_info_ext();
                    // 读取网络接口、逻辑磁盘
                    let net_ifs = match &wmi_fan_conn { Some(c) => network_disk_utils::wmi_list_net_ifs(c), None => None };
                    let logical_disks = match &wmi_fan_conn { Some(c) => network_disk_utils::wmi_list_logical_disks(c), None => None };
                    // SMART 健康：优先 smartctl（若可用），其次 ROOT\WMI 的 FailurePredictStatus
                    // 若失败，再尝试 NVMe 的 Storage 可靠性计数器（PowerShell）
                    // 仍失败，则回退 ROOT\CIMV2 的 DiskDrive.Status
                    let mut smart_health = smartctl_collect();
                    if smart_health.is_none() {
                        smart_health = match &wmi_temp_conn { Some(c) => wmi_list_smart_status(c), None => None };
                    }
                    if smart_health.is_none() {
                        // NVMe 回退（可能仅返回温度/磨损/部分计数）
                        smart_health = nvme_storage_reliability_ps();
                    }
                    if smart_health.is_none() {
                        smart_health = match &wmi_fan_conn { Some(c) => wmi_fallback_disk_status(c), None => None };
                    }
                    // 电池：已在上文解构块中通过 WMI 读取

                    let now_ts = chrono::Local::now().timestamp_millis();
                    let snapshot = SensorSnapshot {
                        cpu_usage,
                        mem_used_gb: used_gb as f32,
                        mem_total_gb: total_gb as f32,
                        mem_pct: mem_pct as f32,
                        mem_avail_gb: Some(avail_gb as f32),
                        swap_used_gb: if swap_total > 0.0 { Some(swap_used_gb as f32) } else { None },
                        swap_total_gb: if swap_total > 0.0 { Some(swap_total_gb as f32) } else { None },
                        // 内存细分字段
                        mem_cache_gb,
                        mem_committed_gb,
                        mem_commit_limit_gb,
                        mem_pool_paged_gb,
                        mem_pool_nonpaged_gb,
                        mem_pages_per_sec,
                        mem_page_reads_per_sec,
                        mem_page_writes_per_sec,
                        mem_page_faults_per_sec,
                        net_rx_bps: ema_net_rx,
                        net_tx_bps: ema_net_tx,
                        public_ip: pub_ip_opt,
                        isp: pub_isp_opt,
                        wifi_ssid: wi.ssid,
                        wifi_signal_pct: wi.signal_pct,
                        wifi_link_mbps: wi.link_mbps.or(wi.rx_mbps).or(wi.tx_mbps),
                        wifi_bssid: wi.bssid,
                        wifi_channel: wi.channel,
                        wifi_radio: wi.radio,
                        wifi_band: wi.band,
                        wifi_rx_mbps: wi.rx_mbps,
                        wifi_tx_mbps: wi.tx_mbps,
                        wifi_rssi_dbm: wi.rssi_dbm,
                        wifi_rssi_estimated: if wi.rssi_dbm.is_some() { Some(wi.rssi_estimated) } else { None },
                        wifi_auth: wi.auth,
                        wifi_cipher: wi.cipher,
                        wifi_chan_width_mhz: wi.chan_width_mhz,
                        net_ifs,
                        disk_r_bps: ema_disk_r,
                        disk_w_bps: ema_disk_w,
                        cpu_temp_c: temp_opt.map(|v| v as f32),
                        mobo_temp_c: bridge_mobo_temp,
                        fan_rpm: fan_best.map(|v| v as i32),
                        mobo_voltages,
                        fans_extra,
                        storage_temps,
                        logical_disks,
                        smart_health,
                        gpus,
                        hb_tick,
                        idle_sec,
                        exc_count,
                        uptime_sec,
                        cpu_pkg_power_w,
                        cpu_avg_freq_mhz,
                        cpu_throttle_active,
                        cpu_throttle_reasons,
                        since_reopen_sec,
                        cpu_core_loads_pct,
                        cpu_core_clocks_mhz,
                        cpu_core_temps_c,
                        disk_r_iops,
                        disk_w_iops,
                        disk_queue_len,
                        net_rx_err_ps,
                        net_tx_err_ps,
                        ping_rtt_ms,
                        packet_loss_pct,
                        active_connections,
                        rtt_multi,
                        top_cpu_procs,
                        top_mem_procs,
                        battery_percent,
                        battery_status,
                        battery_design_capacity,
                        battery_full_charge_capacity,
                        battery_cycle_count,
                        battery_ac_online,
                        battery_time_remaining_sec,
                        battery_time_to_full_sec,
                        timestamp_ms: now_ts,
                    };
                    eprintln!(
                        "[emit] sensor://snapshot ts={} cpu={:.0}% mem={:.0}% net_rx={} net_tx={}",
                        now_ts,
                        cpu_usage,
                        mem_pct,
                        ema_net_rx as u64,
                        ema_net_tx as u64
                    );
                    let _ = app_handle_c.emit("sensor://snapshot", snapshot);

                    thread::sleep(Duration::from_secs(1));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
