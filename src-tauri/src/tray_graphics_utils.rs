// 托盘图标绘制工具模块
// 包含字体定义、文本绘制和托盘图标生成功能

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

/// 绘制带阴影的文本到RGBA缓冲区
pub fn draw_text_rgba(buf: &mut [u8], w: usize, h: usize, x: usize, y: usize, scale: usize, gap: usize, text: &str) {
    // simple shadow
    draw_text_rgba_no_shadow(buf, w, h, x + 1, y + 1, scale, gap, text, [0, 0, 0, 180]);
    draw_text_rgba_no_shadow(buf, w, h, x, y, scale, gap, text, [255, 255, 255, 255]);
}

/// 绘制无阴影文本到RGBA缓冲区
pub fn draw_text_rgba_no_shadow(buf: &mut [u8], w: usize, h: usize, x: usize, y: usize, scale: usize, gap: usize, text: &str, color: [u8;4]) {
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

/// 生成托盘图标
/// 
/// # 参数
/// - `top_text_in`: 顶部文本（如温度）
/// - `bottom_text_in`: 底部文本（如内存使用率）
/// 
/// # 返回
/// 32x32像素的RGBA托盘图标
pub fn make_tray_icon(top_text_in: &str, bottom_text_in: &str) -> tauri::image::Image<'static> {
    let w: usize = 32;
    let h: usize = 32;
    let mut rgba = vec![0u8; w * h * 4]; // transparent background

    // 准备两行文本（由调用方传入）：上行与下行
    let top_initial = top_text_in.to_string();
    let bottom_initial = bottom_text_in.to_string();

    // 计算文本宽度：chars*FONT_W*scale + (chars-1)*gap
    let calc_text_w = |chars: usize, scale: usize, gap: usize| chars * FONT_W * scale + chars.saturating_sub(1) * gap;
    // 优先使用大字号 scale=2，gap=0；若仍溢出，则降到 scale=1，gap=1
    // 顶部文本优先保持大字号，必要时去掉单位字符('C')再判断
    let mut top = top_initial.clone();
    let mut top_scale = 2usize; let mut top_gap = 0usize;
    if calc_text_w(top.chars().count(), top_scale, top_gap) > w {
        if top.ends_with('C') { top.pop(); }
        if calc_text_w(top.chars().count(), top_scale, top_gap) > w { top_scale = 1; top_gap = 1; }
    }
    // 底部文本优先保持大字号，必要时去掉单位字符('%')再判断
    let mut bottom = bottom_initial.clone();
    let mut bot_scale = 2usize; let mut bot_gap = 0usize;
    if calc_text_w(bottom.chars().count(), bot_scale, bot_gap) > w {
        if bottom.ends_with('%') { bottom.pop(); }
        if calc_text_w(bottom.chars().count(), bot_scale, bot_gap) > w { bot_scale = 1; bot_gap = 1; }
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
