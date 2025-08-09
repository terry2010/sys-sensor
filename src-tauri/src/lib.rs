// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::thread;
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        tray::TrayIconBuilder,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            // --- Build non-clickable info area as disabled menu items ---
            let info_cpu = MenuItem::with_id(app, "info_cpu", "CPU: —", false, None::<&str>)?;
            let info_mem = MenuItem::with_id(app, "info_mem", "内存: —", false, None::<&str>)?;
            let info_temp = MenuItem::with_id(app, "info_temp", "温度: —", false, None::<&str>)?;
            let info_fan = MenuItem::with_id(app, "info_fan", "风扇: —", false, None::<&str>)?;
            let info_net = MenuItem::with_id(app, "info_net", "网络: —", false, None::<&str>)?;
            let info_disk = MenuItem::with_id(app, "info_disk", "磁盘: —", false, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;

            // --- Clickable action items ---
            let show_details = MenuItem::with_id(app, "show_details", "显示详情", true, None::<&str>)?;
            let quick_settings = MenuItem::with_id(app, "quick_settings", "快速设置", true, None::<&str>)?;
            let about = MenuItem::with_id(app, "about", "关于我们", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &info_cpu,
                    &info_mem,
                    &info_temp,
                    &info_fan,
                    &info_net,
                    &info_disk,
                    &sep,
                    &show_details,
                    &quick_settings,
                    &about,
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

            // --- Handle menu events ---
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "show_details" => {
                    // TODO: 打开/聚焦详情窗口（后续步骤实现）
                    println!("[tray] 点击 显示详情");
                }
                "quick_settings" => {
                    // TODO: 打开快速设置窗口（后续步骤实现）
                    println!("[tray] 点击 快速设置");
                }
                "about" => {
                    // TODO: 打开关于我们弹窗（后续步骤实现）
                    println!("[tray] 点击 关于我们");
                }
                "exit" => {
                    println!("[tray] 退出");
                    app.exit(0);
                }
                other => {
                    println!("[tray] 未处理的菜单项: {}", other);
                }
            });

            // --- Spawn background refresh thread (1s) ---
            let info_cpu_c = info_cpu.clone();
            let info_mem_c = info_mem.clone();
            let info_temp_c = info_temp.clone();
            let info_fan_c = info_fan.clone();
            let info_net_c = info_net.clone();
            let info_disk_c = info_disk.clone();
            let tray_c = tray.clone();

            thread::spawn(move || {
                use std::time::{Duration, Instant};
                use sysinfo::{Networks, System};

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

                    // --- 网络累计字节合计 ---
                    let mut net_rx_total: u64 = 0;
                    let mut net_tx_total: u64 = 0;
                    for (_, data) in &networks {
                        net_rx_total = net_rx_total.saturating_add(data.total_received());
                        net_tx_total = net_tx_total.saturating_add(data.total_transmitted());
                    }

                    // --- 磁盘累计字节合计（按进程聚合）---
                    let mut disk_r_total: u64 = 0;
                    let mut disk_w_total: u64 = 0;
                    for (_pid, proc_) in sys.processes() {
                        let du = proc_.disk_usage();
                        disk_r_total = disk_r_total.saturating_add(du.total_read_bytes);
                        disk_w_total = disk_w_total.saturating_add(du.total_written_bytes);
                    }

                    // 计算速率（bytes/s）
                    let now = Instant::now();
                    let dt = now.duration_since(last_t).as_secs_f64().max(1e-6);
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

                    // 组织显示文本
                    let cpu_line = format!("CPU: {:.0}%", cpu_usage);
                    let mem_line = format!("内存: {:.1}/{:.1} GB ({:.0}%)", used_gb, total_gb, mem_pct);
                    let temp_line = "温度: —".to_string();
                    let fan_line = "风扇: —".to_string();
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

                    // 更新菜单只读信息（安全忽略错误）
                    let _ = info_cpu_c.set_text(&cpu_line);
                    let _ = info_mem_c.set_text(&mem_line);
                    let _ = info_temp_c.set_text(&temp_line);
                    let _ = info_fan_c.set_text(&fan_line);
                    let _ = info_net_c.set_text(&net_line);
                    let _ = info_disk_c.set_text(&disk_line);

                    // Tooltip（多行）
                    let tooltip = format!(
                        "{}\n{}\n{}\n{}\n{}\n{}",
                        cpu_line, mem_line, temp_line, fan_line, net_line, disk_line
                    );
                    let _ = tray_c.set_tooltip(Some(&tooltip));

                    thread::sleep(Duration::from_secs(1));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
