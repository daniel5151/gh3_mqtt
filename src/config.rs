use std::fs;
use std::path::Path;

use self::schema::RawConfig;
use self::schema::RawSensorConfig;

/// Errors encountered while loading or validating plugin configuration.
#[derive(Debug)]
pub enum ConfigError {
    /// An I/O error occurred while reading the configuration file.
    Io(std::io::Error),
    /// A syntax or schema deserialization error occurred while parsing TOML.
    Parse(toml::de::Error),
    /// A semantic validation error occurred (e.g. out-of-range value).
    Validation(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {}", err),
            Self::Parse(err) => write!(f, "TOML syntax or schema error:\n{}", err),
            Self::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Parse(err) => Some(err),
            Self::Validation(_) => None,
        }
    }
}

/// Sensor and telemetry reporting configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SensorConfig {
    // Low-frequency sensors & events
    /// Star Power active binary sensor (ON/OFF) and attributes.
    pub star_power: bool,
    /// Current song title and artist sensor.
    pub current_song: bool,
    /// Selected game difficulty sensor.
    pub difficulty: bool,
    /// In-game pause binary sensor (ON/OFF).
    pub paused: bool,
    /// Lifecycle events (song_started, song_ended, star_power_activated,
    /// star_power_deactivated).
    pub events: bool,

    // High-frequency sensors & events
    /// Star Power gauge percentage sensor (0.0% - 100.0%).
    pub star_power_gauge: bool,
    /// Note combo streak counter sensor.
    pub streak: bool,
    /// Score multiplier sensor (1x - 8x).
    pub multiplier: bool,
    /// Cumulative song score sensor.
    pub score: bool,
    /// Successfully hit notes counter sensor.
    pub notes_hit: bool,
    /// Missed notes counter sensor.
    pub notes_missed: bool,
    /// Edge-triggered note_missed event on player1/event topic.
    pub note_missed_event: bool,
    /// Periodic full JSON state broadcast on player1/state (1 Hz).
    pub periodic_state: bool,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            star_power: true,
            current_song: true,
            difficulty: true,
            paused: true,
            events: true,

            star_power_gauge: false,
            streak: false,
            multiplier: false,
            score: false,
            notes_hit: false,
            notes_missed: false,
            note_missed_event: false,
            periodic_state: false,
        }
    }
}

/// Global plugin configuration parsed from `gh3_mqtt.toml`.
#[derive(Debug, Clone, PartialEq)]
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
    /// Master toggle for high-frequency telemetry and events.
    pub hifreq: bool,
    pub sensors: SensorConfig,
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
            hifreq: false,
            sensors: SensorConfig::default(),
        }
    }
}

impl Config {
    /// Applies parsed TOML configuration values on top of this instance.
    pub fn apply_toml(&mut self, raw: RawConfig) {
        let RawConfig {
            host,
            port,
            username,
            password,
            topic_prefix,
            ha_discovery,
            ha_discovery_prefix,
            poll_interval_ms,
            gauge_threshold_pct,
            debug_log,
            hifreq,
            sensors,
        } = raw;

        if let Some(host) = host {
            self.host = host;
        }
        if let Some(port) = port {
            self.port = port;
        }
        if let Some(username) = username {
            self.username = if username.is_empty() {
                None
            } else {
                Some(username)
            };
        }
        if let Some(password) = password {
            self.password = if password.is_empty() {
                None
            } else {
                Some(password)
            };
        }
        if let Some(topic_prefix) = topic_prefix {
            self.topic_prefix = topic_prefix;
        }
        if let Some(ha_discovery) = ha_discovery {
            self.ha_discovery = ha_discovery;
        }
        if let Some(ha_discovery_prefix) = ha_discovery_prefix {
            self.ha_discovery_prefix = ha_discovery_prefix;
        }
        if let Some(poll_interval_ms) = poll_interval_ms {
            self.poll_interval_ms = poll_interval_ms;
        }
        if let Some(gauge_threshold_pct) = gauge_threshold_pct {
            self.gauge_threshold_pct = gauge_threshold_pct;
        }
        if let Some(debug_log) = debug_log {
            self.debug_log = debug_log;
        }

        if let Some(hifreq) = hifreq {
            self.hifreq = hifreq;
            self.sensors.star_power_gauge = hifreq;
            self.sensors.streak = hifreq;
            self.sensors.multiplier = hifreq;
            self.sensors.score = hifreq;
            self.sensors.notes_hit = hifreq;
            self.sensors.notes_missed = hifreq;
            self.sensors.note_missed_event = hifreq;
            self.sensors.periodic_state = hifreq;
        }

        if let Some(RawSensorConfig {
            star_power,
            current_song,
            difficulty,
            paused,
            events,
            star_power_gauge,
            streak,
            multiplier,
            score,
            notes_hit,
            notes_missed,
            note_missed_event,
            periodic_state,
        }) = sensors
        {
            if let Some(v) = star_power {
                self.sensors.star_power = v;
            }
            if let Some(v) = current_song {
                self.sensors.current_song = v;
            }
            if let Some(v) = difficulty {
                self.sensors.difficulty = v;
            }
            if let Some(v) = paused {
                self.sensors.paused = v;
            }
            if let Some(v) = events {
                self.sensors.events = v;
            }

            if let Some(v) = star_power_gauge {
                self.sensors.star_power_gauge = v;
            }
            if let Some(v) = streak {
                self.sensors.streak = v;
            }
            if let Some(v) = multiplier {
                self.sensors.multiplier = v;
            }
            if let Some(v) = score {
                self.sensors.score = v;
            }
            if let Some(v) = notes_hit {
                self.sensors.notes_hit = v;
            }
            if let Some(v) = notes_missed {
                self.sensors.notes_missed = v;
            }
            if let Some(v) = note_missed_event {
                self.sensors.note_missed_event = v;
            }
            if let Some(v) = periodic_state {
                self.sensors.periodic_state = v;
            }
        }

        if !self.sensors.events {
            self.sensors.note_missed_event = false;
        }
    }

