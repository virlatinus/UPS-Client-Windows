use crate::icon::build_tray_icon;
use crate::ups::UpsStatus;
use eframe::egui;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// Commands emitted by user interaction with the System Tray icon or context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ShowInterface,
    Quit,
}

/// Manages the Windows System Tray icon, its right-click context menu, and immediate Win32 wakeup.
pub struct SystemTray {
    tray_icon: TrayIcon,
    _menu: Menu,
    _show_item: MenuItem,
    _quit_item: MenuItem,
    show_requested: Arc<AtomicBool>,
    quit_requested: Arc<AtomicBool>,
}

/// Finds the application's Win32 window handle (`HWND`) by cached value or window title.
#[cfg(windows)]
pub fn resolve_app_hwnd(cached_hwnd: &AtomicIsize) -> *mut core::ffi::c_void {
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindow};

    let existing = cached_hwnd.load(Ordering::Relaxed);
    if existing != 0 {
        let ptr = existing as *mut core::ffi::c_void;
        if unsafe { IsWindow(ptr) } != 0 {
            return ptr;
        }
    }

    let title: Vec<u16> = "OpenUPS Client for Windows\0".encode_utf16().collect();
    let found = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if !found.is_null() {
        cached_hwnd.store(found as isize, Ordering::Relaxed);
    }
    found
}

/// Immediately restores, shows, and focuses the main application window using Win32 APIs.
/// This works even when the window is hidden (`SW_HIDE`) and `winit` is not sending `WM_PAINT`.
#[cfg(windows)]
pub fn show_win32_window(cached_hwnd: &AtomicIsize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    let hwnd = resolve_app_hwnd(cached_hwnd);
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            ShowWindow(hwnd, SW_RESTORE);
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(not(windows))]
pub fn show_win32_window(_cached_hwnd: &AtomicIsize) {}

/// Hides the main application window to the System Tray using Win32 `SW_HIDE`.
#[cfg(windows)]
pub fn hide_win32_window(cached_hwnd: &AtomicIsize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    let hwnd = resolve_app_hwnd(cached_hwnd);
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

#[cfg(not(windows))]
pub fn hide_win32_window(_cached_hwnd: &AtomicIsize) {}

/// Posts `WM_CLOSE` to the main application window so `winit` exits cleanly.
#[cfg(windows)]
pub fn post_win32_close(cached_hwnd: &AtomicIsize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

    let hwnd = resolve_app_hwnd(cached_hwnd);
    if !hwnd.is_null() {
        unsafe {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        }
    }
}

#[cfg(not(windows))]
pub fn post_win32_close(_cached_hwnd: &AtomicIsize) {}

/// Returns the maximum client width and height (in physical pixels) that fit comfortably
/// inside the primary desktop work area (desktop bounds minus the Windows Taskbar and titlebar).
#[cfg(windows)]
pub fn query_work_area_max_client_px() -> Option<(f32, f32)> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};

    let mut work_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut work_rect as *mut RECT as *mut core::ffi::c_void,
            0,
        )
    };
    if ok != 0 {
        let work_w = (work_rect.right - work_rect.left) as f32;
        let work_h = (work_rect.bottom - work_rect.top) as f32;
        // Subtract non-client titlebar/frame height (~48px) and breathing margin (~16px)
        let max_client_w = (work_w - 28.0).max(400.0);
        let max_client_h = (work_h - 64.0).max(400.0);
        Some((max_client_w, max_client_h))
    } else {
        None
    }
}

#[cfg(not(windows))]
pub fn query_work_area_max_client_px() -> Option<(f32, f32)> {
    None
}

