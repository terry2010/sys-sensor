// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// ---- WMI helpers: temperature & fan ----
#[derive(serde::Deserialize, Debug)]
struct MSAcpiThermalZoneTemperature {
    #[serde(rename = "CurrentTemperature")] 
    current_temperature: Option<i64>,
}

#[derive(serde::Deserialize, Debug)]
struct Win32Fan {
    #[serde(rename = "DesiredSpeed")]
    desired_speed: Option<u64>,
}

fn wmi_read_cpu_temp_c(conn: &wmi::WMIConnection) -> Option<f32> {
    let res: Result<Vec<MSAcpiThermalZoneTemperature>, _> = conn.query();
    let mut vals: Vec<f32> = Vec::new();
    if let Ok(list) = res {
        for item in list.into_iter() {
            if let Some(kx10) = item.current_temperature {
                // Kelvin x10 -> Celsius
                if kx10 > 0 {
                    let c = (kx10 as f32 / 10.0) - 273.15;
                    // 过滤异常值
                    if c > -50.0 && c < 150.0 {
                        vals.push(c);
                    }
                }
            }
        }
    }
    if vals.is_empty() { None } else { Some(vals.iter().copied().sum::<f32>() / vals.len() as f32) }
}

fn wmi_read_fan_rpm(conn: &wmi::WMIConnection) -> Option<u32> {
    // Win32_Fan 通常不提供实时转速，这里尽力读取 DesiredSpeed 作为近似；若无则返回 None
    let res: Result<Vec<Win32Fan>, _> = conn.query();
    if let Ok(list) = res {
        let mut best: u64 = 0;
        for item in list.into_iter() {
            if let Some(v) = item.desired_speed {
                if v > best { best = v; }
            }
        }
        if best > 0 { return Some(best.min(u32::MAX as u64) as u32); }
    }
    None
}

// ---- Realtime snapshot payload for frontend ----
#[derive(Clone, serde::Serialize)]
struct SensorSnapshot {
    cpu_usage: f32,
    mem_used_gb: f32,
    mem_total_gb: f32,
    mem_pct: f32,
    net_rx_bps: f64,
    net_tx_bps: f64,
    disk_r_bps: f64,
    disk_w_bps: f64,
    // 新增：温度（摄氏度）与风扇转速（RPM），可能不可用
    cpu_temp_c: Option<f32>,
    mobo_temp_c: Option<f32>,
    fan_rpm: Option<u32>,
    timestamp_ms: i64,
}

// ---- Bridge (.NET LibreHardwareMonitor) JSON payload ----
#[derive(Clone, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BridgeFan {
    name: Option<String>,
    rpm: Option<i32>,
    pct: Option<i32>,
}

#[derive(Clone, serde::Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct BridgeOut {
    cpu_temp_c: Option<f32>,
    mobo_temp_c: Option<f32>,
    fans: Option<Vec<BridgeFan>>,
    is_admin: Option<bool>,
    has_temp: Option<bool>,
    has_temp_value: Option<bool>,
    has_fan: Option<bool>,
    has_fan_value: Option<bool>,
}

// ---- Minimal 5x7 bitmap font (digits and a few symbols) ----
const FONT_W: usize = 5;
const FONT_H: usize = 7;

fn glyph_rows(ch: char) -> [u8; FONT_H] {
    match ch {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '%' => [0b10001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b11111],
        'C' => [0b00110, 0b01001, 0b10000, 0b10000, 0b10000, 0b01001, 0b00110],
        'M' => [0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001],
        '-' => [0b00000, 0b00000, 0b00000, 0b01110, 0b00000, 0b00000, 0b00000],
        _ => [0; FONT_H],
    }
}

fn draw_text_rgba(buf: &mut [u8], w: usize, h: usize, x: usize, y: usize, scale: usize, gap: usize, text: &str) {
    // simple shadow
    draw_text_rgba_no_shadow(buf, w, h, x + 1, y + 1, scale, gap, text, [0, 0, 0, 180]);
    draw_text_rgba_no_shadow(buf, w, h, x, y, scale, gap, text, [255, 255, 255, 255]);
}