    /// Validates configuration values and normalizes topic prefixes.
    pub fn validate(&mut self) -> Result<(), ConfigError> {
        if self.host.trim().is_empty() {
            return Err(ConfigError::Validation(
                "field 'host' cannot be empty.".to_string(),
            ));
        }
        if self.port == 0 {
            return Err(ConfigError::Validation(
                "field 'port' must be greater than 0.".to_string(),
            ));
        }
        self.topic_prefix = self.topic_prefix.trim_matches('/').to_string();
        if self.topic_prefix.is_empty() {
            return Err(ConfigError::Validation(
                "field 'topic_prefix' cannot be empty.".to_string(),
            ));
        }
        self.ha_discovery_prefix = self.ha_discovery_prefix.trim_matches('/').to_string();
        if self.ha_discovery && self.ha_discovery_prefix.is_empty() {
            return Err(ConfigError::Validation(
                "field 'ha_discovery_prefix' cannot be empty when 'ha_discovery' is enabled."
                    .to_string(),
            ));
        }
        if !(5..=5000).contains(&self.poll_interval_ms) {
            return Err(ConfigError::Validation(format!(
                "field 'poll_interval_ms' ({}) must be between 5 and 5000 ms.",
                self.poll_interval_ms
            )));
        }
        if !(0.1..=50.0).contains(&self.gauge_threshold_pct) {
            return Err(ConfigError::Validation(format!(
                "field 'gauge_threshold_pct' ({}) must be between 0.1% and 50.0%.",
                self.gauge_threshold_pct
            )));
        }
        Ok(())
    }

    /// Parse and validate configuration strictly from a TOML string.
    pub fn parse_toml_str(content: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(content).map_err(ConfigError::Parse)?;
        let mut config = Self::default();
        config.apply_toml(raw);
        config.validate()?;
        Ok(config)
    }

    /// Load configuration strictly from the specified TOML file.
    ///
    /// If the file does not exist, writes the default TOML template and returns
    /// default config. If the file exists but contains malformed TOML or
    /// invalid values, returns `Err`.
    pub fn load_strict<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            let _ = fs::write(path, DEFAULT_TOML);
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(ConfigError::Io)?;
        Self::parse_toml_str(&content)
    }
}

pub const DEFAULT_TOML: &str = r#"# ==============================================================================
# Guitar Hero III MQTT Integration Plugin Configuration
# ==============================================================================

# MQTT broker address and port
host = "127.0.0.1"
port = 1883

# Optional MQTT credentials (leave unset if your broker does not require authentication)
# username = "my_user"
# password = "my_password"

# Base topic prefix (e.g. gh3 -> gh3/player1/star_power, etc.)
topic_prefix = "gh3"

# Home Assistant MQTT Auto-Discovery (creates devices & entities in HA automatically)
ha_discovery = true
ha_discovery_prefix = "homeassistant"

# How often to check game memory (in milliseconds). 16ms is ~60 FPS.
poll_interval_ms = 16

# Minimum change in Star Power gauge % before sending an MQTT update (reduces network spam)
gauge_threshold_pct = 1.0

# Write diagnostic logs to PLUGINS\gh3_mqtt.log
debug_log = true

# Master toggle for high-frequency telemetry and events (score, streak,
# multiplier, notes_hit, notes_missed, star_power_gauge, periodic state, and note_missed event).
# Off (false) by default to prevent flooding MQTT brokers and Home Assistant databases.
hifreq = false

[sensors]
# Individual sensor and event toggles (uncomment to override the default)
# Low-frequency sensors & events:
# star_power = true
# current_song = true
# difficulty = true
# paused = true
# events = true

# High-frequency sensors & events:
# star_power_gauge = true
# streak = true
# multiplier = true
# score = true
# notes_hit = true
# notes_missed = true
# note_missed_event = true
# periodic_state = true
"#;

/// Raw deserialization schema mapping directly to `gh3_mqtt.toml`.
pub mod schema {
    use serde::Deserialize;

    /// Raw TOML schema for individual sensor and event toggles.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct RawSensorConfig {
        pub star_power: Option<bool>,
        pub current_song: Option<bool>,
        pub difficulty: Option<bool>,
        pub paused: Option<bool>,
        pub events: Option<bool>,
        pub star_power_gauge: Option<bool>,
        pub streak: Option<bool>,
        pub multiplier: Option<bool>,
        pub score: Option<bool>,
        pub notes_hit: Option<bool>,
        pub notes_missed: Option<bool>,
        pub note_missed_event: Option<bool>,
        pub periodic_state: Option<bool>,
    }

    /// Raw TOML schema for the configuration file.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct RawConfig {
        pub host: Option<String>,
        pub port: Option<u16>,
        pub username: Option<String>,
        pub password: Option<String>,
        pub topic_prefix: Option<String>,
        pub ha_discovery: Option<bool>,
        pub ha_discovery_prefix: Option<String>,
        pub poll_interval_ms: Option<u64>,
        pub gauge_threshold_pct: Option<f32>,
        pub debug_log: Option<bool>,
        pub hifreq: Option<bool>,
        pub sensors: Option<RawSensorConfig>,
    }
}