/// Ensures the window's outer rectangle stays within the desktop work area (above the taskbar).
#[cfg(windows)]
pub fn clamp_window_to_work_area(cached_hwnd: &AtomicIsize) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, SetWindowPos, SystemParametersInfoW, SPI_GETWORKAREA, SWP_NOACTIVATE,
        SWP_NOSIZE, SWP_NOZORDER,
    };

    let hwnd = resolve_app_hwnd(cached_hwnd);
    if hwnd.is_null() {
        return;
    }

    let mut work_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut win_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    unsafe {
        if SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut work_rect as *mut RECT as *mut core::ffi::c_void,
            0,
        ) == 0
        {
            return;
        }
        if GetWindowRect(hwnd, &mut win_rect) == 0 {
            return;
        }

        let win_w = win_rect.right - win_rect.left;
        let win_h = win_rect.bottom - win_rect.top;

        let mut new_x = win_rect.left;
        let mut new_y = win_rect.top;

        if new_x + win_w > work_rect.right {
            new_x = (work_rect.right - win_w).max(work_rect.left);
        }
        if new_x < work_rect.left {
            new_x = work_rect.left;
        }

        if new_y + win_h > work_rect.bottom {
            new_y = (work_rect.bottom - win_h).max(work_rect.top);
        }
        if new_y < work_rect.top {
            new_y = work_rect.top;
        }

        if new_x != win_rect.left || new_y != win_rect.top {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                new_x,
                new_y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
pub fn clamp_window_to_work_area(_cached_hwnd: &AtomicIsize) {}

impl SystemTray {
    /// Creates the System Tray icon with right-click menu options: `Show interface` and `Quit`.
    /// Registers immediate event handlers so clicking menu items works even while the window is hidden.
    pub fn new(
        egui_ctx: egui::Context,
        cached_hwnd: Arc<AtomicIsize>,
        allow_exit: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        let menu = Menu::new();

        let show_item = MenuItem::new("Show interface", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        let show_id = show_item.id().clone();
        let quit_id = quit_item.id().clone();

        menu.append(&show_item)
            .map_err(|e| format!("Failed to append 'Show interface' menu item: {}", e))?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|e| format!("Failed to append menu separator: {}", e))?;
        menu.append(&quit_item)
            .map_err(|e| format!("Failed to append 'Quit' menu item: {}", e))?;

        let icon = build_tray_icon([16, 185, 129])?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_menu_on_left_click(false)
            .with_tooltip("OpenUPS Client for Windows — Monitoring")
            .with_icon(icon)
            .build()
            .map_err(|e| format!("Failed to build system tray icon: {}", e))?;

        let show_requested = Arc::new(AtomicBool::new(false));
        let quit_requested = Arc::new(AtomicBool::new(false));

        // 1. Immediate callback for right-click context menu ("Show interface" & "Quit")
        {
            let show_id_clone = show_id;
            let quit_id_clone = quit_id;
            let show_req = Arc::clone(&show_requested);
            let quit_req = Arc::clone(&quit_requested);
            let hwnd_clone = Arc::clone(&cached_hwnd);
            let exit_clone = Arc::clone(&allow_exit);
            let ctx_clone = egui_ctx.clone();

            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                if event.id == show_id_clone {
                    show_req.store(true, Ordering::SeqCst);
                    show_win32_window(&hwnd_clone);
                    ctx_clone.request_repaint();
                } else if event.id == quit_id_clone {
                    quit_req.store(true, Ordering::SeqCst);
                    exit_clone.store(true, Ordering::SeqCst);
                    post_win32_close(&hwnd_clone);
                    ctx_clone.request_repaint();

                    // Fallback watchdog to guarantee termination even if window is hidden
                    std::thread::spawn(|| {
                        std::thread::sleep(Duration::from_millis(120));
                        std::process::exit(0);
                    });
                }
            }));
        }

        // 2. Immediate callback for System Tray icon Double-Click
        {
            let show_req = Arc::clone(&show_requested);
            let hwnd_clone = Arc::clone(&cached_hwnd);
            let ctx_clone = egui_ctx;

            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                if let TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } = event
                {
                    show_req.store(true, Ordering::SeqCst);
                    show_win32_window(&hwnd_clone);
                    ctx_clone.request_repaint();
                }
            }));
        }

        Ok(Self {
            tray_icon,
            _menu: menu,
            _show_item: show_item,
            _quit_item: quit_item,
            show_requested,
            quit_requested,
        })
    }

    /// Returns any pending command requested by the tray callbacks.
    pub fn poll_command(&self) -> Option<TrayCommand> {
        if self.quit_requested.swap(false, Ordering::SeqCst) {
            let _ = self.tray_icon.set_visible(false);
            return Some(TrayCommand::Quit);
        }
        if self.show_requested.swap(false, Ordering::SeqCst) {
            return Some(TrayCommand::ShowInterface);
        }
        None
    }

    /// Explicitly removes the icon from the system tray before process exit.
    pub fn hide_icon(&self) {
        let _ = self.tray_icon.set_visible(false);
    }

    /// Updates the tray tooltip and icon color to reflect the latest UPS telemetry.
    pub fn update_status(&self, status: Option<&UpsStatus>, error: Option<&str>) {
        match (status, error) {
            (Some(s), None) => {
                let tooltip = format!(
                    "UPS: {} ({})\nBattery: {}% | Runtime: {}\nLoad: {}% ({}W)",
                    s.model,
                    s.status_label,
                    s.battery_charge,
                    s.formatted_runtime(),
                    s.load,
                    s.output_watts
                );
                let _ = self.tray_icon.set_tooltip(Some(tooltip));

                let color = if !s.online || s.low_battery || s.should_shutdown {
                    [239, 68, 68] // Red alert
                } else if s.on_battery || s.discharging {
                    [245, 158, 11] // Amber warning
                } else {
                    [16, 185, 129] // Emerald online
                };
                if let Ok(icon) = build_tray_icon(color) {
                    let _ = self.tray_icon.set_icon(Some(icon));
                }
            }
            (_, Some(err)) => {
                let tooltip = format!("OpenUPS Client for Windows — Connection Error\n{}", err);
                let _ = self.tray_icon.set_tooltip(Some(tooltip));
                if let Ok(icon) = build_tray_icon([148, 163, 184]) {
                    let _ = self.tray_icon.set_icon(Some(icon));
                }
            }
            _ => {}
        }
    }
}
