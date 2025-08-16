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

// 全局静态变量：上次WMI重建时间
static mut LAST_WMI_REOPEN: Option<std::time::Instant> = None;
// 全局静态变量：上次EMA平滑值
static mut EMA_NET_RX: f64 = 0.0;
static mut EMA_NET_TX: f64 = 0.0;
static mut EMA_DISK_R: f64 = 0.0;
static mut EMA_DISK_W: f64 = 0.0;
// 全局静态变量：上次网络字节数
static mut LAST_NET_RX_BYTES: u64 = 0;
static mut LAST_NET_TX_BYTES: u64 = 0;
// 全局静态变量：上次磁盘字节数
static mut LAST_DISK_R_BYTES: u64 = 0;
static mut LAST_DISK_W_BYTES: u64 = 0;
static mut LAST_NET_TIMESTAMP: Option<std::time::Instant> = None;

// 导入各模块的公共类型和函数
use smart_utils::{wmi_list_smart_status, wmi_fallback_disk_status};
use process_utils::*;
use wifi_utils::*;
use types::*;
use config_utils::*;
use wmi_utils::*;
use crate::types::{SensorSnapshot, GpuPayload, StorageTempPayload, FanPayload, VoltagePayload};
use crate::gpu_utils::BridgeGpu;
use crate::process_utils::RttResultPayload;
use crate::power_utils::read_power_status;
use powershell_utils::nvme_storage_reliability_ps;

// ================================================================================
// 1. TAURI 命令函数
// ================================================================================

// greet 命令已移至 config_utils 模块

// ================================================================================
// 2. 辅助函数定义
// ================================================================================

/// 从sysinfo获取网络和磁盘字节数（备用数据源）
fn get_sysinfo_bytes(networks: &sysinfo::Networks, sys: &sysinfo::System) -> (u64, u64, u64, u64) {
    // 网络字节数
    let mut net_rx_bytes: u64 = 0;
    let mut net_tx_bytes: u64 = 0;
    
    // 统计所有活跃网卡（放宽过滤条件）
    for (if_name, net_if) in networks {
        let name = if_name;
        
        // 只过滤明显的虚拟接口，保留所有可能的物理网卡
        if name.is_empty() || name.contains("Loopback") {
            continue;
        }
        
        // 安全获取字节数，避免空值
        let rx_bytes = net_if.received();
        let tx_bytes = net_if.transmitted();
        
        // 只统计有活动的网卡
        if rx_bytes > 0 || tx_bytes > 0 {
            net_rx_bytes += rx_bytes;
            net_tx_bytes += tx_bytes;
            
            eprintln!("[debug] sysinfo网卡 {} 累计接收: {} 字节, 累计发送: {} 字节", 
                     name, rx_bytes, tx_bytes);
        }
    }
    
    // 磁盘字节数
    let mut disk_r_total: u64 = 0;
    let mut disk_w_total: u64 = 0;
    
    // 遍历所有进程，累计磁盘读写字节数
    for (_, process) in sys.processes() {
        let disk_usage = process.disk_usage();
        disk_r_total += disk_usage.read_bytes;
        disk_w_total += disk_usage.written_bytes;
    }
    
    (net_rx_bytes, net_tx_bytes, disk_r_total, disk_w_total)
}