fn draw_text_rgba_no_shadow(buf: &mut [u8], w: usize, h: usize, x: usize, y: usize, scale: usize, gap: usize, text: &str, color: [u8;4]) {
    let mut pen_x = x;
    for ch in text.chars() {
        let rows = glyph_rows(ch);
        for (ry, row_bits) in rows.iter().enumerate() {
            for rx in 0..FONT_W {
                if (row_bits >> (FONT_W - 1 - rx)) & 1 == 1 {
                    // draw a scale x scale block
                    for oy in 0..scale {
                        for ox in 0..scale {
                            let px = pen_x + rx * scale + ox;
                            let py = y + ry * scale + oy;
                            if px < w && py < h {
                                let idx = (py * w + px) * 4;
                                buf[idx] = color[0];
                                buf[idx + 1] = color[1];
                                buf[idx + 2] = color[2];
                                buf[idx + 3] = color[3];
                            }
                        }
                    }
                }
            }
        }
        // width = FONT_W*scale + gap
        pen_x += FONT_W * scale + gap;
    }
}

fn make_tray_icon(cpu_temp_c: Option<i32>, cpu_pct: u32) -> tauri::image::Image<'static> {
    let w: usize = 32;
    let h: usize = 32;
    let mut rgba = vec![0u8; w * h * 4]; // transparent background

    // 准备两行文本：上=温度(如 70C；无则用CPU%)，下=CPU%
    let cpu_pct_clamped = cpu_pct.min(100);
    let top_initial = match cpu_temp_c {
        Some(t) => format!("{}C", t),
        None => format!("{}%", cpu_pct_clamped),
    };
    let bottom_initial = format!("{}%", cpu_pct_clamped);

    // 计算文本宽度：chars*FONT_W*scale + (chars-1)*gap
    let calc_text_w = |chars: usize, scale: usize, gap: usize| chars * FONT_W * scale + chars.saturating_sub(1) * gap;
    // 优先使用大字号 scale=2，gap=0；若仍溢出，则降到 scale=1，gap=1
    // 顶部文本优先保持大字号，必要时去掉单位字符('C')再判断
    let mut top = top_initial.clone();
    let mut top_scale = 2usize; let mut top_gap = 0usize;
    if calc_text_w(top.chars().count(), top_scale, top_gap) > w {
        if top.ends_with('C') {
            top.pop();
        }
        if calc_text_w(top.chars().count(), top_scale, top_gap) > w {
            top_scale = 1; top_gap = 1;
        }
    }
    // 底部文本优先保持大字号，必要时去掉单位字符('%')再判断
    let mut bottom = bottom_initial.clone();
    let mut bot_scale = 2usize; let mut bot_gap = 0usize;
    if calc_text_w(bottom.chars().count(), bot_scale, bot_gap) > w {
        if bottom.ends_with('%') {
            bottom.pop();
        }
        if calc_text_w(bottom.chars().count(), bot_scale, bot_gap) > w {
            bot_scale = 1; bot_gap = 1;
        }
    }

    // 水平居中坐标
    let text_w_top = calc_text_w(top.chars().count(), top_scale, top_gap);
    let text_w_bot = calc_text_w(bottom.chars().count(), bot_scale, bot_gap);
    let x_top = (w.saturating_sub(text_w_top)) / 2;
    let x_bot = (w.saturating_sub(text_w_bot)) / 2;

    // 垂直布局：顶部留 3px，行间距 2px
    let y_top = 3usize;
    let y_bot = y_top + FONT_H * top_scale + 2;

    draw_text_rgba(&mut rgba, w, h, x_top, y_top, top_scale, top_gap, &top);
    draw_text_rgba(&mut rgba, w, h, x_bot, y_bot, bot_scale, bot_gap, &bottom);

    tauri::image::Image::new_owned(rgba, w as u32, h as u32)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::thread;
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        image::Image,
        Emitter,
        tray::TrayIconBuilder,
        WebviewWindowBuilder,
        WebviewUrl,
        Manager,
    };

    use tauri::path::BaseDirectory;

    // ---- App configuration (persisted as JSON) ----
    #[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
    struct AppConfig {
        // 托盘第二行：true=显示内存%，false=显示CPU%
        tray_show_mem: bool,
        // 网络接口白名单：为空或缺省表示聚合全部
        net_interfaces: Option<Vec<String>>,
    }

    struct AppState(std::sync::Arc<std::sync::Mutex<AppConfig>>);

    fn resolve_config_path(app: &tauri::AppHandle) -> std::path::PathBuf {
        app.path()
            .resolve("config.json", BaseDirectory::AppConfig)
            .unwrap_or_else(|_| std::path::PathBuf::from("config.json"))
    }

    fn load_config(app: &tauri::AppHandle) -> AppConfig {
        let path = resolve_config_path(app);
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(cfg) = serde_json::from_slice::<AppConfig>(&bytes) {
                return cfg;
            }
        }
        AppConfig::default()
    }

    fn save_config(app: &tauri::AppHandle, cfg: &AppConfig) -> std::io::Result<()> {
        let path = resolve_config_path(app);
        if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
        let data = serde_json::to_vec_pretty(cfg).unwrap_or_else(|_| b"{}".to_vec());
        std::fs::write(path, data)
    }

    #[tauri::command]
    fn get_config(state: tauri::State<'_, AppState>) -> AppConfig {
        if let Ok(guard) = state.0.lock() { guard.clone() } else { AppConfig::default() }
    }

    #[tauri::command]
    fn set_config(app: tauri::AppHandle, state: tauri::State<'_, AppState>, new_cfg: AppConfig) -> Result<(), String> {
        save_config(&app, &new_cfg).map_err(|e| e.to_string())?;
        if let Ok(mut guard) = state.0.lock() { *guard = new_cfg; }
        let _ = app.emit("config://changed", "ok");
        Ok(())
    }

    #[tauri::command]
    fn list_net_interfaces() -> Vec<String> {
        use sysinfo::Networks;
        let nets = Networks::new_with_refreshed_list();
        nets.iter().map(|(name, _)| name.to_string()).collect()
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_config, set_config, list_net_interfaces])
        .setup(|app| {
            use tauri::WindowEvent;
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
            use std::io::{BufRead, BufReader};
            use std::process::{Command, Stdio};
            use std::sync::{Arc, Mutex};
            use std::time::Instant as StdInstant;
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

            // 初始化配置并注入状态
            let cfg_arc: Arc<Mutex<AppConfig>> = Arc::new(Mutex::new(load_config(&app.handle())));
            app.manage(AppState(cfg_arc.clone()));

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
            let app_handle = app.handle();

            // --- Spawn sensor-bridge (.NET) and share latest output ---
            let bridge_data: Arc<Mutex<(Option<BridgeOut>, StdInstant)>> = Arc::new(Mutex::new((None, StdInstant::now())));
            {
                let bridge_data_c = bridge_data.clone();
                std::thread::spawn(move || {
                    // Resolve project root by walking up until we find `sensor-bridge/sensor-bridge.csproj`
                    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
                    let mut cursor = exe_dir.clone();
                    let mut found_root: Option<std::path::PathBuf> = None;
                    for _ in 0..6 {
                        if let Some(dir) = cursor {
                            let probe = dir.join("sensor-bridge").join("sensor-bridge.csproj");
                            if probe.exists() {
                                found_root = Some(dir.clone());
                                break;
                            }
                            cursor = dir.parent().map(|p| p.to_path_buf());
                        } else {
                            break;
                        }
                    }
                    let project_root = found_root
                        .or_else(|| exe_dir.and_then(|p| p.parent().map(|p| p.to_path_buf())))
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
                    eprintln!("[bridge] Using project_root: {}", project_root.display());

                    loop {
                        let dll_candidates = [
                            project_root.join("sensor-bridge/bin/Release/net8.0/sensor-bridge.dll"),
                            project_root.join("sensor-bridge/bin/Debug/net8.0/sensor-bridge.dll"),
                        ];
                        let exe_candidates = [
                            project_root.join("sensor-bridge/bin/Release/net8.0/sensor-bridge.exe"),
                            project_root.join("sensor-bridge/bin/Debug/net8.0/sensor-bridge.exe"),
                        ];

                        // 1) 优先使用 dll: dotnet <dll>
                        let mut child = if let Some(dll) = dll_candidates.iter().find(|p| p.exists()) {
                            eprintln!("[bridge] spawning via dotnet: {}", dll.display());
                            Command::new("dotnet")
                                .arg(dll)
                                .current_dir(project_root.clone())
                                .stdout(Stdio::piped())
                                .stderr(Stdio::piped())
                                .spawn()
                                .ok()
                        // 2) 其次尝试 exe 直接启动
                        } else if let Some(exe) = exe_candidates.iter().find(|p| p.exists()) {
                            eprintln!("[bridge] spawning exe: {}", exe.display());
                            Command::new(exe)
                                .current_dir(project_root.clone())
                                .stdout(Stdio::piped())
                                .stderr(Stdio::piped())
                                .spawn()
                                .ok()
                        } else {
                            // 3) 最后 fallback 到 dotnet run
                            eprintln!("[bridge] fallback to 'dotnet run --project sensor-bridge'");
                            Command::new("dotnet")
                                .args(["run", "--project", "sensor-bridge"])
                                .current_dir(project_root.clone())
                                .stdout(Stdio::piped())
                                .stderr(Stdio::piped())
                                .spawn()
                                .ok()
                        };

                        if let Some(ref mut child_proc) = child {
                            if let Some(stdout) = child_proc.stdout.take() {
                                let reader = BufReader::new(stdout);
                                for line in reader.lines().flatten() {
                                    if line.trim().is_empty() { continue; }
                                    if let Ok(parsed) = serde_json::from_str::<BridgeOut>(&line) {
                                        if let Ok(mut guard) = bridge_data_c.lock() {
                                            *guard = (Some(parsed), StdInstant::now());
                                        }
                                    } else {
                                        eprintln!("[bridge] Non-JSON line: {}", line);
                                    }
                                }
                            }
                            // Drain and print stderr if available for diagnostics
                            if let Some(mut stderr) = child_proc.stderr.take() {
                                std::thread::spawn(move || {
                                    use std::io::Read;
                                    let mut buf = String::new();
                                    if stderr.read_to_string(&mut buf).is_ok() {
                                        if !buf.trim().is_empty() {
                                            eprintln!("[bridge][stderr]\n{}", buf);
                                        }
                                    }
                                });
                            }
                            // Wait child and then respawn
                            let _ = child_proc.wait();
                            eprintln!("[bridge] bridge process exited, will respawn in 3s...");
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            continue;
                        } else {
                            eprintln!("[bridge] Failed to spawn sensor-bridge process, retry in 3s.");
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            continue;
                        }
                    }
                });
            }

            // --- Handle menu events ---
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "show_details" => {
                    println!("[tray] 点击 显示详情");
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                        let _ = win.eval("if (location.hash !== '#/details') { location.hash = '#/details'; }");
                    } else {
                        // 兜底：若没有主窗口（理论上不会发生），创建一个并直接进入 details
                        let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html#/details".into()))
                            .title("sys-sensor · 详情")
                            .inner_size(900.0, 600.0)
                            .min_inner_size(600.0, 400.0)
                            .resizable(true)
                            .build();
                    }
                }
                "quick_settings" => {
                    println!("[tray] 点击 快速设置");
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                        let _ = win.eval("if (location.hash !== '#/settings') { location.hash = '#/settings'; }");
                    } else {
                        let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html#/settings".into()))
                            .title("sys-sensor · 快速设置")
                            .inner_size(640.0, 520.0)
                            .min_inner_size(480.0, 360.0)
                            .resizable(true)
                            .build();
                    }
                }
                "about" => {
                    println!("[tray] 点击 关于我们");
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                        let _ = win.eval("if (location.hash !== '#/about') { location.hash = '#/about'; }");
                    } else {
                        let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html#/about".into()))
                            .title("关于 sys-sensor")
                            .inner_size(420.0, 360.0)
                            .min_inner_size(380.0, 320.0)
                            .resizable(false)
                            .build();
                    }
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
            let app_handle_c = app_handle.clone();
            let bridge_data_sampling = bridge_data.clone();
            let cfg_state_c = cfg_arc.clone();

            thread::spawn(move || {
                use std::time::{Duration, Instant};
                use sysinfo::{Networks, System};

                // 初始化 WMI 连接（在后台线程中初始化 COM）
                let wmi_temp_conn: Option<wmi::WMIConnection> = {
                    if let Ok(com) = wmi::COMLibrary::new() {
                        wmi::WMIConnection::with_namespace_path("ROOT\\WMI", com.into()).ok()
                    } else { None }
                };
                let wmi_fan_conn: Option<wmi::WMIConnection> = {
                    if let Ok(com) = wmi::COMLibrary::new() {
                        wmi::WMIConnection::new(com).ok() // 默认 ROOT\CIMV2
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
                    // 读取温度与风扇（优先桥接数据，其次 WMI）
                    let (bridge_cpu_temp, bridge_mobo_temp, bridge_cpu_fan, case_fan, bridge_cpu_fan_pct, case_fan_pct, is_admin, has_temp, has_temp_value, has_fan, has_fan_value) = {
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
                        if let Ok(guard) = bridge_data_sampling.lock() {
                            if let (Some(ref b), ts) = (&guard.0, guard.1) {
                                // 若超过 5s 未更新则视为过期
                                if ts.elapsed().as_secs() <= 5 {
                                    cpu_t = b.cpu_temp_c;
                                    mobo_t = b.mobo_temp_c;
                                    is_admin = b.is_admin;
                                    has_temp = b.has_temp;
                                    has_temp_value = b.has_temp_value;
                                    has_fan = b.has_fan;
                                    has_fan_value = b.has_fan_value;
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
                                }
                            }
                        }
                        (cpu_t, mobo_t, cpu_fan, case_fan, cpu_fan_pct, case_fan_pct, is_admin, has_temp, has_temp_value, has_fan, has_fan_value)
                    };

                    let temp_opt = bridge_cpu_temp.or_else(|| wmi_temp_conn.as_ref().and_then(|c| wmi_read_cpu_temp_c(c)));
                    let fan_opt = bridge_cpu_fan.or_else(|| wmi_fan_conn.as_ref().and_then(|c| wmi_read_fan_rpm(c)));

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

                    let fan_line = {
                        // 优先显示 RPM（桥接或 WMI），否则显示占空比
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
                            match (has_fan, has_fan_value, is_admin) {
                                (Some(true), Some(false), Some(false)) => s.push_str(" (需管理员)"),
                                (Some(true), Some(false), _) => s.push_str(" (无读数)"),
                                (Some(false), _, _) => s.push_str(" (不支持)"),
                                _ => {}
                            }
                            s
                        }
                    };
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

                    // 更新托盘“纯文本图标”：上=CPU温度(无则CPU%)，下=CPU%或内存%（可配置）
                    let bottom_pct = {
                        let show_mem = cfg_state_c.lock().ok().map(|c| c.tray_show_mem).unwrap_or(false);
                        if show_mem { mem_pct.round() as u32 } else { cpu_usage.round() as u32 }
                    };
                    let cpu_temp_i: Option<i32> = temp_opt.map(|v| v.round() as i32);
                    let icon_img: Image = make_tray_icon(cpu_temp_i, bottom_pct);
                    let _ = tray_c.set_icon(Some(icon_img));

                    // 广播到前端
                    let snapshot = SensorSnapshot {
                        cpu_usage,
                        mem_used_gb: used_gb as f32,
                        mem_total_gb: total_gb as f32,
                        mem_pct: mem_pct as f32,
                        net_rx_bps: ema_net_rx,
                        net_tx_bps: ema_net_tx,
                        disk_r_bps: ema_disk_r,
                        disk_w_bps: ema_disk_w,
                        cpu_temp_c: temp_opt.map(|v| v as f32),
                        mobo_temp_c: bridge_mobo_temp,
                        fan_rpm: fan_opt,
                        timestamp_ms: chrono::Local::now().timestamp_millis(),
                    };
                    let _ = app_handle_c.emit("sensor://snapshot", snapshot);

                    thread::sleep(Duration::from_secs(1));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
