use crate::actions::execute_trigger_action;
use crate::config::{AppConfig, TriggerAction};
use crate::startup::{is_startup_enabled, set_startup_enabled};
use crate::tray::{
    clamp_window_to_work_area, hide_win32_window, query_work_area_max_client_px, resolve_app_hwnd,
    show_win32_window, SystemTray, TrayCommand,
};
use crate::ups::{fetch_ups_status, UpsStatus};
use crate::widgets::{
    draw_action_card, draw_big_kpi_tile, draw_glowing_battery_bar, draw_glowing_status_pill,
    draw_header_lightning, draw_icon_button, draw_large_trend_graph, draw_pill_toggle,
    draw_settings_toggle_header, draw_threshold_stepper, paint_aurora_background, VectorIcon,
};
use eframe::egui::{
    self, Align, Color32, FontId, Layout, Margin, RichText, Rounding, Stroke, Vec2,
};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Fixed logical window width (at 1.0x scale).
pub const BASE_WINDOW_WIDTH: f32 = 560.0;
/// Logical window height when the Settings panel is collapsed (default).
pub const BASE_WINDOW_HEIGHT: f32 = 608.0;
/// Logical window height when the Settings panel is expanded.
pub const EXPANDED_SETTINGS_HEIGHT: f32 = 868.0;
/// Logical window height when the Settings panel is expanded AND "Execute command" is selected.
pub const EXPANDED_COMMAND_HEIGHT: f32 = 930.0;

const MAX_HISTORY_POINTS: usize = 50;

/// Single historical sample for rendering real-time trendline graphs.
#[derive(Debug, Clone, Copy)]
struct TelemetrySample {
    battery_charge: f32,
    output_watts: f32,
    input_voltage: f32,
}

/// Shared configuration and command state used by the background polling thread.
struct SharedPollState {
    config: AppConfig,
    action_triggered_latch: bool,
    force_poll: bool,
}

/// Messages sent from the background UPS polling thread to the UI state.
enum PollUpdate {
    Status {
        result: Result<UpsStatus, String>,
        action_banner: Option<(String, bool)>,
    },
}

/// Main application state for `OpenUPS Client for Windows`.
pub struct UpsClientApp {
    config: AppConfig,
    settings_expanded: bool,
    scroll_to_settings: bool,
    needs_work_area_clamp: bool,
    pending_ui_scale: f32,
    latest_status: Option<UpsStatus>,
    history: VecDeque<TelemetrySample>,
    last_error: Option<String>,
    last_updated_text: String,
    status_banner: Option<(String, bool)>,
    allow_exit: Arc<AtomicBool>,
    cached_hwnd: Arc<AtomicIsize>,
    shared_poll: Arc<Mutex<SharedPollState>>,
    poll_rx: mpsc::Receiver<PollUpdate>,
    tray: Option<SystemTray>,
    screenshot_path: Option<String>,
    screenshot_frames: u32,
}

