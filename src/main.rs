#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod app;
mod config;
mod icon;
mod startup;
mod tray;
mod ups;
mod widgets;

use app::{UpsClientApp, BASE_WINDOW_HEIGHT, BASE_WINDOW_WIDTH};
use eframe::egui;
use std::env;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Support launching directly minimized to the System Tray (e.g., via Windows startup `--minimized`)
    let args: Vec<String> = env::args().collect();
    let start_minimized = args
        .iter()
        .any(|arg| arg == "--minimized" || arg == "--tray" || arg == "-m");

    let screenshot_path = args
        .windows(2)
        .find(|w| w[0] == "--screenshot")
        .map(|w| w[1].clone());

    let initial_cfg = config::AppConfig::load();
    let initial_scale = if screenshot_path.is_some() {
        1.0
    } else {
        initial_cfg.ui_scale.clamp(1.0, 2.0)
    };

    let viewport = egui::ViewportBuilder::default()
        .with_title("UPS Client Windows")
        .with_inner_size([
            BASE_WINDOW_WIDTH * initial_scale,
            BASE_WINDOW_HEIGHT * initial_scale,
        ])
        .with_resizable(false)
        .with_maximize_button(false)
        .with_visible(!start_minimized)
        .with_icon(icon::build_window_icon());

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "UPS Client Windows",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(UpsClientApp::new(
                cc,
                start_minimized,
                screenshot_path,
            )))
        }),
    )
}
