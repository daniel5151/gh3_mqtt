use crate::config::SensorConfig;
use serde_json::json;

pub struct DiscoveryMessage {
    pub topic: String,
    pub payload: String,
}

pub fn generate_ha_discovery_messages(
    discovery_prefix: &str,
    topic_prefix: &str,
    sensors: &SensorConfig,
) -> Vec<DiscoveryMessage> {
    let mut messages = Vec::new();
    let status_topic = format!("{}/status", topic_prefix);
    let song_active_topic = format!("{}/game/song_active", topic_prefix);

    let device = json!({
        "identifiers": ["gh3_pc_player1"],
        "name": "Guitar Hero III",
        "model": "GH3+ PC",
        "manufacturer": "Neversoft / Aspyr"
    });

    // Dual availability for gameplay sensors: game must be running AND song
    // must be playing
    let in_game_availability = json!([
        {
            "topic": status_topic,
            "payload_available": "online",
            "payload_not_available": "offline"
        },
        {
            "topic": song_active_topic,
            "payload_available": "playing",
            "payload_not_available": "idle"
        }
    ]);

    // Game-level availability (available whenever GH3 is running)
    let game_level_availability = json!([
        {
            "topic": status_topic,
            "payload_available": "online",
            "payload_not_available": "offline"
        }
    ]);

    // 1. Star Power Active (Binary Sensor - in-game only)
    let star_power_topic = format!(
        "{}/binary_sensor/{}_star_power/config",
        discovery_prefix, topic_prefix
    );
    if sensors.star_power {
        messages.push(DiscoveryMessage {
            topic: star_power_topic,
            payload: json!({
                "name": "Star Power",
                "unique_id": format!("{}_star_power", topic_prefix),
                "state_topic": format!("{}/player1/star_power/state", topic_prefix),
                "payload_on": "ON",
                "payload_off": "OFF",
                "json_attributes_topic": format!("{}/player1/star_power/attributes", topic_prefix),
                "availability": in_game_availability,
                "availability_mode": "all",
                "icon": "mdi:lightning-bolt",
                "device": &device
            })
            .to_string(),
        });
    } else {
        messages.push(DiscoveryMessage {
            topic: star_power_topic,
            payload: String::new(),
        });
    }

    // 2. Star Power Gauge (Sensor % - in-game only)
    let gauge_topic = format!(
        "{}/sensor/{}_star_power_gauge/config",
        discovery_prefix, topic_prefix
    );
    if sensors.star_power_gauge {
        messages.push(DiscoveryMessage {
            topic: gauge_topic,
            payload: json!({
                "name": "Star Power Gauge",
                "unique_id": format!("{}_star_power_gauge", topic_prefix),
                "state_topic": format!("{}/player1/star_power/gauge", topic_prefix),
                "unit_of_measurement": "%",
                "availability": in_game_availability,
                "availability_mode": "all",
                "icon": "mdi:lightning-bolt-circle",
                "device": &device
            })
            .to_string(),
        });
    } else {
        messages.push(DiscoveryMessage {
            topic: gauge_topic,
            payload: String::new(),
        });
    }

    // 3. Current Song (Sensor: "Title - Artist" - game level)
    let current_song_topic = format!(
        "{}/sensor/{}_current_song/config",
        discovery_prefix, topic_prefix
    );
    if sensors.current_song {
        messages.push(DiscoveryMessage {
            topic: current_song_topic,
            payload: json!({
                "name": "Current Song",
                "unique_id": format!("{}_current_song", topic_prefix),
                "state_topic": format!("{}/game/current_song", topic_prefix),
                "availability": game_level_availability,
                "availability_mode": "all",
                "icon": "mdi:music",
                "device": &device
            })
            .to_string(),
        });
    } else {
        messages.push(DiscoveryMessage {
            topic: current_song_topic,
            payload: String::new(),
        });
    }

    // 4. Difficulty (Sensor: "Easy"/"Medium"/"Hard"/"Expert" - game level)
    let difficulty_topic = format!(
        "{}/sensor/{}_difficulty/config",
        discovery_prefix, topic_prefix
    );
    if sensors.difficulty {
        messages.push(DiscoveryMessage {
            topic: difficulty_topic,
            payload: json!({
                "name": "Difficulty",
                "unique_id": format!("{}_difficulty", topic_prefix),
                "state_topic": format!("{}/game/difficulty", topic_prefix),
                "availability": game_level_availability,
                "availability_mode": "all",
                "icon": "mdi:speedometer",
                "device": &device
            })
            .to_string(),
        });
    } else {
        messages.push(DiscoveryMessage {
            topic: difficulty_topic,
            payload: String::new(),
        });
    }

    // 5. Note Streak (Sensor - in-game only)
    let streak_topic = format!("{}/sensor/{}_streak/config", discovery_prefix, topic_prefix);
    if sensors.streak {
        messages.push(DiscoveryMessage {
            topic: streak_topic,
            payload: json!({
                "name": "Note Streak",
                "unique_id": format!("{}_streak", topic_prefix),
                "state_topic": format!("{}/player1/streak", topic_prefix),
                "availability": in_game_availability,
                "availability_mode": "all",
                "icon": "mdi:music-note",
                "device": &device
            })
            .to_string(),
        });
    } else {
        messages.push(DiscoveryMessage {
            topic: streak_topic,
            payload: String::new(),
        });
    }

    // 6. Score Multiplier (Sensor - in-game only)
    let multiplier_topic = format!(
        "{}/sensor/{}_multiplier/config",
        discovery_prefix, topic_prefix
    );
    if sensors.multiplier {
        messages.push(DiscoveryMessage {
            topic: multiplier_topic,
            payload: json!({
                "name": "Score Multiplier",
                "unique_id": format!("{}_multiplier", topic_prefix),
                "state_topic": format!("{}/player1/multiplier", topic_prefix),
                "unit_of_measurement": "x",
                "availability": in_game_availability,
                "availability_mode": "all",
                "icon": "mdi:close",
                "device": &device
            })
            .to_string(),
        });
    } else {
        messages.push(DiscoveryMessage {
            topic: multiplier_topic,
            payload: String::new(),
        });
    }

    // 7. Total Score (Sensor - in-game only)
    let score_topic = format!("{}/sensor/{}_score/config", discovery_prefix, topic_prefix);
    if sensors.score {
        messages.push(DiscoveryMessage {
            topic: score_topic,
            payload: json!({
                "name": "Score",
                "unique_id": format!("{}_score", topic_prefix),
                "state_topic": format!("{}/player1/score", topic_prefix),
                "availability": in_game_availability,
                "availability_mode": "all",
                "icon": "mdi:counter",
                "device": &device
            })
            .to_string(),
        });
    } else {
        messages.push(DiscoveryMessage {
            topic: score_topic,
            payload: String::new(),
        });
    }

    // 8. Notes Missed (Sensor - in-game only)
    let notes_missed_topic = format!(
        "{}/sensor/{}_notes_missed/config",
        discovery_prefix, topic_prefix
    );
    if sensors.notes_missed {
        messages.push(DiscoveryMessage {
            topic: notes_missed_topic,
            payload: json!({
                "name": "Notes Missed",
                "unique_id": format!("{}_notes_missed", topic_prefix),
                "state_topic": format!("{}/player1/notes_missed", topic_prefix),
                "availability": in_game_availability,
                "availability_mode": "all",
                "icon": "mdi:close-circle-outline",
                "device": &device
            })
            .to_string(),
        });
    } else {
        messages.push(DiscoveryMessage {
            topic: notes_missed_topic,
            payload: String::new(),
        });
    }

    // 9. Notes Hit (Sensor - in-game only)
    let notes_hit_topic = format!(
        "{}/sensor/{}_notes_hit/config",
        discovery_prefix, topic_prefix
    );
    if sensors.notes_hit {
        messages.push(DiscoveryMessage {
            topic: notes_hit_topic,
            payload: json!({
                "name": "Notes Hit",
                "unique_id": format!("{}_notes_hit", topic_prefix),
                "state_topic": format!("{}/player1/notes_hit", topic_prefix),
                "availability": in_game_availability,
                "availability_mode": "all",
                "icon": "mdi:check-circle-outline",
                "device": &device
            })
            .to_string(),
        });
    } else {
        messages.push(DiscoveryMessage {
            topic: notes_hit_topic,
            payload: String::new(),
        });
    }

    // 10. Game Paused (Binary Sensor - in-game only)
    let paused_topic = format!(
        "{}/binary_sensor/{}_paused/config",
        discovery_prefix, topic_prefix
    );
    if sensors.paused {
        messages.push(DiscoveryMessage {
            topic: paused_topic,
            payload: json!({
                "name": "Game Paused",
                "unique_id": format!("{}_paused", topic_prefix),
                "state_topic": format!("{}/game/paused/state", topic_prefix),
                "payload_on": "ON",
                "payload_off": "OFF",
                "availability": in_game_availability,
                "availability_mode": "all",
                "icon": "mdi:pause",
                "device": &device
            })
            .to_string(),
        });
    } else {
        messages.push(DiscoveryMessage {
            topic: paused_topic,
            payload: String::new(),
        });
    }

    messages
}
