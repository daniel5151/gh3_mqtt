use self::schema::RawConfig;
use self::schema::RawSensorConfig;

/// Errors encountered while parsing or validating plugin configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A syntax or schema deserialization error occurred while parsing TOML.
    #[error("TOML syntax or schema error:\n{0}")]
    Parse(#[from] toml::de::Error),
    /// A semantic validation error occurred (e.g. out-of-range value).
    #[error("Validation error: {0}")]
    Validation(String),
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

impl Config {
    /// Constructs a validated `Config` from raw deserialized TOML, applying
    /// inline defaults for optional fields.
    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(content).map_err(ConfigError::Parse)?;

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

        if host.is_empty() {
            return Err(ConfigError::Validation(
                "field 'host' cannot be empty.".to_string(),
            ));
        }
        if port == 0 {
            return Err(ConfigError::Validation(
                "field 'port' must be greater than 0.".to_string(),
            ));
        }
        if topic_prefix.is_empty() {
            return Err(ConfigError::Validation(
                "field 'topic_prefix' cannot be empty.".to_string(),
            ));
        }
        if topic_prefix.starts_with('/') || topic_prefix.ends_with('/') {
            return Err(ConfigError::Validation(
                "field 'topic_prefix' cannot start or end with '/'.".to_string(),
            ));
        }
        if topic_prefix.contains("//") {
            return Err(ConfigError::Validation(
                "field 'topic_prefix' cannot contain consecutive slashes ('//').".to_string(),
            ));
        }
        if topic_prefix.contains('+') || topic_prefix.contains('#') {
            return Err(ConfigError::Validation(
                "field 'topic_prefix' cannot contain MQTT wildcards ('+' or '#').".to_string(),
            ));
        }

        if ha_discovery {
            if ha_discovery_prefix.is_empty() {
                return Err(ConfigError::Validation(
                    "field 'ha_discovery_prefix' cannot be empty when 'ha_discovery' is enabled."
                        .to_string(),
                ));
            }
            if ha_discovery_prefix.starts_with('/') || ha_discovery_prefix.ends_with('/') {
                return Err(ConfigError::Validation(
                    "field 'ha_discovery_prefix' cannot start or end with '/'.".to_string(),
                ));
            }
            if ha_discovery_prefix.contains("//") {
                return Err(ConfigError::Validation(
                    "field 'ha_discovery_prefix' cannot contain consecutive slashes ('//')."
                        .to_string(),
                ));
            }
            if ha_discovery_prefix.contains('+') || ha_discovery_prefix.contains('#') {
                return Err(ConfigError::Validation(
                    "field 'ha_discovery_prefix' cannot contain MQTT wildcards ('+' or '#')."
                        .to_string(),
                ));
            }
        }

        if !(5..=5000).contains(&poll_interval_ms) {
            return Err(ConfigError::Validation(format!(
                "field 'poll_interval_ms' ({}) must be between 5 and 5000 ms.",
                poll_interval_ms
            )));
        }
        if !(0.1..=50.0).contains(&gauge_threshold_pct) {
            return Err(ConfigError::Validation(format!(
                "field 'gauge_threshold_pct' ({}) must be between 0.1% and 50.0%.",
                gauge_threshold_pct
            )));
        }

        let RawSensorConfig {
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
        } = sensors.unwrap_or_default();

        let events = events.unwrap_or(true);

        Ok(Self {
            host,
            port,
            username: username.filter(|u| !u.is_empty()),
            password: password.filter(|p| !p.is_empty()),
            topic_prefix,
            ha_discovery,
            ha_discovery_prefix,
            poll_interval_ms,
            gauge_threshold_pct,
            debug_log,
            hifreq,
            sensors: SensorConfig {
                star_power: star_power.unwrap_or(true),
                current_song: current_song.unwrap_or(true),
                difficulty: difficulty.unwrap_or(true),
                paused: paused.unwrap_or(true),
                events,

                star_power_gauge: star_power_gauge.unwrap_or(hifreq),
                streak: streak.unwrap_or(hifreq),
                multiplier: multiplier.unwrap_or(hifreq),
                score: score.unwrap_or(hifreq),
                notes_hit: notes_hit.unwrap_or(hifreq),
                notes_missed: notes_missed.unwrap_or(hifreq),
                note_missed_event: if !events {
                    false
                } else {
                    note_missed_event.unwrap_or(hifreq)
                },
                periodic_state: periodic_state.unwrap_or(hifreq),
            },
        })
    }
}

/// Raw deserialization schema mapping directly to `gh3_mqtt.toml`.
mod schema {
    use serde::Deserialize;

    /// Raw TOML schema for individual sensor and event toggles.
    #[derive(Debug, Clone, Default, Deserialize)]
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
        pub hifreq: bool,
        pub sensors: Option<RawSensorConfig>,
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
