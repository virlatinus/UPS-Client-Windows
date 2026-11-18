use eframe::egui;
use tray_icon::Icon;

/// Generates a 32x32 RGBA battery/lightning icon for the System Tray and window title bar.
pub fn generate_rgba_icon(accent_rgb: [u8; 3]) -> (Vec<u8>, u32, u32) {
    let width: u32 = 32;
    let height: u32 = 32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;

            // Battery cap at top: x in 12..20, y in 2..5
            let in_cap = (12..20).contains(&x) && (2..5).contains(&y);
            // Battery body: x in 7..25, y in 5..29
            let in_body = (7..25).contains(&x) && (5..29).contains(&y);
            // Inner battery area: x in 9..23, y in 7..27
            let in_inner = (9..23).contains(&x) && (7..27).contains(&y);

            // Stylized lightning bolt inside battery
            let in_bolt = ((9..=17).contains(&y) && (x >= (21 - (y - 9) / 2) && x <= 18))
                || ((16..=24).contains(&y) && (x >= 13 && x <= (16 + (24 - y) / 2)));

            if in_bolt {
                // Bright white/gold lightning bolt
                rgba[idx] = 255;
                rgba[idx + 1] = 245;
                rgba[idx + 2] = 157;
                rgba[idx + 3] = 255;
            } else if in_inner {
                // Accent fill
                rgba[idx] = accent_rgb[0];
                rgba[idx + 1] = accent_rgb[1];
                rgba[idx + 2] = accent_rgb[2];
                rgba[idx + 3] = 255;
            } else if in_cap || in_body {
                // Crisp outer frame
                rgba[idx] = 230;
                rgba[idx + 1] = 236;
                rgba[idx + 2] = 245;
                rgba[idx + 3] = 255;
            }
        }
    }

    (rgba, width, height)
}

/// Builds a `tray_icon::Icon` with the default emerald/teal online color.
pub fn build_tray_icon(accent_rgb: [u8; 3]) -> Result<Icon, String> {
    let (rgba, width, height) = generate_rgba_icon(accent_rgb);
    Icon::from_rgba(rgba, width, height).map_err(|e| format!("Failed to create tray icon: {}", e))
}

/// Builds an `egui::IconData` for the application window.
pub fn build_window_icon() -> egui::IconData {
    let (rgba, width, height) = generate_rgba_icon([16, 185, 129]);
    egui::IconData {
        rgba,
        width,
        height,
    }
}
