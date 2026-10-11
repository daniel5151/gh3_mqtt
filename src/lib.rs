//! Guitar Hero III MQTT integration plugin.

mod config;
mod discovery;
mod gh3;
mod mqtt;

use anyhow::Context;
use config::Config;
use discovery::generate_ha_discovery_messages;
use gh3::GameState;
use mqtt::MqttClient;
use simplelog::ConfigBuilder;
use simplelog::LevelFilter;
use simplelog::WriteLogger;

use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use windows::Win32::Foundation::HMODULE;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::System::SystemServices::DLL_PROCESS_DETACH;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use windows::Win32::UI::WindowsAndMessaging::MB_OK;
use windows::Win32::UI::WindowsAndMessaging::MessageBoxW;
use windows::core::PCWSTR;

static RUNNING: AtomicBool = AtomicBool::new(false);

fn show_error_box(title: &str, message: &str) {
    let wide_title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let wide_message: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        // SAFETY: Calling standard Win32 MessageBoxW with null HWND and
        // null-terminated UTF-16 strings.
        MessageBoxW(
            HWND::default(),
            PCWSTR(wide_message.as_ptr()),
            PCWSTR(wide_title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn get_plugin_dir() -> PathBuf {
    if Path::new("PLUGINS").is_dir() {
        PathBuf::from("PLUGINS")
    } else {
        PathBuf::from(".")
    }
}

fn publish_all_entities(client: &mut MqttClient, config: &Config, state: &GameState) {
    let pfx = &config.topic_prefix;
    let s = &config.sensors;

    // 1. Song Active Availability (playing / idle)
    let song_active_payload: &[u8] = if state.is_playing {
        b"playing"
    } else {
        b"idle"
    };
    if let Err(e) = client.publish(
        &format!("{}/game/song_active", pfx),
        song_active_payload,
        true,
    ) {
        log::debug!("Failed to publish game/song_active: {e:#}");
    }

    // 2. Game-level sensors (Song Title - Artist, Difficulty, Paused)
    if s.current_song {
        let song_text = if state.is_playing {
            &state.current_song
        } else {
            "Idle"
        };
        if let Err(e) = client.publish(
            &format!("{}/game/current_song", pfx),
            song_text.as_bytes(),
            false,
        ) {
            log::debug!("Failed to publish game/current_song: {e:#}");
        }
    }
    if s.difficulty {
        if let Err(e) = client.publish(
            &format!("{}/game/difficulty", pfx),
            state.difficulty.as_bytes(),
            false,
        ) {
            log::debug!("Failed to publish game/difficulty: {e:#}");
        }
    }
    if s.paused {
        let payload: &[u8] = if state.is_paused { b"ON" } else { b"OFF" };
        if let Err(e) = client.publish(&format!("{}/game/paused/state", pfx), payload, false) {
            log::debug!("Failed to publish game/paused/state: {e:#}");
        }
    }

    // 3. Gameplay sensors
    if s.star_power {
        let payload: &[u8] = if state.star_power_active {
            b"ON"
        } else {
            b"OFF"
        };
        if let Err(e) = client.publish(&format!("{}/player1/star_power/state", pfx), payload, false)
        {
            log::debug!("Failed to publish player1/star_power/state: {e:#}");
        }
        let attr_json = serde_json::json!({
            "song": state.current_song,
            "difficulty": state.difficulty,
            "gauge": state.star_power_gauge,
            "streak": state.streak,
            "multiplier": state.effective_multiplier,
            "score": state.score,
            "notes_missed": state.notes_missed,
            "notes_hit": state.notes_hit,
            "is_paused": state.is_paused,
        });
        if let Err(e) = client.publish(
            &format!("{}/player1/star_power/attributes", pfx),
            attr_json.to_string().as_bytes(),
            false,
        ) {
            log::debug!("Failed to publish player1/star_power/attributes: {e:#}");
        }
    }
    if s.star_power_gauge {
        let payload = format!("{:.1}", state.star_power_gauge);
        if let Err(e) = client.publish(
            &format!("{}/player1/star_power/gauge", pfx),
            payload.as_bytes(),
            false,
        ) {
            log::debug!("Failed to publish player1/star_power/gauge: {e:#}");
        }
    }
    if s.streak {
        let payload = state.streak.to_string();
        if let Err(e) = client.publish(
            &format!("{}/player1/streak", pfx),
            payload.as_bytes(),
            false,
        ) {
            log::debug!("Failed to publish player1/streak: {e:#}");
        }
    }
    if s.multiplier {
        let payload = state.effective_multiplier.to_string();
        if let Err(e) = client.publish(
            &format!("{}/player1/multiplier", pfx),
            payload.as_bytes(),
            false,
        ) {
            log::debug!("Failed to publish player1/multiplier: {e:#}");
        }
    }
    if s.score {
        let payload = state.score.to_string();
        if let Err(e) = client.publish(&format!("{}/player1/score", pfx), payload.as_bytes(), false)
        {
            log::debug!("Failed to publish player1/score: {e:#}");
        }
    }
    if s.notes_missed {
        let payload = state.notes_missed.to_string();
        if let Err(e) = client.publish(
            &format!("{}/player1/notes_missed", pfx),
            payload.as_bytes(),
            false,
        ) {
            log::debug!("Failed to publish player1/notes_missed: {e:#}");
        }
    }
    if s.notes_hit {
        let payload = state.notes_hit.to_string();
        if let Err(e) = client.publish(
            &format!("{}/player1/notes_hit", pfx),
            payload.as_bytes(),
            false,
        ) {
            log::debug!("Failed to publish player1/notes_hit: {e:#}");
        }
    }

    if s.periodic_state {
        if let Ok(json_bytes) = serde_json::to_vec(state) {
            if let Err(e) = client.publish(&format!("{}/player1/state", pfx), &json_bytes, false) {
                log::debug!("Failed to publish player1/state: {e:#}");
            }
        }
    }
}

fn load_config<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let path = path.as_ref();
    if !path.exists() {
        std::fs::write(path, config::DEFAULT_TOML).with_context(|| {
            format!(
                "Could not create default configuration file '{}'",
                path.display()
            )
        })?;
        return Config::from_toml(config::DEFAULT_TOML)
            .context("Failed to parse embedded default configuration");
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read configuration file '{}'", path.display()))?;

    Config::from_toml(&content)
        .with_context(|| format!("Failed to parse configuration file '{}'", path.display()))
}

fn run_plugin() -> anyhow::Result<()> {
    let plugin_dir = get_plugin_dir();
    let toml_path = plugin_dir.join("gh3_mqtt.toml");

    let config = load_config(&toml_path).with_context(|| {
        format!(
            "Failed to load configuration from '{}'",
            toml_path.display()
        )
    })?;

    if config.debug_log {
        log::set_max_level(LevelFilter::Debug);
    } else {
        log::set_max_level(LevelFilter::Info);
    }

    log::info!("GH3 MQTT Plugin starting up...");
    log::info!(
        "Config loaded: broker={}:{}, prefix={}, poll={}ms, hifreq={}",
        config.host,
        config.port,
        config.topic_prefix,
        config.poll_interval_ms,
        config.hifreq
    );

    let availability_topic = format!("{}/status", config.topic_prefix);
    let client_id = format!("gh3_pc_{:x}", std::process::id());

    let mut client = MqttClient::new(
        config.host.clone(),
        config.port,
        client_id,
        config.username.clone(),
        config.password.clone(),
        availability_topic.clone(),
        "offline".to_string(),
    );

    let mut last_state = GameState::default();
    let mut last_periodic_publish = Instant::now();
    let mut discovery_sent = false;

    // SAFETY: Running inside the Guitar Hero III process where game engine
    // memory is valid.
    let mem = unsafe { gh3::MemoryView::new() };

    while RUNNING.load(Ordering::SeqCst) {
        if !client.is_connected() {
            log::debug!(
                "Connecting to MQTT broker at {}:{}...",
                config.host,
                config.port
            );

            match client.connect() {
                Ok(_) => {
                    log::info!("Connected to MQTT broker successfully!");

                    // Publish availability = online (retained)
                    if let Err(e) = client.publish(&availability_topic, b"online", true) {
                        log::warn!("Failed to publish online availability: {e:#}");
                    }

                    // Publish Home Assistant auto-discovery configs (retained)
                    if config.ha_discovery && !discovery_sent {
                        let msgs = generate_ha_discovery_messages(
                            &config.ha_discovery_prefix,
                            &config.topic_prefix,
                            &config.sensors,
                        );
                        for msg in msgs {
                            if let Err(e) = client.publish(&msg.topic, msg.payload.as_bytes(), true)
                            {
                                log::warn!(
                                    "Failed to publish HA discovery message for '{}': {e:#}",
                                    msg.topic
                                );
                            }
                        }
                        discovery_sent = true;
                        log::debug!("Home Assistant MQTT discovery payloads published.");
                    }

                    // Immediately publish initial values for all entities
                    let initial_state = gh3::read_game_state(&mem);
                    publish_all_entities(&mut client, &config, &initial_state);
                    last_state = initial_state;
                }
                Err(err) => {
                    if err.is_fatal() {
                        return Err(anyhow::Error::from(err).context(format!(
                            "Fatal rejection by MQTT broker at {}:{}",
                            config.host, config.port
                        )));
                    }
                    log::warn!("MQTT connect error: {err}. Retrying in 3 seconds...");
                    thread::sleep(Duration::from_millis(3000));
                    continue;
                }
            }
        }

        // Read current game state safely
        let current_state = gh3::read_game_state(&mem);

        // 1. Song Playing State Change
        if current_state.is_playing != last_state.is_playing {
            let song_active_topic = format!("{}/game/song_active", config.topic_prefix);
            let song_active_payload: &[u8] = if current_state.is_playing {
                b"playing"
            } else {
                b"idle"
            };
            if let Err(e) = client.publish(&song_active_topic, song_active_payload, true) {
                log::debug!("Failed to publish game/song_active: {e:#}");
            }

            if config.sensors.events {
                let event_topic = format!("{}/player1/event", config.topic_prefix);
                let event_payload = if current_state.is_playing {
                    "song_started"
                } else {
                    "song_ended"
                };
                if let Err(e) = client.publish(&event_topic, event_payload.as_bytes(), false) {
                    log::debug!("Failed to publish player1/event: {e:#}");
                }
            }

            log::debug!(
                "Song playing state changed: is_playing={}, song='{}', diff={}, paused={}",
                current_state.is_playing,
                current_state.current_song,
                current_state.difficulty,
                current_state.is_paused
            );

            publish_all_entities(&mut client, &config, &current_state);
        }

        // 2. Song Paused State Change
        if config.sensors.paused && current_state.is_paused != last_state.is_paused {
            let topic = format!("{}/game/paused/state", config.topic_prefix);
            let payload = if current_state.is_paused { "ON" } else { "OFF" };
            if let Err(e) = client.publish(&topic, payload.as_bytes(), false) {
                log::debug!("Failed to publish game/paused/state: {e:#}");
            }

            log::debug!(
                "Game pause state changed: is_paused={}",
                current_state.is_paused
            );
        }

        // 3. Song or Difficulty Change
        if config.sensors.current_song && current_state.current_song != last_state.current_song {
            let topic = format!("{}/game/current_song", config.topic_prefix);
            let payload = if current_state.is_playing {
                &current_state.current_song
            } else {
                "Idle"
            };
            if let Err(e) = client.publish(&topic, payload.as_bytes(), false) {
                log::debug!("Failed to publish game/current_song: {e:#}");
            }

            log::debug!("Song name updated: current_song='{}'", payload);
        }

        if config.sensors.difficulty && current_state.difficulty != last_state.difficulty {
            let topic = format!("{}/game/difficulty", config.topic_prefix);
            if let Err(e) = client.publish(&topic, current_state.difficulty.as_bytes(), false) {
                log::debug!("Failed to publish game/difficulty: {e:#}");
            }
        }

        // 4. Star Power Active State Change
        if current_state.star_power_active != last_state.star_power_active {
            if config.sensors.star_power {
                let topic = format!("{}/player1/star_power/state", config.topic_prefix);
                let payload = if current_state.star_power_active {
                    "ON"
                } else {
                    "OFF"
                };
                if let Err(e) = client.publish(&topic, payload.as_bytes(), false) {
                    log::debug!("Failed to publish player1/star_power/state: {e:#}");
                }

                let attr_topic = format!("{}/player1/star_power/attributes", config.topic_prefix);
                let attr_json = serde_json::json!({
                    "song": current_state.current_song,
                    "difficulty": current_state.difficulty,
                    "gauge": current_state.star_power_gauge,
                    "streak": current_state.streak,
                    "multiplier": current_state.effective_multiplier,
                    "score": current_state.score,
                    "notes_missed": current_state.notes_missed,
                    "notes_hit": current_state.notes_hit,
                    "is_paused": current_state.is_paused,
                });
                if let Err(e) = client.publish(&attr_topic, attr_json.to_string().as_bytes(), false)
                {
                    log::debug!("Failed to publish player1/star_power/attributes: {e:#}");
                }
            }

            if config.sensors.events {
                let event_topic = format!("{}/player1/event", config.topic_prefix);
                let event_payload = if current_state.star_power_active {
                    "star_power_activated"
                } else {
                    "star_power_deactivated"
                };
                if let Err(e) = client.publish(&event_topic, event_payload.as_bytes(), false) {
                    log::debug!("Failed to publish player1/event: {e:#}");
                }
            }

            log::debug!(
                "Star Power state changed: active={} (gauge={:.1}%, streak={}, mult={}x)",
                current_state.star_power_active,
                current_state.star_power_gauge,
                current_state.streak,
                current_state.effective_multiplier,
            );
        }

        // 5. Star Power Gauge
        if config.sensors.star_power_gauge {
            let gauge_diff = (current_state.star_power_gauge - last_state.star_power_gauge).abs();
            if gauge_diff >= config.gauge_threshold_pct
                || (current_state.star_power_active != last_state.star_power_active)
            {
                let topic = format!("{}/player1/star_power/gauge", config.topic_prefix);
                let payload = format!("{:.1}", current_state.star_power_gauge);
                if let Err(e) = client.publish(&topic, payload.as_bytes(), false) {
                    log::debug!("Failed to publish player1/star_power/gauge: {e:#}");
                }
            }
        }

        // 6. Streak Change & Missed Note Detection
        let streak_broken =
            current_state.is_playing && last_state.streak > 0 && current_state.streak == 0;
        let notes_missed_increased =
            current_state.is_playing && current_state.notes_missed > last_state.notes_missed;
        let note_missed = streak_broken || notes_missed_increased;

        if current_state.streak != last_state.streak {
            if config.sensors.streak {
                let topic = format!("{}/player1/streak", config.topic_prefix);
                let payload = current_state.streak.to_string();
                if let Err(e) = client.publish(&topic, payload.as_bytes(), false) {
                    log::debug!("Failed to publish player1/streak: {e:#}");
                }
            }

            if streak_broken {
                log::debug!("Note missed / streak broken! (was {})", last_state.streak);
            }
        }

        // Edge-triggered note_missed event
        if config.sensors.note_missed_event && note_missed {
            let event_topic = format!("{}/player1/event", config.topic_prefix);
            if let Err(e) = client.publish(&event_topic, b"note_missed", false) {
                log::debug!("Failed to publish player1/event note_missed: {e:#}");
            }
        }

        // 7. Notes Missed Counter Change
        if config.sensors.notes_missed && current_state.notes_missed != last_state.notes_missed {
            let topic = format!("{}/player1/notes_missed", config.topic_prefix);
            let payload = current_state.notes_missed.to_string();
            if let Err(e) = client.publish(&topic, payload.as_bytes(), false) {
                log::debug!("Failed to publish player1/notes_missed: {e:#}");
            }
        }

        // 8. Notes Hit Counter Change
        if config.sensors.notes_hit && current_state.notes_hit != last_state.notes_hit {
            let topic = format!("{}/player1/notes_hit", config.topic_prefix);
            let payload = current_state.notes_hit.to_string();
            if let Err(e) = client.publish(&topic, payload.as_bytes(), false) {
                log::debug!("Failed to publish player1/notes_hit: {e:#}");
            }
        }

        // 9. Multiplier Change
        if config.sensors.multiplier
            && current_state.effective_multiplier != last_state.effective_multiplier
        {
            let topic = format!("{}/player1/multiplier", config.topic_prefix);
            let payload = current_state.effective_multiplier.to_string();
            if let Err(e) = client.publish(&topic, payload.as_bytes(), false) {
                log::debug!("Failed to publish player1/multiplier: {e:#}");
            }
        }

        // 10. Total Score Change
        if config.sensors.score && current_state.score != last_state.score {
            let topic = format!("{}/player1/score", config.topic_prefix);
            let payload = current_state.score.to_string();
            if let Err(e) = client.publish(&topic, payload.as_bytes(), false) {
                log::debug!("Failed to publish player1/score: {e:#}");
            }
        }

        // 11. Periodic full state update (every 1 second)
        if config.sensors.periodic_state
            && last_periodic_publish.elapsed() >= Duration::from_millis(1000)
        {
            let topic = format!("{}/player1/state", config.topic_prefix);
            if let Ok(json_bytes) = serde_json::to_vec(&current_state) {
                if let Err(e) = client.publish(&topic, &json_bytes, false) {
                    log::debug!("Failed to publish player1/state: {e:#}");
                }
            }
            last_periodic_publish = Instant::now();
        }

        last_state = current_state;

        // Keepalive / Ping
        if let Err(e) = client.ping_if_needed() {
            log::warn!("Ping error: {e:#}. Will reconnect.");
        }

        thread::sleep(Duration::from_millis(config.poll_interval_ms));
    }

    // Clean shutdown
    if let Err(e) = client.publish(&availability_topic, b"offline", true) {
        log::debug!("Failed to publish offline status on shutdown: {e:#}");
    }
    let song_active_topic = format!("{}/game/song_active", config.topic_prefix);
    if let Err(e) = client.publish(&song_active_topic, b"idle", true) {
        log::debug!("Failed to publish idle status on shutdown: {e:#}");
    }
    client.disconnect();
    log::info!("GH3 MQTT Plugin stopped.");
    Ok(())
}

fn init_logger<P: AsRef<Path>>(log_path: P) -> anyhow::Result<()> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .context("Failed to open plugin log file")?;

    let log_config = ConfigBuilder::new()
        .set_target_level(LevelFilter::Off)
        .set_thread_level(LevelFilter::Off)
        .build();

    let _ = WriteLogger::init(LevelFilter::Debug, log_config, file);
    Ok(())
}

fn worker_main() {
    let plugin_dir = get_plugin_dir();
    let log_path = plugin_dir.join("gh3_mqtt.log");
    if let Err(err) = init_logger(&log_path) {
        show_error_box(
            "Guitar Hero III - MQTT Plugin Error",
            &format!("Failed to initialize plugin logger:\n\n{err:#}"),
        );
        return;
    }

    if let Err(err) = run_plugin() {
        log::error!("Fatal error encountered in GH3 MQTT plugin: {err:#}");
        show_error_box(
            "Guitar Hero III - MQTT Plugin Error",
            &format!("GH3 MQTT Plugin encountered a fatal error and has stopped:\n\n{err:#}"),
        );
        RUNNING.store(false, Ordering::SeqCst);
    }
}

/// Standard Windows DLL entry point.
///
/// # Safety
///
/// Must be invoked by the Windows dynamic linker with valid parameters during
/// process/thread attach and detach lifecycle events.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    hinst: HMODULE,
    reason: u32,
    _reserved: *mut std::ffi::c_void,
) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            // SAFETY: Safe to disable thread attach/detach calls as this DLL
            // does not track thread state.
            let _ = unsafe { DisableThreadLibraryCalls(hinst) };
            RUNNING.store(true, Ordering::SeqCst);
            thread::spawn(worker_main);
        }
        DLL_PROCESS_DETACH => {
            RUNNING.store(false, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(50));
        }
        _ => {}
    }
    1
}
