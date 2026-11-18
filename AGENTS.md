# AGENTS.md — AI Agent Guide for `OpenUPS Client for Windows`

## Project Overview

**OpenUPS Client for Windows** (`UPS-Client-Windows`) is a modern Windows desktop & System Tray application written in Rust that monitors any compatible UPS endpoint via HTTP and triggers alerts or actions on specific events.

- **Repository**: `https://github.com/virlatinus/UPS-Client-Windows`
- **Language / Edition**: Rust (2021 Edition)
- **Target OS**: Windows 10 / Windows 11 (`x86_64-pc-windows-msvc`)
- **License**: MIT

---

## Architecture & Module Map

```text
UPS-Client-Windows/
├── .github/workflows/
│   ├── ci.yml                # GitHub Actions CI pipeline (fmt, clippy, test, release build)
│   └── release.yml           # Release workflow (vX.Y.Z tags): builds binary, .exe (NSIS), and .msi (WiX)
├── assets/
│   └── screenshot.png        # Application UI screenshot embedded in README.md
├── installer/
│   ├── ups-client-windows.nsi# Open-source NSIS script for generating the Windows .exe setup installer
│   └── build-installers.ps1  # Local & CI PowerShell script to build binary, .exe, .msi, and SHA256SUMS
├── wix/
│   ├── main.wxs              # Open-source WiX Toolset / cargo-wix manifest for generating the .msi package
│   └── License.rtf           # RTF license displayed by the .msi installer UI
├── Cargo.toml                # Crate metadata and dependencies
├── src/
│   ├── main.rs               # Entry point, CLI flags (--minimized, --screenshot), eframe viewport setup
│   ├── app.rs                # Main eframe::App UI, background polling worker, tray event loop
│   ├── widgets.rs            # Custom painted vector icons, KPI tiles, trendline graphs, stepper & toggles
│   ├── config.rs             # AppConfig & TriggerAction serialization, 10% step & UI scale validation
│   ├── ups.rs                # UpsStatus JSON model, HTTP polling via ureq, unit tests
│   ├── actions.rs            # Action handlers (Toast notification, Win32 sound, shutdown, command)
│   ├── startup.rs            # Windows Registry startup management (HKCU\...\Run)
│   ├── tray.rs               # System Tray icon, Win32 window/work-area management, right-click menu
│   └── icon.rs               # Procedural RGBA icon generator for tray and window icons
├── AGENTS.md                 # Instructions and context for AI coding agents
├── README.md                 # User and developer documentation
└── LICENSE                   # MIT License
```

---

## Core Functional Invariants

When modifying this repository, preserve these user-facing behaviors:

1. **System Tray Behavior**:
   - The app remains active in the System Tray when the main window is closed (`ViewportCommand::CancelClose` + `ViewportCommand::Visible(false)`).
   - **Double-clicking** (`TrayIconEvent::DoubleClick` with `MouseButton::Left`) the System Tray icon restores and focuses the settings window.
   - **Right-clicking** the System Tray icon displays the context menu with:
     - `Show interface`
     - `Quit`
2. **Startup Registration**:
   - Toggling **Run on Windows startup** updates `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` under the value name `"UPS-Client-Windows"`, passing `--minimized` so startup launches directly into the System Tray.
3. **Threshold & Actions**:
   - Thresholds are configured strictly in **10% increments** (`10%`, `20%`, ..., `100%`).
   - Supported actions (`TriggerAction`):
     - `NativeNotification`: Native Windows Toast notification (`winrt-notification`).
     - `PlaySound`: Native Windows audio alert (`windows-sys` `PlaySoundW` + `MessageBeep` + `Beep`).
     - `GracefulShutdown`: `shutdown /s /t 30 /c "..."`.
     - `ExecuteCommand`: Runs the user-provided command string via `cmd /C` with `UPS_*` environment variables.
   - Action execution uses a latch (`action_triggered_latch`) so an alert or command is triggered once per threshold crossing and resets automatically when the UPS returns to a healthy state above the threshold.
4. **UPS JSON Payload Compatibility**:
   - `UpsStatus` in `src/ups.rs` must always deserialize the reference payload cleanly and use `#[serde(default)]` for forward compatibility.

---

## Development & Verification Commands

- **Format check**: `cargo fmt --all -- --check`
- **Lint check**: `cargo clippy --all-targets -- -D warnings`
- **Run unit tests**: `cargo test`
- **Run app**: `cargo run`
- **Build release**: `cargo build --release`
