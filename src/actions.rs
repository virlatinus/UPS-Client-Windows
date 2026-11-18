use crate::config::{AppConfig, TriggerAction};
use crate::ups::UpsStatus;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Executes the configured action in response to a UPS threshold event (or manual test).
pub fn execute_trigger_action(config: &AppConfig, status: &UpsStatus) -> Result<String, String> {
    match config.action {
        TriggerAction::NativeNotification => send_native_notification(config, status),
        TriggerAction::PlaySound => play_alert_sound(status),
        TriggerAction::GracefulShutdown => initiate_graceful_shutdown(status),
        TriggerAction::ExecuteCommand => execute_custom_command(&config.custom_command, status),
    }
}

/// a. Alert with native Windows notification.
pub fn send_native_notification(config: &AppConfig, status: &UpsStatus) -> Result<String, String> {
    let title = format!("UPS Alert: {} ({}%)", status.model, status.battery_charge);
    let body = format!(
        "Status: {} | Battery: {}% (Threshold: {}%) | Runtime: {} | Load: {}% ({}W)",
        status.status_label,
        status.battery_charge,
        config.threshold_percent,
        status.formatted_runtime(),
        status.load,
        status.output_watts
    );

    #[cfg(windows)]
    {
        use winrt_notification::{Duration, Sound, Toast};

        Toast::new(Toast::POWERSHELL_APP_ID)
            .title(&title)
            .text1(&body)
            .sound(Some(Sound::Reminder))
            .duration(Duration::Short)
            .show()
            .map_err(|e| format!("Failed to display Windows Toast notification: {}", e))?;
    }

    Ok(format!("Displayed Windows notification: {}", title))
}

/// b. Play a sound using native Windows audio APIs.
pub fn play_alert_sound(status: &UpsStatus) -> Result<String, String> {
    #[cfg(windows)]
    {
        std::thread::spawn(|| unsafe {
            use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC};
            use windows_sys::Win32::System::Diagnostics::Debug::{Beep, MessageBeep};
            use windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONWARNING;

            // Trigger Windows system exclamation/warning sound alias asynchronously
            let alias: Vec<u16> = "SystemExclamation\0".encode_utf16().collect();
            PlaySoundW(alias.as_ptr(), std::ptr::null_mut(), SND_ALIAS | SND_ASYNC);
            MessageBeep(MB_ICONWARNING);

            // Follow with a short distinct UPS audible alert tone sequence
            let _ = Beep(880, 220);
            let _ = Beep(988, 220);
            let _ = Beep(1175, 320);
        });
    }

    Ok(format!(
        "Played alert sound (UPS {} at {}%)",
        status.model, status.battery_charge
    ))
}

/// c. Graceful Windows shutdown.
pub fn initiate_graceful_shutdown(status: &UpsStatus) -> Result<String, String> {
    let comment = format!(
        "OpenUPS Client for Windows: {} battery reached {}% (status: {}). Initiating graceful shutdown.",
        status.model, status.battery_charge, status.status_label
    );

    let mut cmd = Command::new("shutdown");
    cmd.args(["/s", "/t", "30", "/c", &comment]);

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.spawn()
        .map_err(|e| format!("Failed to initiate graceful shutdown: {}", e))?;

    Ok("Initiated graceful Windows shutdown (30s countdown)".to_string())
}

/// d. Execute user-specified command.
pub fn execute_custom_command(command_line: &str, status: &UpsStatus) -> Result<String, String> {
    let trimmed = command_line.trim();
    if trimmed.is_empty() {
        return Err("No custom command configured to execute.".to_string());
    }

    let mut cmd = Command::new("cmd");
    cmd.args(["/C", trimmed])
        .env("UPS_MODEL", &status.model)
        .env("UPS_STATUS", &status.status)
        .env("UPS_STATUS_LABEL", &status.status_label)
        .env("UPS_BATTERY_CHARGE", status.battery_charge.to_string())
        .env("UPS_BATTERY_RUNTIME", status.battery_runtime.to_string())
        .env("UPS_ON_BATTERY", status.on_battery.to_string())
        .env("UPS_LOAD", status.load.to_string())
        .env("UPS_OUTPUT_WATTS", status.output_watts.to_string());

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.spawn()
        .map_err(|e| format!("Failed to execute command '{}': {}", trimmed, e))?;

    Ok(format!("Executed command: {}", trimmed))
}