impl UpsClientApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        start_minimized: bool,
        screenshot_path: Option<String>,
    ) -> Self {
        Self::configure_modern_theme(&cc.egui_ctx);

        let mut config = AppConfig::load();
        config.normalize();
        if is_startup_enabled() {
            config.run_on_startup = true;
        }

        if screenshot_path.is_some() {
            config.ui_scale = 1.0;
        }
        let initial_scale = config.ui_scale.clamp(1.0, 2.0);
        cc.egui_ctx.set_zoom_factor(initial_scale);
        cc.egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                BASE_WINDOW_WIDTH,
                BASE_WINDOW_HEIGHT,
            )));

        let allow_exit = Arc::new(AtomicBool::new(false));
        let cached_hwnd = Arc::new(AtomicIsize::new(0));
        #[cfg(windows)]
        {
            let _ = resolve_app_hwnd(&cached_hwnd);
        }

        let tray = match SystemTray::new(
            cc.egui_ctx.clone(),
            Arc::clone(&cached_hwnd),
            Arc::clone(&allow_exit),
        ) {
            Ok(t) => Some(t),
            Err(err) => {
                log::error!("Failed to initialize system tray: {}", err);
                None
            }
        };

        let shared_poll = Arc::new(Mutex::new(SharedPollState {
            config: config.clone(),
            action_triggered_latch: false,
            force_poll: true,
        }));

        let (poll_tx, poll_rx) = mpsc::channel();

        // Background thread: Polls UPS HTTP endpoint periodically AND triggers threshold actions
        // even while the window is hidden in the System Tray.
        {
            let shared_clone = Arc::clone(&shared_poll);
            let exit_clone = Arc::clone(&allow_exit);
            let ctx_clone = cc.egui_ctx.clone();

            thread::spawn(move || {
                let mut last_poll = Instant::now() - Duration::from_secs(3600);
                while !exit_clone.load(Ordering::Relaxed) {
                    let (cfg, force) = {
                        let mut guard = shared_clone.lock().unwrap();
                        let f = guard.force_poll;
                        guard.force_poll = false;
                        (guard.config.clone(), f)
                    };

                    if force
                        || last_poll.elapsed() >= Duration::from_secs(cfg.poll_interval_secs.max(2))
                    {
                        let result = fetch_ups_status(&cfg.ups_status_url);
                        let mut action_banner = None;

                        if let Ok(ref status) = result {
                            let should_trigger =
                                status.should_trigger_action(cfg.threshold_percent);
                            let mut guard = shared_clone.lock().unwrap();
                            if should_trigger && !guard.action_triggered_latch {
                                guard.action_triggered_latch = true;
                                drop(guard);
                                match execute_trigger_action(&cfg, status) {
                                    Ok(msg) => {
                                        action_banner =
                                            Some((format!("Threshold triggered: {}", msg), false));
                                    }
                                    Err(err) => {
                                        action_banner =
                                            Some((format!("Trigger action error: {}", err), true));
                                    }
                                }
                            } else if !should_trigger && status.online {
                                guard.action_triggered_latch = false;
                            }
                        }

                        let _ = poll_tx.send(PollUpdate::Status {
                            result,
                            action_banner,
                        });
                        last_poll = Instant::now();
                        ctx_clone.request_repaint();
                    }

                    thread::sleep(Duration::from_millis(250));
                }
            });
        }

        if start_minimized {
            hide_win32_window(&cached_hwnd);
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        Self {
            pending_ui_scale: initial_scale,
            settings_expanded: false, // Collapsed by default so big stats & trendlines take center stage
            scroll_to_settings: false,
            needs_work_area_clamp: true,
            config,
            latest_status: None,
            history: VecDeque::with_capacity(MAX_HISTORY_POINTS),
            last_error: None,
            last_updated_text: String::from("Connecting to UPS endpoint..."),
            status_banner: None,
            allow_exit,
            cached_hwnd,
            shared_poll,
            poll_rx,
            tray,
            screenshot_path,
            screenshot_frames: 0,
        }
    }

    /// Writes an `egui::ColorImage` to a 32-bit uncompressed BMP file.
    fn save_color_image_bmp(image: &egui::ColorImage, path: &str) -> Result<(), String> {
        let w = image.width() as u32;
        let h = image.height() as u32;
        let pixel_bytes = w * h * 4;
        let file_size = 54 + pixel_bytes;

        let mut buf = Vec::with_capacity(file_size as usize);
        buf.extend_from_slice(b"BM");
        buf.extend_from_slice(&file_size.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&54u32.to_le_bytes());
        buf.extend_from_slice(&40u32.to_le_bytes());
        buf.extend_from_slice(&(w as i32).to_le_bytes());
        buf.extend_from_slice(&(-(h as i32)).to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&32u16.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&pixel_bytes.to_le_bytes());
        buf.extend_from_slice(&2835u32.to_le_bytes());
        buf.extend_from_slice(&2835u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());

        for px in &image.pixels {
            let [r, g, b, _a] = px.to_srgba_unmultiplied();
            buf.push(b);
            buf.push(g);
            buf.push(r);
            buf.push(255);
        }

        std::fs::write(path, buf).map_err(|e| e.to_string())
    }

    /// Applies a sleek, modern Windows 11 dark theme to `egui`.
    fn configure_modern_theme(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(13, 18, 30);
        visuals.window_fill = Color32::from_rgb(13, 18, 30);
        visuals.faint_bg_color = Color32::from_rgb(20, 29, 46);
        visuals.extreme_bg_color = Color32::from_rgb(9, 13, 22);
        visuals.selection.bg_fill = Color32::from_rgb(56, 189, 248);
        visuals.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(11, 17, 30));

        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(22, 32, 50);
        visuals.widgets.noninteractive.fg_stroke =
            Stroke::new(1.0_f32, Color32::from_rgb(226, 232, 240));
        visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

        visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 39, 60);
        visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(24, 34, 52);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85));
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(241, 245, 249));
        visuals.widgets.inactive.rounding = Rounding::same(7.0);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 52, 78);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(34, 48, 72);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(96, 165, 250));
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
        visuals.widgets.hovered.rounding = Rounding::same(7.0);

        visuals.widgets.active.bg_fill = Color32::from_rgb(56, 189, 248);
        visuals.widgets.active.weak_bg_fill = Color32::from_rgb(14, 165, 233);
        visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
        visuals.widgets.active.rounding = Rounding::same(7.0);

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(12.0, 6.0);
        ctx.set_style(style);
    }

    /// Computes desired logical window height based on whether Settings (and Execute command) are expanded.
    fn desired_logical_height(&self) -> f32 {
        if !self.settings_expanded {
            BASE_WINDOW_HEIGHT
        } else if self.config.action == TriggerAction::ExecuteCommand {
            EXPANDED_COMMAND_HEIGHT
        } else {
            EXPANDED_SETTINGS_HEIGHT
        }
    }

    /// Ensures the OS window size matches `BASE_WINDOW_WIDTH x desired_logical_height()`
    /// multiplied by the active `zoom_factor` (`1.0x`..=`2.0x`), while clamping the maximum
    /// size to the available desktop work area (desktop bounds minus the Windows Taskbar).
    fn sync_viewport_geometry(&mut self, ctx: &egui::Context) {
        let desired_scale = self.config.ui_scale.clamp(1.0, 2.0);

        // Step 1: If egui's active zoom_factor hasn't updated to desired_scale yet, set it and repaint.
        // Note: egui applies `set_zoom_factor` at the start of the NEXT pass.
        if (ctx.zoom_factor() - desired_scale).abs() > 0.005 {
            ctx.set_zoom_factor(desired_scale);
            ctx.request_repaint();
            return;
        }

        // Step 2: Clamp desired logical dimensions so the physical window never exceeds the
        // desktop work area (desktop height/width minus taskbar and titlebar).
        let ppp = ctx.pixels_per_point().max(1.0);
        let (max_logical_w, max_logical_h) =
            if let Some((max_w_px, max_h_px)) = query_work_area_max_client_px() {
                (max_w_px / ppp, max_h_px / ppp)
            } else {
                (BASE_WINDOW_WIDTH, self.desired_logical_height())
            };

        let target_width = BASE_WINDOW_WIDTH.min(max_logical_w);
        let target_height = self.desired_logical_height().min(max_logical_h);

        let current_rect = ctx.screen_rect();
        if (current_rect.width() - target_width).abs() > 2.5
            || (current_rect.height() - target_height).abs() > 2.5
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                target_width,
                target_height,
            )));
            self.needs_work_area_clamp = true;
            ctx.request_repaint();
        } else if self.needs_work_area_clamp {
            self.needs_work_area_clamp = false;
            clamp_window_to_work_area(&self.cached_hwnd);
        }
    }

    /// Shows and focuses the main settings window from the System Tray.
    fn show_window(&self, ctx: &egui::Context) {
        show_win32_window(&self.cached_hwnd);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    /// Hides the window to the System Tray without terminating the process.
    fn hide_to_tray(&self, ctx: &egui::Context) {
        #[cfg(windows)]
        {
            let _ = resolve_app_hwnd(&self.cached_hwnd);
        }
        hide_win32_window(&self.cached_hwnd);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    /// Pushes updated configuration to the background polling worker.
    fn sync_shared_poll_config(&self, reset_latch: bool, force_poll: bool) {
        if let Ok(mut guard) = self.shared_poll.lock() {
            guard.config = self.config.clone();
            if reset_latch {
                guard.action_triggered_latch = false;
            }
            if force_poll {
                guard.force_poll = true;
            }
        }
    }

    /// Saves the current configuration, updates startup registry, and notifies the polling worker.
    fn apply_and_save_settings(&mut self) {
        self.config.normalize();

        if let Err(err) = set_startup_enabled(self.config.run_on_startup) {
            self.status_banner = Some((format!("Startup registry error: {}", err), true));
            return;
        }

        self.sync_shared_poll_config(false, true);

        match self.config.save() {
            Ok(()) => {
                self.status_banner = Some(("Settings saved and applied.".to_string(), false));
            }
            Err(err) => {
                self.status_banner = Some((err, true));
            }
        }
    }

    /// Processes incoming UPS telemetry updates and records trendline samples.
    fn process_poll_updates(&mut self) {
        while let Ok(PollUpdate::Status {
            result,
            action_banner,
        }) = self.poll_rx.try_recv()
        {
            if let Some(banner) = action_banner {
                self.status_banner = Some(banner);
            }

            match result {
                Ok(status) => {
                    let sample = TelemetrySample {
                        battery_charge: status.battery_charge as f32,
                        output_watts: status.output_watts as f32,
                        input_voltage: status.input_voltage as f32,
                    };

                    if self.history.is_empty() {
                        for _ in 0..16 {
                            self.history.push_back(sample);
                        }
                    } else {
                        if self.history.len() >= MAX_HISTORY_POINTS {
                            self.history.pop_front();
                        }
                        self.history.push_back(sample);
                    }

                    if let Some(ref tray) = self.tray {
                        tray.update_status(Some(&status), None);
                    }

                    self.latest_status = Some(status);
                    self.last_error = None;
                    self.last_updated_text = "Live • Updated just now".to_string();
                }
                Err(err) => {
                    if let Some(ref tray) = self.tray {
                        tray.update_status(self.latest_status.as_ref(), Some(&err));
                    }
                    self.last_error = Some(err);
                }
            }
        }
    }
}

impl eframe::App for UpsClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(windows)]
        if self.cached_hwnd.load(Ordering::Relaxed) == 0 {
            let _ = resolve_app_hwnd(&self.cached_hwnd);
        }

        // 1. Poll System Tray events
        let mut should_quit = false;
        if let Some(ref tray) = self.tray {
            while let Some(cmd) = tray.poll_command() {
                match cmd {
                    TrayCommand::ShowInterface => {
                        self.show_window(ctx);
                    }
                    TrayCommand::Quit => {
                        should_quit = true;
                    }
                }
            }
        }

        if should_quit || self.allow_exit.load(Ordering::SeqCst) {
            if let Some(ref tray) = self.tray {
                tray.hide_icon();
            }
            self.tray = None;
            std::process::exit(0);
        }

        // 2. Intercept window close [X] button to minimize to System Tray unless Quit was chosen
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_exit.load(Ordering::SeqCst)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_to_tray(ctx);
        }

        // 3. Process background UPS status poll results
        self.process_poll_updates();

        // Optional automated screenshot capture via `--screenshot <path>`
        if let Some(ref out_path) = self.screenshot_path.clone() {
            let events = ctx.input(|i| i.raw.events.clone());
            for ev in events {
                if let egui::Event::Screenshot { image, .. } = ev {
                    let _ = Self::save_color_image_bmp(&image, out_path);
                    if let Some(ref tray) = self.tray {
                        tray.hide_icon();
                    }
                    self.tray = None;
                    std::process::exit(0);
                }
            }

            self.screenshot_frames += 1;
            if (self.latest_status.is_some() && self.screenshot_frames >= 8)
                || self.screenshot_frames >= 90
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            ctx.request_repaint_after(Duration::from_millis(50));
        }

        // 4. Render modern UI with prominent telemetry stats, large trendlines, and collapsible Settings
        let panel_frame = egui::Frame::none()
            .fill(Color32::from_rgb(13, 18, 30))
            .inner_margin(Margin::symmetric(16.0, 12.0));

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                paint_aurora_background(ui, ui.max_rect());

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // =============================================================
                        // HEADER BAR
                        // =============================================================
                        ui.horizontal(|ui| {
                            draw_header_lightning(ui);
                            ui.add_space(2.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(
                                    RichText::new("OpenUPS Client for Windows")
                                        .font(FontId::proportional(21.0))
                                        .strong()
                                        .color(Color32::from_rgb(248, 250, 252)),
                                );
                                ui.label(
                                    RichText::new(&self.last_updated_text)
                                        .font(FontId::proportional(11.5))
                                        .color(Color32::from_rgb(148, 163, 184)),
                                );
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let (badge_text, badge_bg, text_col) =
                                    match (&self.latest_status, &self.last_error) {
                                        (_, Some(_)) => (
                                            "OFFLINE",
                                            Color32::from_rgb(239, 68, 68),
                                            Color32::WHITE,
                                        ),
                                        (Some(s), None) if s.low_battery || s.should_shutdown => (
                                            "LOW BATTERY",
                                            Color32::from_rgb(239, 68, 68),
                                            Color32::WHITE,
                                        ),
                                        (Some(s), None) if s.on_battery || s.discharging => (
                                            "ON BATTERY",
                                            Color32::from_rgb(245, 158, 11),
                                            Color32::from_rgb(15, 23, 42),
                                        ),
                                        (Some(s), None) => (
                                            s.status_label.as_str(),
                                            Color32::from_rgb(52, 211, 153),
                                            Color32::from_rgb(6, 46, 34),
                                        ),
                                        (None, None) => (
                                            "CONNECTING",
                                            Color32::from_rgb(71, 85, 105),
                                            Color32::WHITE,
                                        ),
                                    };

                                draw_glowing_status_pill(ui, badge_text, badge_bg, text_col);
                            });
                        });

                        ui.add_space(6.0);

                        // =============================================================
                        // HERO TELEMETRY & LARGE TRENDLINE GRAPHS CARD
                        // =============================================================
                        egui::Frame::none()
                            .fill(Color32::from_rgb(20, 29, 46))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(42, 56, 82)))
                            .rounding(Rounding::same(12.0))
                            .inner_margin(Margin::symmetric(16.0, 12.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());

                                let status = self.latest_status.clone().unwrap_or_default();
                                let beeper_label = if status.beeper.eq_ignore_ascii_case("enabled")
                                {
                                    "On"
                                } else {
                                    status.beeper.as_str()
                                };

                                // Row 1: Model & Prominent Runtime Readout
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("Model: {}", status.model))
                                            .font(FontId::proportional(14.5))
                                            .strong()
                                            .color(Color32::from_rgb(241, 245, 249)),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "Runtime: {}",
                                                status.formatted_runtime()
                                            ))
                                            .font(FontId::proportional(15.0))
                                            .strong()
                                            .color(Color32::from_rgb(56, 189, 248)),
                                        );
                                    });
                                });

                                ui.add_space(4.0);

                                // Row 2: Glowing Emerald Battery Charge Bar
                                draw_glowing_battery_bar(
                                    ui,
                                    status.battery_charge,
                                    self.config.threshold_percent,
                                    status.on_battery,
                                );

                                ui.add_space(6.0);

                                // Row 3: 3 Prominent KPI Stat Cards (Big Numbers + Vector Icons)
                                let kpi_gap = 8.0;
                                let kpi_w =
                                    ((ui.available_width() - kpi_gap * 2.0) / 3.0).max(110.0);

                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = kpi_gap;

                                    draw_big_kpi_tile(
                                        ui,
                                        kpi_w,
                                        VectorIcon::LoadWeight,
                                        "POWER LOAD",
                                        &format!("{}W ({}%)", status.output_watts, status.load),
                                        &format!("Capacity: {}W", status.realpower_nominal),
                                        Color32::from_rgb(56, 189, 248),
                                    );

                                    draw_big_kpi_tile(
                                        ui,
                                        kpi_w,
                                        VectorIcon::AcOutlet,
                                        "AC LINE I/O",
                                        &format!("{:.1}V AC", status.input_voltage),
                                        &format!("Output: {:.1}V AC", status.output_voltage),
                                        Color32::from_rgb(52, 211, 153),
                                    );

                                    draw_big_kpi_tile(
                                        ui,
                                        kpi_w,
                                        VectorIcon::BatteryMini,
                                        "BATTERY DC",
                                        &format!("{:.1}V DC", status.battery_voltage),
                                        &format!(
                                            "Nom {:.0}V • Beeper: {}",
                                            status.battery_voltage_nominal, beeper_label
                                        ),
                                        Color32::from_rgb(167, 139, 250),
                                    );
                                });

                                ui.add_space(8.0);

                                // Row 4: Large, High-Visibility Trendline Graphs
                                let watts_hist: Vec<f32> =
                                    self.history.iter().map(|s| s.output_watts).collect();
                                let volt_hist: Vec<f32> =
                                    self.history.iter().map(|s| s.input_voltage).collect();
                                let batt_hist: Vec<f32> =
                                    self.history.iter().map(|s| s.battery_charge).collect();
                                let max_watts =
                                    (status.realpower_nominal.max(300) as f32 * 0.8).max(300.0);

                                // Primary Full-Width Power Load Graph (115px tall)
                                draw_large_trend_graph(
                                    ui,
                                    ui.available_width(),
                                    112.0,
                                    "POWER LOAD TREND (WATTS)",
                                    &format!("{}W  ({}%)", status.output_watts, status.load),
                                    &format!(
                                        "Nominal Capacity: {}W  •  Active Output",
                                        status.realpower_nominal
                                    ),
                                    &watts_hist,
                                    0.0,
                                    max_watts,
                                    "W",
                                    Color32::from_rgb(56, 189, 248),
                                    None,
                                );

                                ui.add_space(6.0);

                                // Secondary Side-by-Side Large Graphs: AC Line Voltage & Battery Charge (108px tall)
                                let half_gap = 8.0;
                                let half_w = ((ui.available_width() - half_gap) * 0.5).max(140.0);

                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = half_gap;

                                    draw_large_trend_graph(
                                        ui,
                                        half_w,
                                        106.0,
                                        "AC VOLTAGE TREND",
                                        &format!("{:.1}V", status.input_voltage),
                                        &format!("Out: {:.1}V AC", status.output_voltage),
                                        &volt_hist,
                                        95.0,
                                        135.0,
                                        "V",
                                        Color32::from_rgb(52, 211, 153),
                                        None,
                                    );

                                    draw_large_trend_graph(
                                        ui,
                                        half_w,
                                        106.0,
                                        "BATTERY CHARGE",
                                        &format!("{}%", status.battery_charge),
                                        &format!(
                                            "Alert Threshold: {}%",
                                            self.config.threshold_percent
                                        ),
                                        &batt_hist,
                                        0.0,
                                        100.0,
                                        "%",
                                        Color32::from_rgb(167, 139, 250),
                                        Some(self.config.threshold_percent as f32),
                                    );
                                });
                            });

                        ui.add_space(8.0);

                        // =============================================================
                        // COLLAPSIBLE SETTINGS CARD (Collapsed by default, expands on click)
                        // =============================================================
                        egui::Frame::none()
                            .fill(Color32::from_rgb(20, 29, 46))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(42, 56, 82)))
                            .rounding(Rounding::same(12.0))
                            .inner_margin(Margin::symmetric(18.0, 10.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());

                                // Accordion Header: Clickable [Chevron + Settings] on left, Scale Slider on right
                                let summary = if self.settings_expanded {
                                    String::from("Click to collapse")
                                } else {
                                    format!(
                                        "Threshold: {}% • Click to configure",
                                        self.config.threshold_percent
                                    )
                                };

                                ui.horizontal(|ui| {
                                    if draw_settings_toggle_header(
                                        ui,
                                        self.settings_expanded,
                                        &summary,
                                    )
                                    .clicked()
                                    {
                                        self.settings_expanded = !self.settings_expanded;
                                        if self.settings_expanded {
                                            self.scroll_to_settings = true;
                                        }
                                    }

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let slider_resp = ui.add(
                                            egui::Slider::new(
                                                &mut self.pending_ui_scale,
                                                1.0..=2.0,
                                            )
                                            .step_by(0.05)
                                            .custom_formatter(|n, _| format!("{:.2}x", n)),
                                        );
                                        if !slider_resp.dragged()
                                            && (self.config.ui_scale - self.pending_ui_scale).abs()
                                                > 0.005
                                        {
                                            self.config.ui_scale = self.pending_ui_scale;
                                            let _ = self.config.save();
                                        }
                                        ui.label(
                                            RichText::new("Scale")
                                                .size(12.0)
                                                .color(Color32::from_rgb(148, 163, 184)),
                                        );
                                    });
                                });

                                if self.settings_expanded {
                                    ui.separator();
                                    ui.add_space(2.0);

                                    // Run on Windows startup
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("Run on Windows startup")
                                                .font(FontId::proportional(13.2))
                                                .color(Color32::from_rgb(226, 232, 240)),
                                        );
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if draw_pill_toggle(
                                                    ui,
                                                    &mut self.config.run_on_startup,
                                                )
                                                .changed()
                                                {
                                                    let _ = set_startup_enabled(
                                                        self.config.run_on_startup,
                                                    );
                                                    let _ = self.config.save();
                                                }
                                            },
                                        );
                                    });

                                    ui.add_space(4.0);

                                    // UPS Status URL
                                    ui.label(
                                        RichText::new("UPS Status URL")
                                            .font(FontId::proportional(13.0))
                                            .color(Color32::from_rgb(226, 232, 240)),
                                    );
                                    ui.horizontal(|ui| {
                                        let btn_w = 78.0;
                                        let input_w = (ui.available_width()
                                            - btn_w
                                            - ui.spacing().item_spacing.x)
                                            .max(120.0);
                                        ui.add_sized(
                                            [input_w, 32.0],
                                            egui::TextEdit::singleline(
                                                &mut self.config.ups_status_url,
                                            )
                                            .hint_text("http://192.168.2.161:8000/api/ups")
                                            .vertical_align(Align::Center),
                                        );
                                        if draw_icon_button(ui, None, "Poll Now", true).clicked() {
                                            self.sync_shared_poll_config(false, true);
                                        }
                                    });

                                    ui.add_space(4.0);

                                    // Battery Alert Threshold (Stepped Track + 10%..100% Pill Badges)
                                    ui.label(
                                        RichText::new("Battery Alert Threshold")
                                            .font(FontId::proportional(13.0))
                                            .color(Color32::from_rgb(226, 232, 240)),
                                    );
                                    if draw_threshold_stepper(
                                        ui,
                                        &mut self.config.threshold_percent,
                                    ) {
                                        self.sync_shared_poll_config(true, false);
                                    }

                                    ui.add_space(2.0);

                                    // Action on Threshold (2x2 Grid of Icon + Pill-Toggle Cards)
                                    ui.label(
                                        RichText::new("Action on Threshold")
                                            .font(FontId::proportional(13.0))
                                            .color(Color32::from_rgb(226, 232, 240)),
                                    );

                                    let col_gap = 10.0;
                                    let card_w =
                                        ((ui.available_width() - col_gap) * 0.5).max(140.0);

                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = col_gap;
                                        if draw_action_card(
                                            ui,
                                            card_w,
                                            VectorIcon::Bell,
                                            "Alert with native notification",
                                            self.config.action == TriggerAction::NativeNotification,
                                        ) {
                                            self.config.action = TriggerAction::NativeNotification;
                                            self.sync_shared_poll_config(false, false);
                                        }
                                        if draw_action_card(
                                            ui,
                                            card_w,
                                            VectorIcon::Speaker,
                                            "Play a sound",
                                            self.config.action == TriggerAction::PlaySound,
                                        ) {
                                            self.config.action = TriggerAction::PlaySound;
                                            self.sync_shared_poll_config(false, false);
                                        }
                                    });

                                    ui.add_space(2.0);

                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = col_gap;
                                        if draw_action_card(
                                            ui,
                                            card_w,
                                            VectorIcon::PowerOff,
                                            "Graceful shutdown",
                                            self.config.action == TriggerAction::GracefulShutdown,
                                        ) {
                                            self.config.action = TriggerAction::GracefulShutdown;
                                            self.sync_shared_poll_config(false, false);
                                        }
                                        if draw_action_card(
                                            ui,
                                            card_w,
                                            VectorIcon::Terminal,
                                            "Execute command",
                                            self.config.action == TriggerAction::ExecuteCommand,
                                        ) {
                                            self.config.action = TriggerAction::ExecuteCommand;
                                            self.sync_shared_poll_config(false, false);
                                        }
                                    });

                                    // Dynamic Command Input Row when "Execute command" is selected
                                    if self.config.action == TriggerAction::ExecuteCommand {
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new("Command to execute")
                                                .font(FontId::proportional(12.2))
                                                .color(Color32::from_rgb(56, 189, 248)),
                                        );
                                        if ui
                                    .add_sized(
                                        [ui.available_width(), 32.0],
                                        egui::TextEdit::singleline(&mut self.config.custom_command)
                                            .hint_text(
                                                "powershell.exe -File C:\\scripts\\ups-alert.ps1",
                                            )
                                            .vertical_align(Align::Center),
                                    )
                                    .changed()
                                {
                                    self.sync_shared_poll_config(false, false);
                                }
                                    }
                                }
                            });

                        ui.add_space(6.0);

                        // =============================================================
                        // FOOTER STATUS & ACTION BUTTONS
                        // =============================================================
                        let banner_msg = self
                            .status_banner
                            .as_ref()
                            .map(|(m, is_err)| (m.as_str(), *is_err))
                            .or_else(|| self.last_error.as_ref().map(|e| (e.as_str(), true)));

                        if let Some((msg, is_err)) = banner_msg {
                            let color = if is_err {
                                Color32::from_rgb(248, 113, 113)
                            } else {
                                Color32::from_rgb(52, 211, 153)
                            };
                            ui.label(
                                RichText::new(msg)
                                    .font(FontId::proportional(12.0))
                                    .color(color),
                            );
                            ui.add_space(2.0);
                        } else {
                            ui.add_space(16.0);
                        }

                        ui.horizontal(|ui| {
                            if draw_icon_button(
                                ui,
                                Some(VectorIcon::FloppySave),
                                "Save Settings",
                                true,
                            )
                            .clicked()
                            {
                                self.apply_and_save_settings();
                            }

                            if draw_icon_button(
                                ui,
                                Some(VectorIcon::PlayTriangle),
                                "Test Action",
                                false,
                            )
                            .clicked()
                            {
                                let sample_status = self.latest_status.clone().unwrap_or_default();
                                match execute_trigger_action(&self.config, &sample_status) {
                                    Ok(msg) => {
                                        self.status_banner =
                                            Some((format!("Test: {}", msg), false));
                                    }
                                    Err(err) => {
                                        self.status_banner =
                                            Some((format!("Test error: {}", err), true));
                                    }
                                }
                            }

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if draw_icon_button(ui, None, "Minimize to Tray", false).clicked() {
                                    self.hide_to_tray(ctx);
                                }
                            });
                        });

                        if self.scroll_to_settings {
                            self.scroll_to_settings = false;
                            ui.scroll_to_cursor(Some(Align::BOTTOM));
                        }
                    });
            });

        // 5. Keep fixed window geometry & zoom factor synchronized at the end of the pass
        self.sync_viewport_geometry(ctx);
    }
}
