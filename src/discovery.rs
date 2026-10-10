use serde_json::json;

pub struct DiscoveryMessage {
    pub topic: String,
    pub payload: String,
}

pub fn generate_ha_discovery_messages(
    discovery_prefix: &str,
    topic_prefix: &str,
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
    messages.push(DiscoveryMessage {
        topic: format!(
            "{}/binary_sensor/{}_star_power/config",
            discovery_prefix, topic_prefix
        ),
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
            "device": device
        })
        .to_string(),
    });

    // 2. Star Power Gauge (Sensor % - in-game only)
    messages.push(DiscoveryMessage {
        topic: format!(
            "{}/sensor/{}_star_power_gauge/config",
            discovery_prefix, topic_prefix
        ),
        payload: json!({
            "name": "Star Power Gauge",
            "unique_id": format!("{}_star_power_gauge", topic_prefix),
            "state_topic": format!("{}/player1/star_power/gauge", topic_prefix),
            "unit_of_measurement": "%",
            "availability": in_game_availability,
            "availability_mode": "all",
            "icon": "mdi:lightning-bolt-circle",
            "device": device
        })
        .to_string(),
    });

    // 3. Current Song (Sensor: "Title - Artist" - game level)
    messages.push(DiscoveryMessage {
        topic: format!(
            "{}/sensor/{}_current_song/config",
            discovery_prefix, topic_prefix
        ),
        payload: json!({
            "name": "Current Song",
            "unique_id": format!("{}_current_song", topic_prefix),
            "state_topic": format!("{}/game/current_song", topic_prefix),
            "availability": game_level_availability,
            "availability_mode": "all",
            "icon": "mdi:music",
            "device": device
        })
        .to_string(),
    });

    // 4. Difficulty (Sensor: "Easy"/"Medium"/"Hard"/"Expert" - game level)
    messages.push(DiscoveryMessage {
        topic: format!(
            "{}/sensor/{}_difficulty/config",
            discovery_prefix, topic_prefix
        ),
        payload: json!({
            "name": "Difficulty",
            "unique_id": format!("{}_difficulty", topic_prefix),
            "state_topic": format!("{}/game/difficulty", topic_prefix),
            "availability": game_level_availability,
            "availability_mode": "all",
            "icon": "mdi:speedometer",
            "device": device
        })
        .to_string(),
    });

    // 5. Note Streak (Sensor - in-game only)
    messages.push(DiscoveryMessage {
        topic: format!("{}/sensor/{}_streak/config", discovery_prefix, topic_prefix),
        payload: json!({
            "name": "Note Streak",
            "unique_id": format!("{}_streak", topic_prefix),
            "state_topic": format!("{}/player1/streak", topic_prefix),
            "availability": in_game_availability,
            "availability_mode": "all",
            "icon": "mdi:music-note",
            "device": device
        })
        .to_string(),
    });

    // 6. Score Multiplier (Sensor - in-game only)
    messages.push(DiscoveryMessage {
        topic: format!(
            "{}/sensor/{}_multiplier/config",
            discovery_prefix, topic_prefix
        ),
        payload: json!({
            "name": "Score Multiplier",
            "unique_id": format!("{}_multiplier", topic_prefix),
            "state_topic": format!("{}/player1/multiplier", topic_prefix),
            "unit_of_measurement": "x",
            "availability": in_game_availability,
            "availability_mode": "all",
            "icon": "mdi:close",
            "device": device
        })
        .to_string(),
    });

    // 7. Total Score (Sensor - in-game only)
    messages.push(DiscoveryMessage {
        topic: format!("{}/sensor/{}_score/config", discovery_prefix, topic_prefix),
        payload: json!({
            "name": "Score",
            "unique_id": format!("{}_score", topic_prefix),
            "state_topic": format!("{}/player1/score", topic_prefix),
            "availability": in_game_availability,
            "availability_mode": "all",
            "icon": "mdi:counter",
            "device": device
        })
        .to_string(),
    });

    // 8. Notes Missed (Sensor - in-game only)
    messages.push(DiscoveryMessage {
        topic: format!(
            "{}/sensor/{}_notes_missed/config",
            discovery_prefix, topic_prefix
        ),
        payload: json!({
            "name": "Notes Missed",
            "unique_id": format!("{}_notes_missed", topic_prefix),
            "state_topic": format!("{}/player1/notes_missed", topic_prefix),
            "availability": in_game_availability,
            "availability_mode": "all",
            "icon": "mdi:close-circle-outline",
            "device": device
        })
        .to_string(),
    });

    // 9. Notes Hit (Sensor - in-game only)
    messages.push(DiscoveryMessage {
        topic: format!(
            "{}/sensor/{}_notes_hit/config",
            discovery_prefix, topic_prefix
        ),
        payload: json!({
            "name": "Notes Hit",
            "unique_id": format!("{}_notes_hit", topic_prefix),
            "state_topic": format!("{}/player1/notes_hit", topic_prefix),
            "availability": in_game_availability,
            "availability_mode": "all",
            "icon": "mdi:check-circle-outline",
            "device": device
        })
        .to_string(),
    });

    // 10. Game Paused (Binary Sensor - in-game only)
    messages.push(DiscoveryMessage {
        topic: format!(
            "{}/binary_sensor/{}_paused/config",
            discovery_prefix, topic_prefix
        ),
        payload: json!({
            "name": "Game Paused",
            "unique_id": format!("{}_paused", topic_prefix),
            "state_topic": format!("{}/game/paused/state", topic_prefix),
            "payload_on": "ON",
            "payload_off": "OFF",
            "availability": in_game_availability,
            "availability_mode": "all",
            "icon": "mdi:pause",
            "device": device
        })
        .to_string(),
    });

    messages
}
