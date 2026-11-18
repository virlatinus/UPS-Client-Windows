use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Represents the telemetry payload returned by the UPS HTTP status endpoint.
///
/// Example JSON payload:
/// ```json
/// {
///   "online": true,
///   "model": "CP1500PFCLCDa",
///   "status": "OL",
///   "status_label": "ONLINE",
///   "on_battery": false,
///   "low_battery": false,
///   "charging": false,
///   "discharging": false,
///   "should_shutdown": false,
///   "battery_charge": 100,
///   "battery_charge_low": 10,
///   "battery_charge_warning": 20,
///   "battery_runtime": 3350,
///   "battery_runtime_min": 56,
///   "battery_runtime_low": 300,
///   "battery_voltage": 27.1,
///   "battery_voltage_nominal": 24.0,
///   "input_voltage": 113.0,
///   "output_voltage": 113.0,
///   "load": 15,
///   "realpower_nominal": 1000,
///   "output_watts": 150,
///   "beeper": "enabled"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UpsStatus {
    pub online: bool,
    pub model: String,
    pub status: String,
    pub status_label: String,
    pub on_battery: bool,
    pub low_battery: bool,
    pub charging: bool,
    pub discharging: bool,
    pub should_shutdown: bool,
    pub battery_charge: u8,
    pub battery_charge_low: u8,
    pub battery_charge_warning: u8,
    pub battery_runtime: u32,
    pub battery_runtime_min: u32,
    pub battery_runtime_low: u32,
    pub battery_voltage: f64,
    pub battery_voltage_nominal: f64,
    pub input_voltage: f64,
    pub output_voltage: f64,
    pub load: u8,
    pub realpower_nominal: u32,
    pub output_watts: u32,
    pub beeper: String,
}

impl Default for UpsStatus {
    fn default() -> Self {
        Self {
            online: true,
            model: String::from("Unknown"),
            status: String::from("UNKNOWN"),
            status_label: String::from("UNKNOWN"),
            on_battery: false,
            low_battery: false,
            charging: false,
            discharging: false,
            should_shutdown: false,
            battery_charge: 100,
            battery_charge_low: 10,
            battery_charge_warning: 20,
            battery_runtime: 0,
            battery_runtime_min: 0,
            battery_runtime_low: 0,
            battery_voltage: 0.0,
            battery_voltage_nominal: 24.0,
            input_voltage: 0.0,
            output_voltage: 0.0,
            load: 0,
            realpower_nominal: 0,
            output_watts: 0,
            beeper: String::from("unknown"),
        }
    }
}

impl UpsStatus {
    /// Returns true if the UPS battery charge is at or below the configured threshold
    /// (in 10% increments) while on battery, in low_battery state, or when `should_shutdown`
    /// is signaled by the endpoint, or if `battery_charge <= threshold_percent`.
    pub fn should_trigger_action(&self, threshold_percent: u8) -> bool {
        self.should_shutdown || self.low_battery || self.battery_charge <= threshold_percent
    }

    /// Formats runtime seconds into a human-friendly string (e.g., "55m 50s").
    pub fn formatted_runtime(&self) -> String {
        let mins = self.battery_runtime / 60;
        let secs = self.battery_runtime % 60;
        if mins >= 60 {
            let hours = mins / 60;
            let rem_mins = mins % 60;
            format!("{}h {}m", hours, rem_mins)
        } else {
            format!("{}m {}s", mins, secs)
        }
    }
}

/// Fetches the current `UpsStatus` from the configured HTTP/HTTPS endpoint.
pub fn fetch_ups_status(url: &str) -> Result<UpsStatus, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("UPS status URL is empty".to_string());
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(4))
        .timeout_read(Duration::from_secs(5))
        .build();

    let response = agent
        .get(trimmed)
        .set("Accept", "application/json")
        .set("User-Agent", "UPS-Client-Windows/0.1.0")
        .call()
        .map_err(|e| format!("Request error: {}", e))?;

    let status: UpsStatus = response
        .into_json()
        .map_err(|e| format!("Invalid JSON payload: {}", e))?;

    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reference_json_payload() {
        let raw = r#"{"online": true, "model": "CP1500PFCLCDa", "status": "OL", "status_label": "ONLINE", "on_battery": false, "low_battery": false, "charging": false, "discharging": false, "should_shutdown": false, "battery_charge": 100, "battery_charge_low": 10, "battery_charge_warning": 20, "battery_runtime": 3350, "battery_runtime_min": 56, "battery_runtime_low": 300, "battery_voltage": 27.1, "battery_voltage_nominal": 24.0, "input_voltage": 113.0, "output_voltage": 113.0, "load": 15, "realpower_nominal": 1000, "output_watts": 150, "beeper": "enabled"}"#;

        let parsed: UpsStatus = serde_json::from_str(raw).expect("Should parse reference UPS JSON");
        assert!(parsed.online);
        assert_eq!(parsed.model, "CP1500PFCLCDa");
        assert_eq!(parsed.status, "OL");
        assert_eq!(parsed.status_label, "ONLINE");
        assert!(!parsed.on_battery);
        assert!(!parsed.low_battery);
        assert_eq!(parsed.battery_charge, 100);
        assert_eq!(parsed.battery_runtime, 3350);
        assert_eq!(parsed.battery_runtime_min, 56);
        assert!((parsed.battery_voltage - 27.1).abs() < f64::EPSILON);
        assert_eq!(parsed.load, 15);
        assert_eq!(parsed.realpower_nominal, 1000);
        assert_eq!(parsed.output_watts, 150);
        assert_eq!(parsed.beeper, "enabled");
        assert!(!parsed.should_trigger_action(50));
        assert!(parsed.should_trigger_action(100));
    }
}
