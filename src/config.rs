use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub topic_prefix: String,
    pub ha_discovery: bool,
    pub ha_discovery_prefix: String,
    pub poll_interval_ms: u64,
    pub gauge_threshold_pct: f32,
    pub debug_log: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 1883,
            username: None,
            password: None,
            topic_prefix: "gh3".to_string(),
            ha_discovery: true,
            ha_discovery_prefix: "homeassistant".to_string(),
            poll_interval_ms: 16,
            gauge_threshold_pct: 1.0,
            debug_log: true,
        }
    }
}

impl Config {
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        let mut config = Self::default();
        let content = match fs::read_to_string(path.as_ref()) {
            Ok(c) => c,
            Err(_) => {
                // If not found, create a template config file
                let _ = fs::write(path.as_ref(), DEFAULT_INI);
                return config;
            }
        };

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with('#')
                || line.starts_with(';')
                || line.starts_with('[')
            {
                continue;
            }

            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim().to_lowercase();
                let val = val.trim();
                match key.as_str() {
                    "host" | "broker_host" => config.host = val.to_string(),
                    "port" | "broker_port" => {
                        if let Ok(p) = val.parse::<u16>() {
                            config.port = p;
                        }
                    }
                    "username" => {
                        config.username = if val.is_empty() {
                            None
                        } else {
                            Some(val.to_string())
                        };
                    }
                    "password" => {
                        config.password = if val.is_empty() {
                            None
                        } else {
                            Some(val.to_string())
                        };
                    }
                    "topic_prefix" => {
                        config.topic_prefix = val.trim_matches('/').to_string();
                    }
                    "ha_discovery" => {
                        config.ha_discovery = val.eq_ignore_ascii_case("true") || val == "1";
                    }
                    "ha_discovery_prefix" => {
                        config.ha_discovery_prefix = val.trim_matches('/').to_string();
                    }
                    "poll_interval_ms" => {
                        if let Ok(ms) = val.parse::<u64>() {
                            config.poll_interval_ms = ms.clamp(5, 5000);
                        }
                    }
                    "gauge_threshold_pct" => {
                        if let Ok(t) = val.parse::<f32>() {
                            config.gauge_threshold_pct = t.clamp(0.1, 50.0);
                        }
                    }
                    "debug_log" => {
                        config.debug_log = val.eq_ignore_ascii_case("true") || val == "1";
                    }
                    _ => {}
                }
            }
        }

        config
    }
}

pub const DEFAULT_INI: &str = r#"[mqtt]
# MQTT broker address and port
host = 127.0.0.1
port = 1883

# Optional MQTT credentials (leave blank if your broker does not require authentication)
username =
password =

# Base topic prefix (e.g. gh3 -> gh3/player1/star_power, etc.)
topic_prefix = gh3

# Home Assistant MQTT Auto-Discovery (creates devices & entities in HA automatically)
ha_discovery = true
ha_discovery_prefix = homeassistant

# How often to check game memory (in milliseconds). 16ms is ~60 FPS.
poll_interval_ms = 16

# Minimum change in Star Power gauge % before sending an MQTT update (reduces network spam)
gauge_threshold_pct = 1.0

# Write debug logs to PLUGINS\gh3_mqtt.log
debug_log = true
"#;