// ================================================================================
// 3. 前端数据结构定义 (PAYLOAD 结构体)
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
                    
                    // 查询内存细分指标（通过WMI）
                    let (mem_cache_gb, mem_committed_gb, mem_commit_limit_gb, 
                         mem_pool_paged_gb, mem_pool_nonpaged_gb, mem_pages_per_sec,
                         mem_page_reads_per_sec, mem_page_writes_per_sec, mem_page_faults_per_sec) = 
                        match &wmi_perf_conn {
                            Some(conn) => wmi_utils::wmi_perf_memory(conn),
                            None => (None, None, None, None, None, None, None, None, None)
                        };
                    
                    // 从桥接获取传感器数据
                    let bridge_out = match bridge_data_sampling.lock() {
                        Ok(g) => g.0.clone(),
                        Err(_) => None,
                    };
                    
                    // 提取各种传感器数据
                    let temp_opt = bridge_out.as_ref().and_then(|b| b.cpu_temp_c);
                    let mobo_temp_opt = bridge_out.as_ref().and_then(|b| b.mobo_temp_c);
                    // 从fans数组中提取第一个风扇的RPM，转换为f64
                    let fan_opt = bridge_out.as_ref()
                        .and_then(|b| b.fans.as_ref())
                        .and_then(|fans| fans.first())
                        .and_then(|fan| fan.rpm)
                        .map(|rpm| rpm as f64);
                    
                    // 类型转换函数：BridgeVoltage -> VoltagePayload  
                    let mobo_voltages_opt = bridge_out.as_ref()
                        .and_then(|b| b.mobo_voltages.as_ref())
                        .map(|voltages| voltages.iter().map(|v| VoltagePayload {
                            name: v.name.clone(),
                            volts: v.volts,
                        }).collect());
                    
                    // 类型转换函数：BridgeFan -> FanPayload
                    let fans_extra_opt = bridge_out.as_ref()
                        .and_then(|b| b.fans_extra.as_ref())
                        .map(|fans| fans.iter().map(|f| FanPayload {
                            name: f.name.clone(),
                            rpm: f.rpm,
                            pct: f.pct,
                        }).collect());
                    
                    // 类型转换函数：BridgeStorageTemp -> StorageTempPayload
                    let storage_temps_opt = bridge_out.as_ref()
                        .and_then(|b| b.storage_temps.as_ref())
                        .map(|temps| temps.iter().map(|t| StorageTempPayload {
                            name: t.name.clone(),
                            temp_c: t.temp_c,
                            drive_letter: None, // BridgeStorageTemp没有drive_letter字段
                        }).collect());
                    
                    // 类型转换函数：BridgeGpu -> GpuPayload
                    let gpus_opt: Option<Vec<GpuPayload>> = bridge_out.as_ref()
                        .and_then(|b| b.gpus.as_ref())
                        .map(|gpus| gpus.iter().map(|g| GpuPayload {
                            name: g.name.clone(),
                            temp_c: g.temp_c,
                            load_pct: g.load_pct,
                            core_mhz: g.core_mhz,
                            memory_mhz: g.memory_mhz,
                            fan_rpm: g.fan_rpm,
                            fan_duty_pct: g.fan_duty_pct,
                            vram_used_mb: g.vram_used_mb,
                            vram_total_mb: g.vram_total_mb,
                            vram_usage_pct: g.vram_used_mb.and_then(|used| g.vram_total_mb.map(|total| if total > 0.0 { (used / total) * 100.0 } else { 0.0 })),
                            power_w: g.power_w,
                            power_limit_w: g.power_limit_w,
                            voltage_v: g.voltage_v,
                            hotspot_temp_c: g.hotspot_temp_c,
                            vram_temp_c: g.vram_temp_c,
                            encode_util_pct: g.encode_util_pct,
                            decode_util_pct: g.decode_util_pct,
                            vram_bandwidth_pct: g.vram_bandwidth_pct,
                            p_state: g.p_state.clone(),
                        }).collect());
                    
                    // 磁盘IOPS相关（从WMI性能计数器获取，失败时提供基于速率的估算）
                    let (disk_r_iops_opt, disk_w_iops_opt, disk_queue_len_opt) = match &wmi_perf_conn {
                        Some(conn) => {
                            let (r_iops, w_iops, queue) = wmi_utils::wmi_perf_disk(conn);
                            // 如果WMI返回0值，基于磁盘速率估算IOPS（假设平均4KB每次IO）
                            let estimated_r_iops = if r_iops.unwrap_or(0.0) == 0.0 && ema_disk_r > 1024.0 {
                                Some(ema_disk_r / 4096.0) // 4KB per IO估算
                            } else { r_iops };
                            let estimated_w_iops = if w_iops.unwrap_or(0.0) == 0.0 && ema_disk_w > 1024.0 {
                                Some(ema_disk_w / 4096.0) // 4KB per IO估算
                            } else { w_iops };
                            (estimated_r_iops, estimated_w_iops, queue)
                        },
                        None => (None, None, None)
                    };
                    
                    // 网络错误相关（从WMI性能计数器获取）
                    let (net_rx_err_opt, net_tx_err_opt, packet_loss_opt, active_conn_opt, _) = match &wmi_perf_conn {
                        Some(conn) => wmi_utils::wmi_perf_net_err(conn),
                        None => (None, None, None, None, None)
                    };
                    
                    // 网络延迟（简单ping测试）
                    let ping_rtt_opt: Option<f64> = None; // 暂时使用None，可后续实现
                    let rtt_multi_opt: Option<Vec<RttResultPayload>> = None;
                    
                    // 进程相关（从系统信息获取）
                    let (top_cpu_procs_opt, top_mem_procs_opt) = get_top_processes(&sys, 5);
                    
                    // 电池相关（使用系统API获取）
                    let (battery_ac_opt, battery_time_remaining_opt, battery_time_to_full_opt) = read_power_status();
                    let battery_pct_opt: Option<i32> = None;
                    let battery_status_opt: Option<String> = None;
                    let battery_design_opt: Option<u32> = None;
                    let battery_full_opt: Option<u32> = None;
                    let battery_cycles_opt: Option<u32> = None;
                    
                    eprintln!("[debug] 内存细分 - 缓存: {:?} GB, 提交: {:?} GB, 分页池: {:?} GB, 非分页池: {:?} GB", 
                             mem_cache_gb, mem_committed_gb, mem_pool_paged_gb, mem_pool_nonpaged_gb);

                    // --- 网络和磁盘累计字节数（优先使用WMI，备用sysinfo）---
                    let (net_rx_bytes, net_tx_bytes, disk_r_total, disk_w_total) = match &wmi_perf_conn {
                        Some(conn) => {
                            let (wmi_net_rx, wmi_net_tx, wmi_disk_r, wmi_disk_w) = wmi_utils::wmi_get_network_disk_bytes(conn);
                            
                            // 如果WMI数据有效，使用WMI数据
                            if wmi_net_rx > 0 || wmi_net_tx > 0 || wmi_disk_r > 0 || wmi_disk_w > 0 {
                                eprintln!("[debug] 使用WMI数据源 - 网络接收: {} 字节, 网络发送: {} 字节, 磁盘读: {} 字节, 磁盘写: {} 字节", 
                                         wmi_net_rx, wmi_net_tx, wmi_disk_r, wmi_disk_w);
                                (wmi_net_rx, wmi_net_tx, wmi_disk_r, wmi_disk_w)
                            } else {
                                // WMI数据无效，回退到sysinfo
                                eprintln!("[debug] WMI数据无效，回退到sysinfo数据源");
                                let (sysinfo_net_rx, sysinfo_net_tx, sysinfo_disk_r, sysinfo_disk_w) = get_sysinfo_bytes(&networks, &sys);
                                (sysinfo_net_rx, sysinfo_net_tx, sysinfo_disk_r, sysinfo_disk_w)
                            }
                        },
                        None => {
                            // 无WMI连接，使用sysinfo
                            eprintln!("[debug] 无WMI连接，使用sysinfo数据源");
                            let (sysinfo_net_rx, sysinfo_net_tx, sysinfo_disk_r, sysinfo_disk_w) = get_sysinfo_bytes(&networks, &sys);
                            (sysinfo_net_rx, sysinfo_net_tx, sysinfo_disk_r, sysinfo_disk_w)
                        }
                    };
                    
                    eprintln!("[debug] 最终数据 - 网络接收: {} 字节, 网络发送: {} 字节, 磁盘读: {} 字节, 磁盘写: {} 字节", 
                             net_rx_bytes, net_tx_bytes, disk_r_total, disk_w_total);
                        
                    // 获取当前时间点
                    let now = Instant::now();
                    
                    // 从全局变量读取上次的累计字节数
                    let mut last_net_rx_total = 0;
                    let mut last_net_tx_total = 0;
                    let mut last_disk_r_total = 0;
                    let mut last_disk_w_total = 0;
                    let mut last_timestamp = now - Duration::from_secs(1); // 默认1秒前，避免时间差为0
                    
                    unsafe {
                        last_net_rx_total = LAST_NET_RX_BYTES;
                        last_net_tx_total = LAST_NET_TX_BYTES;
                        last_disk_r_total = LAST_DISK_R_BYTES;
                        last_disk_w_total = LAST_DISK_W_BYTES;
                        if let Some(ts) = LAST_NET_TIMESTAMP {
                            last_timestamp = ts;
                        }
                    }
                    
                    // 计算时间差（秒）
                    let dt = now.duration_since(last_timestamp).as_secs_f64();
                    if dt <= 0.01 { // 降低阈值到10ms
                        eprintln!("[warn] 时间差异过小: {:.3}s，跳过本次计算", dt);
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                    
                    // 检查是否需要重建 WMI 连接（长时间间隔可能是系统休眠后恢复）
                    let need_reopen = dt > 30.0; // 超过30秒，可能是系统休眠后恢复
                    if need_reopen {
                        eprintln!("[warn] 检测到长时间间隔 {:.1}s，可能是系统休眠后恢复，重建 WMI 连接", dt);
                        // 重建 WMI 连接
                        wmi_temp_conn = {
                            if let Ok(com) = wmi::COMLibrary::new() {
                                wmi::WMIConnection::with_namespace_path("ROOT\\WMI", com.into()).ok()
                            } else { None }
                        };
                        wmi_fan_conn = {
                            if let Ok(com) = wmi::COMLibrary::new() {
                                wmi::WMIConnection::new(com).ok()
                            } else { None }
                        };
                        wmi_perf_conn = {
                            if let Ok(com) = wmi::COMLibrary::new() {
                                wmi::WMIConnection::new(com).ok()
                            } else { None }
                        };
                        unsafe {
                            LAST_WMI_REOPEN = Some(now);
                        }
                    }
                    
                    // 检查是否为首次运行
                    if last_net_rx_total == 0 && last_net_tx_total == 0 && last_disk_r_total == 0 && last_disk_w_total == 0 {
                        eprintln!("[debug] 首次运行，建立基线数据");
                        unsafe {
                            LAST_NET_RX_BYTES = net_rx_bytes;
                            LAST_NET_TX_BYTES = net_tx_bytes;
                            LAST_DISK_R_BYTES = disk_r_total;
                            LAST_DISK_W_BYTES = disk_w_total;
                            LAST_NET_TIMESTAMP = Some(now);
                        }
                        thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                    
                    // 检查计数器是否重置（如系统重启），但允许合理的波动
                    // 只有在长时间间隔且计数器明显回退时才认为重置
                    let rx_reset = dt > 10.0 && net_rx_bytes < last_net_rx_total && (last_net_rx_total - net_rx_bytes) > 100_000_000; // 100MB差异且超过10秒
                    let tx_reset = dt > 10.0 && net_tx_bytes < last_net_tx_total && (last_net_tx_total - net_tx_bytes) > 100_000_000;
                    let disk_r_reset = dt > 10.0 && disk_r_total < last_disk_r_total && (last_disk_r_total - disk_r_total) > 1_000_000_000; // 1GB差异且超过10秒
                    let disk_w_reset = dt > 10.0 && disk_w_total < last_disk_w_total && (last_disk_w_total - disk_w_total) > 1_000_000_000;
                    
                    if rx_reset || tx_reset || disk_r_reset || disk_w_reset {
                        eprintln!("[warn] 检测到计数器重置，重新初始化基线");
                        unsafe {
                            LAST_NET_RX_BYTES = net_rx_bytes;
                            LAST_NET_TX_BYTES = net_tx_bytes;
                            LAST_DISK_R_BYTES = disk_r_total;
                            LAST_DISK_W_BYTES = disk_w_total;
                            LAST_NET_TIMESTAMP = Some(now);
                        }
                        thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                    // 计算速率（bytes/s），防止溢出
                    let net_rx_rate = if net_rx_bytes >= last_net_rx_total {
                        (net_rx_bytes - last_net_rx_total) as f64 / dt
                    } else {
                        eprintln!("[warn] 网络接收计数器回退: {} -> {}", last_net_rx_total, net_rx_bytes);
                        0.0
                    };
                    let net_tx_rate = if net_tx_bytes >= last_net_tx_total {
                        (net_tx_bytes - last_net_tx_total) as f64 / dt
                    } else {
                        eprintln!("[warn] 网络发送计数器回退: {} -> {}", last_net_tx_total, net_tx_bytes);
                        0.0
                    };
                    let disk_r_rate = if disk_r_total >= last_disk_r_total {
                        (disk_r_total - last_disk_r_total) as f64 / dt
                    } else {
                        eprintln!("[warn] 磁盘读取计数器回退: {} -> {}", last_disk_r_total, disk_r_total);
                        0.0
                    };
                    let disk_w_rate = if disk_w_total >= last_disk_w_total {
                        (disk_w_total - last_disk_w_total) as f64 / dt
                    } else {
                        eprintln!("[warn] 磁盘写入计数器回退: {} -> {}", last_disk_w_total, disk_w_total);
                        0.0
                    };
                    
                    eprintln!("[debug] 速率计算 - 网络接收: {:.1} KB/s, 网络发送: {:.1} KB/s, 磁盘读: {:.1} KB/s, 磁盘写: {:.1} KB/s", 
                             net_rx_rate / 1024.0, net_tx_rate / 1024.0, disk_r_rate / 1024.0, disk_w_rate / 1024.0);
                    
                    // 更新全局变量
                    unsafe {
                        LAST_NET_RX_BYTES = net_rx_bytes;
                        LAST_NET_TX_BYTES = net_tx_bytes;
                        LAST_DISK_R_BYTES = disk_r_total;
                        LAST_DISK_W_BYTES = disk_w_total;
                        LAST_NET_TIMESTAMP = Some(now);
                    }
                    
                    // 应用EMA平滑 - 优化参数以更好反映实时速度
                    let alpha = 0.7; // 提高EMA平滑因子，更快响应速度变化
                    unsafe {
                        EMA_NET_RX = alpha * net_rx_rate + (1.0 - alpha) * EMA_NET_RX;
                        EMA_NET_TX = alpha * net_tx_rate + (1.0 - alpha) * EMA_NET_TX;
                        EMA_DISK_R = alpha * disk_r_rate + (1.0 - alpha) * EMA_DISK_R;
                        EMA_DISK_W = alpha * disk_w_rate + (1.0 - alpha) * EMA_DISK_W;
                    }
                    
                    // 转换为前端使用的变量
                    let ema_net_rx = unsafe { EMA_NET_RX };
                    let ema_net_tx = unsafe { EMA_NET_TX };
                    let ema_disk_r = unsafe { EMA_DISK_R };
                    let ema_disk_w = unsafe { EMA_DISK_W };
                    
                    eprintln!("[debug] EMA平滑后 - 网络接收: {:.1} KB/s, 网络发送: {:.1} KB/s, 磁盘读: {:.1} KB/s, 磁盘写: {:.1} KB/s", 
                             ema_net_rx / 1024.0, ema_net_tx / 1024.0, ema_disk_r / 1024.0, ema_disk_w / 1024.0);

                    // GPU行（最多显示2个，余量以+N表示）
                    let gs: Option<Vec<crate::gpu_utils::BridgeGpu>> = None; // 临时占位
                    let gpu_line: String = match &gs {
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
                    let storage_temps: Option<Vec<StorageTempPayload>> = None; // 临时占位
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
                    let hb_tick: Option<u32> = None; // 临时占位
                    let idle_sec: Option<u32> = None; // 临时占位
                    let exc_count: Option<u32> = None; // 临时占位
                    let uptime_sec: Option<u32> = None; // 临时占位
                    let since_reopen_sec: Option<u32> = None; // 临时占位
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
                    let fan_best: Option<f64> = fan_opt;

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

                    // 构建各种显示行
                    let cpu_line = format!("CPU: {:.0}%", cpu_usage);
                    let mem_line = format!("内存: {:.1}/{:.1}GB ({:.0}%)", used_gb, total_gb, mem_pct);
                    let temp_line = if let Some(t) = temp_opt {
                        format!("温度: {:.0}°C", t)
                    } else {
                        "温度: —".to_string()
                    };
                    let fan_line = if let Some(f) = fan_best {
                        format!("风扇: {:.0} RPM", f)
                    } else {
                        "风扇: —".to_string()
                    };
                    let net_line = format!("网络: ↓{:.1} ↑{:.1} KB/s", ema_net_rx / 1024.0, ema_net_tx / 1024.0);
                    let disk_line = format!("磁盘: R{:.1} W{:.1} KB/s", ema_disk_r / 1024.0, ema_disk_w / 1024.0);
                    let gpu_line = if let Some(gpus) = &gpus_opt {
                        if let Some(gpu) = gpus.first() {
                            format!("GPU: {:.0}% {:.0}°C", gpu.load_pct.unwrap_or(0.0), gpu.temp_c.unwrap_or(0.0))
                        } else {
                            "GPU: —".to_string()
                        }
                    } else {
                        "GPU: —".to_string()
                    };
                    let storage_line = "存储: —".to_string();
                    let bridge_line = format!("桥接: {}", if bridge_out.is_some() { "已连接" } else { "未连接" });
                    
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
                        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                        cpu_line, mem_line, temp_line, fan_line, net_line, disk_line, gpu_line, public_line, bridge_line
                    );
                    let _ = tray_c.set_tooltip(Some(&tooltip));
                    // 保存以供 [debug] 复制
                    if let Ok(mut g) = last_info_text_c.lock() { *g = tooltip.clone(); }

                    // 托盘顶部文本：优先温度整数（如 65C），否则 CPU%
                    let top_text = if let Some(t) = temp_opt {
                        format!("{}C", t as i32)
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
                            Some(rpm) if rpm > 0.0 => format!("{}", rpm), // 仅数字，节省宽度
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
                        mem_pages_per_sec: mem_pages_per_sec,
                        mem_page_reads_per_sec: mem_page_reads_per_sec,
                        mem_page_writes_per_sec: mem_page_writes_per_sec,
                        mem_page_faults_per_sec: mem_page_faults_per_sec,
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
                        cpu_temp_c: temp_opt,
                        mobo_temp_c: mobo_temp_opt,
                        fan_rpm: fan_opt.map(|f| f as i32),
                        mobo_voltages: mobo_voltages_opt,
                        fans_extra: fans_extra_opt,
                        storage_temps: storage_temps_opt,
                        logical_disks,
                        smart_health,
                        gpus: gpus_opt,
                        hb_tick: bridge_out.as_ref().and_then(|b| b.hb_tick),
                        idle_sec: bridge_out.as_ref().and_then(|b| b.idle_sec),
                        exc_count: bridge_out.as_ref().and_then(|b| b.exc_count),
                        uptime_sec: bridge_out.as_ref().and_then(|b| b.uptime_sec),
                        cpu_pkg_power_w: bridge_out.as_ref().and_then(|b| b.cpu_pkg_power_w),
                        cpu_avg_freq_mhz: bridge_out.as_ref().and_then(|b| b.cpu_avg_freq_mhz),
                        cpu_throttle_active: bridge_out.as_ref().and_then(|b| b.cpu_throttle_active),
                        cpu_throttle_reasons: bridge_out.as_ref().and_then(|b| b.cpu_throttle_reasons.clone()),
                        since_reopen_sec: bridge_out.as_ref().and_then(|b| b.since_reopen_sec),
                        cpu_core_loads_pct: bridge_out.as_ref().and_then(|b| b.cpu_core_loads_pct.clone()),
                        cpu_core_clocks_mhz: bridge_out.as_ref().and_then(|b| b.cpu_core_clocks_mhz.clone()),
                        cpu_core_temps_c: bridge_out.as_ref().and_then(|b| b.cpu_core_temps_c.clone()),
                        disk_r_iops: disk_r_iops_opt,
                        disk_w_iops: disk_w_iops_opt,
                        disk_queue_len: disk_queue_len_opt,
                        net_rx_err_ps: net_rx_err_opt,
                        net_tx_err_ps: net_tx_err_opt,
                        ping_rtt_ms: ping_rtt_opt,
                        packet_loss_pct: packet_loss_opt,
                        active_connections: active_conn_opt,
                        rtt_multi: rtt_multi_opt,
                        top_cpu_procs: top_cpu_procs_opt,
                        top_mem_procs: top_mem_procs_opt,
                        battery_percent: battery_pct_opt,
                        battery_status: battery_status_opt,
                        battery_design_capacity: battery_design_opt,
                        battery_full_charge_capacity: battery_full_opt,
                        battery_cycle_count: battery_cycles_opt,
                        battery_ac_online: battery_ac_opt,
                        battery_time_remaining_sec: battery_time_remaining_opt,
                        battery_time_to_full_sec: battery_time_to_full_opt,
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
