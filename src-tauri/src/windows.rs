use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
pub fn ui_create_window(
    app: AppHandle,
    label: String,
    route: Option<String>,
    always_on_top: Option<bool>,
    decorations: Option<bool>,
    width: Option<f64>,
    height: Option<f64>,
    x: Option<i32>,
    y: Option<i32>,
) -> Result<(), String> {
    if app.get_webview_window(&label).is_some() {
        return Ok(());
    }
    let url = route.unwrap_or_else(|| "/#/floating".to_string());
    let mut builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url.into()));
    if let Some(w) = width { builder = builder.inner_size(w, height.unwrap_or(360.0)); }
    if let Some(h) = height { builder = builder.inner_size(width.unwrap_or(480.0), h); }
    if let Some(dx) = x { builder = builder.position(dx as f64, y.unwrap_or(0) as f64); }
    if let Some(dy) = y { builder = builder.position(x.unwrap_or(0) as f64, dy as f64); }
    if let Some(dec) = decorations { builder = builder.decorations(dec); }
    // 置顶
    let win = builder.build().map_err(|e| e.to_string())?;
    if let Some(top) = always_on_top { let _ = win.set_always_on_top(top); }
    Ok(())
}

#[tauri::command]
pub fn ui_set_topmost(app: AppHandle, label: String, topmost: bool) -> Result<(), String> {
    let Some(win) = app.get_webview_window(&label) else { return Err("window not found".into()) };
    win.set_always_on_top(topmost).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ui_show(app: AppHandle, label: String) -> Result<(), String> {
    let Some(win) = app.get_webview_window(&label) else { return Err("window not found".into()) };
    let _ = win.show();
    let _ = win.set_focus();
    Ok(())
}

#[tauri::command]
pub fn ui_hide(app: AppHandle, label: String) -> Result<(), String> {
    let Some(win) = app.get_webview_window(&label) else { return Err("window not found".into()) };
    win.hide().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ui_snap_to_edge(app: AppHandle, label: String, edge: String, margin: Option<i32>) -> Result<(), String> {
    let Some(win) = app.get_webview_window(&label) else { return Err("window not found".into()) };
    let monitor = app.primary_monitor().map_err(|e| e.to_string())?.ok_or("no monitor")?;
    let size = monitor.size();
    let margin = margin.unwrap_or(2).max(0) as f64;
    // 读取窗口大小
    let inner = win.inner_size().map_err(|e| e.to_string())?;
    let (w, h) = (inner.width as f64, inner.height as f64);
    let pos = match edge.to_ascii_lowercase().as_str() {
        "left" => (margin, (size.height as f64 - h) / 2.0),
        "right" => (size.width as f64 - w - margin, (size.height as f64 - h) / 2.0),
        "top" => ((size.width as f64 - w) / 2.0, margin),
        "bottom" => ((size.width as f64 - w) / 2.0, size.height as f64 - h - margin),
        _ => (margin, margin),
    };
    win.set_position(tauri::PhysicalPosition::new(pos.0, pos.1)).map_err(|e| e.to_string())
}
