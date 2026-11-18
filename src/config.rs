use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Valid 10% increment thresholds for triggering UPS actions.
#[allow(dead_code)]
pub const THRESHOLD_STEPS: [u8; 10] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

/// Action to perform when the UPS threshold is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerAction {
    /// a. Alert with native Windows notification
    NativeNotification,
    /// b. Play a sound
    PlaySound,
    /// c. Graceful shutdown
    GracefulShutdown,
    /// d. Execute command (uses `AppConfig::custom_command`)
    ExecuteCommand,
}

#[allow(dead_code)]
impl TriggerAction {
    pub fn label(&self) -> &'static str {
        match self {
            TriggerAction::NativeNotification => "Alert with native notification",
            TriggerAction::PlaySound => "Play a sound",
            TriggerAction::GracefulShutdown => "Graceful shutdown",
            TriggerAction::ExecuteCommand => "Execute command",
        }
    }

    pub const ALL: [TriggerAction; 4] = [
        TriggerAction::NativeNotification,
        TriggerAction::PlaySound,
        TriggerAction::GracefulShutdown,
        TriggerAction::ExecuteCommand,
    ];
}

/// Persistent user configuration for UPS Client Windows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    /// Whether to run on Windows startup.
    pub run_on_startup: bool,
    /// UPS HTTP status URL to monitor.
    pub ups_status_url: String,
    /// Battery charge threshold to trigger actions (10% increments: 10..=100).
    pub threshold_percent: u8,
    /// Action to perform when the threshold is reached.
    pub action: TriggerAction,
    /// Command line to execute when `action == TriggerAction::ExecuteCommand`.
    pub custom_command: String,
    /// Polling interval in seconds.
    pub poll_interval_secs: u64,
    /// UI scaling factor from 1.0x to 2.0x.
    pub ui_scale: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            run_on_startup: false,
            ups_status_url: String::from("http://192.168.2.161:8000/api/ups"),
            threshold_percent: 30,
            action: TriggerAction::NativeNotification,
            custom_command: String::new(),
            poll_interval_secs: 10,
            ui_scale: 1.0,
        }
    }
}

impl AppConfig {
    /// Snaps any percentage value to the nearest valid 10% increment in `10..=100`.
    pub fn snap_threshold(value: u8) -> u8 {
        let clamped = value.clamp(10, 100);
        let rounded = ((clamped as u16 + 5) / 10) * 10;
        (rounded as u8).clamp(10, 100)
    }

    /// Ensures configuration invariants (10% threshold step, reasonable poll interval, 1.0..=2.0 scale).
    pub fn normalize(&mut self) {
        self.threshold_percent = Self::snap_threshold(self.threshold_percent);
        self.poll_interval_secs = self.poll_interval_secs.clamp(2, 300);
        self.ui_scale = (self.ui_scale * 20.0).round() / 20.0;
        self.ui_scale = self.ui_scale.clamp(1.0, 2.0);
    }

    /// Returns the path to `config.json` inside the user's roaming/local AppData directory.
    pub fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "virlatinus", "UPS-Client-Windows")
            .map(|dirs| dirs.config_dir().join("config.json"))
    }

    /// Loads configuration from disk, falling back to `AppConfig::default()` if not found.
    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&contents) {
                    cfg.normalize();
                    return cfg;
                }
            }
        }
        Self::default()
    }

    /// Saves configuration to disk.
    pub fn save(&mut self) -> Result<(), String> {
        self.normalize();
        let path = Self::config_path()
            .ok_or_else(|| "Unable to resolve Windows AppData directory".to_string())?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let serialized = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&path, serialized).map_err(|e| format!("Failed to write config file: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_snaps_to_ten_percent_increments() {
        assert_eq!(AppConfig::snap_threshold(0), 10);
        assert_eq!(AppConfig::snap_threshold(14), 10);
        assert_eq!(AppConfig::snap_threshold(15), 20);
        assert_eq!(AppConfig::snap_threshold(50), 50);
        assert_eq!(AppConfig::snap_threshold(98), 100);
    }
}
