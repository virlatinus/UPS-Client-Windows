use std::env;

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APP_REGISTRY_NAME: &str = "UPS-Client-Windows";

/// Checks whether the application is currently registered in Windows startup (`HKCU\...\Run`).
#[cfg(windows)]
pub fn is_startup_enabled() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(run_key) = hkcu.open_subkey_with_flags(RUN_KEY_PATH, KEY_READ) {
        let val: Result<String, _> = run_key.get_value(APP_REGISTRY_NAME);
        return val.is_ok();
    }
    false
}

#[cfg(not(windows))]
pub fn is_startup_enabled() -> bool {
    false
}

/// Enables or disables starting the application automatically on Windows login.
/// When launched at startup, `--minimized` is passed so the app starts directly in the System Tray.
#[cfg(windows)]
pub fn set_startup_enabled(enable: bool) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run_key, _) = hkcu
        .create_subkey(RUN_KEY_PATH)
        .map_err(|e| format!("Failed to open HKCU Run registry key: {}", e))?;

    if enable {
        let exe_path = env::current_exe()
            .map_err(|e| format!("Failed to determine current executable path: {}", e))?;
        let command = format!("\"{}\" --minimized", exe_path.display());
        run_key
            .set_value(APP_REGISTRY_NAME, &command)
            .map_err(|e| format!("Failed to write startup registry value: {}", e))?;
    } else {
        let _ = run_key.delete_value(APP_REGISTRY_NAME);
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn set_startup_enabled(_enable: bool) -> Result<(), String> {
    Ok(())
}
