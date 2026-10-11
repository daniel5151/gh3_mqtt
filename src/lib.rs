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

struct Publisher<'a> {
    client: &'a mut MqttClient,
    prefix: &'a str,
}

impl<'a> Publisher<'a> {
    fn new(client: &'a mut MqttClient, prefix: &'a str) -> Self {
        Self { client, prefix }
    }

    fn publish(&mut self, subtopic: &str, payload: impl ToString) {
        self.publish_opts(subtopic, payload, false);
    }

    fn publish_retained(&mut self, subtopic: &str, payload: impl ToString) {
        self.publish_opts(subtopic, payload, true);
    }

    fn publish_opts(&mut self, subtopic: &str, payload: impl ToString, retain: bool) {
        let topic = format!("{}/{}", self.prefix, subtopic);
        let payload = payload.to_string();
        if let Err(e) = self.client.publish(&topic, payload.as_bytes(), retain) {
            log::debug!("Failed to publish {subtopic}: {e:#}");
        }
    }
}

/// Builds the Home Assistant `json_attributes_topic` payload for
/// `binary_sensor.gh3_star_power`.
///
/// This provides contextual snapshot metadata (current song, difficulty, gauge,
/// streak, multiplier, score, notes missed/hit) attached directly to the Star
/// Power activation trigger without requiring high-frequency (`hifreq = true`)
/// per-note MQTT telemetry.
fn star_power_attributes(state: &GameState) -> serde_json::Value {
    serde_json::json!({
        "song": state.current_song,
        "difficulty": state.difficulty,
        "gauge": state.star_power_gauge,
        "streak": state.streak,
        "multiplier": state.effective_multiplier,
        "score": state.score,
        "notes_missed": state.notes_missed,
        "notes_hit": state.notes_hit,
        "is_paused": state.is_paused,
    })
}

fn publish_game_state(
    publisher: &mut Publisher<'_>,
    config: &Config,
    current: &GameState,
    last: Option<&GameState>,
) {
    let s = &config.sensors;
    let force = last.is_none_or(|prev| current.is_playing != prev.is_playing);

    // 1. Song Active Availability & Transition Events
    if force {
        publisher.publish_retained(
            "game/song_active",
            if current.is_playing {
                "playing"
            } else {
                "idle"
            },
        );

        if last.is_some() {
            if s.events {
                publisher.publish(
                    "player1/event",
                    if current.is_playing {
                        "song_started"
                    } else {
                        "song_ended"
                    },
                );
            }
            log::debug!(
                "Song playing state changed: is_playing={}, song='{}', diff={}, paused={}",
                current.is_playing,
                current.current_song,
                current.difficulty,
                current.is_paused
            );
        }
    }

    // 2. Song Paused State
    if s.paused && (force || last.is_some_and(|p| current.is_paused != p.is_paused)) {
        publisher.publish(
            "game/paused/state",
            if current.is_paused { "ON" } else { "OFF" },
        );
        if !force {
            log::debug!("Game pause state changed: is_paused={}", current.is_paused);
        }
    }

    // 3. Current Song
    if s.current_song && (force || last.is_some_and(|p| current.current_song != p.current_song)) {
        let song_text = if current.is_playing {
            &current.current_song
        } else {
            "Idle"
        };
        publisher.publish("game/current_song", song_text);
        if !force {
            log::debug!("Song name updated: current_song='{}'", song_text);
        }
    }

    // 4. Difficulty
    if s.difficulty && (force || last.is_some_and(|p| current.difficulty != p.difficulty)) {
        publisher.publish("game/difficulty", &current.difficulty);
    }

    // 5. Star Power Active State & Events
    let sp_active_changed = last.is_some_and(|p| current.star_power_active != p.star_power_active);
    if s.star_power && (force || sp_active_changed) {
        publisher.publish(
            "player1/star_power/state",
            if current.star_power_active {
                "ON"
            } else {
                "OFF"
            },
        );
        publisher.publish(
            "player1/star_power/attributes",
            star_power_attributes(current),
        );
    }
    if !force && sp_active_changed {
        if s.events {
            publisher.publish(
                "player1/event",
                if current.star_power_active {
                    "star_power_activated"
                } else {
                    "star_power_deactivated"
                },
            );
        }
        log::debug!(
            "Star Power state changed: active={} (gauge={:.1}%, streak={}, mult={}x)",
            current.star_power_active,
            current.star_power_gauge,
            current.streak,
            current.effective_multiplier,
        );
    }

    // 6. Star Power Gauge
    if s.star_power_gauge {
        let gauge_changed = force
            || last.is_some_and(|p| {
                let gauge_diff = (current.star_power_gauge - p.star_power_gauge).abs();
                gauge_diff >= config.gauge_threshold_pct
                    || current.star_power_active != p.star_power_active
            });
        if gauge_changed {
            publisher.publish(
                "player1/star_power/gauge",
                format!("{:.1}", current.star_power_gauge),
            );
        }
    }

    // 7. Streak & Missed Note Detection
    let streak_changed = last.is_some_and(|p| current.streak != p.streak);
    if s.streak && (force || streak_changed) {
        publisher.publish("player1/streak", current.streak);
    }

    if let Some(prev) = last {
        if !force {
            let streak_broken = current.is_playing && prev.streak > 0 && current.streak == 0;
            let notes_missed_increased =
                current.is_playing && current.notes_missed > prev.notes_missed;
            let note_missed = streak_broken || notes_missed_increased;

            if streak_broken {
                log::debug!("Note missed / streak broken! (was {})", prev.streak);
            }
            if s.note_missed_event && note_missed {
                publisher.publish("player1/event", "note_missed");
            }
        }
    }

    // 8. Notes Missed Counter
    if s.notes_missed && (force || last.is_some_and(|p| current.notes_missed != p.notes_missed)) {
        publisher.publish("player1/notes_missed", current.notes_missed);
    }

    // 9. Notes Hit Counter
    if s.notes_hit && (force || last.is_some_and(|p| current.notes_hit != p.notes_hit)) {
        publisher.publish("player1/notes_hit", current.notes_hit);
    }

    // 10. Multiplier
    if s.multiplier
        && (force || last.is_some_and(|p| current.effective_multiplier != p.effective_multiplier))
    {
        publisher.publish("player1/multiplier", current.effective_multiplier);
    }

    // 11. Score
    if s.score && (force || last.is_some_and(|p| current.score != p.score)) {
        publisher.publish("player1/score", current.score);
    }

    // 12. Initial / On-Transition Full State
    if force && s.periodic_state {
        if let Ok(json) = serde_json::to_string(current) {
            publisher.publish("player1/state", json);
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
                    let mut publisher = Publisher::new(&mut client, &config.topic_prefix);
                    publish_game_state(&mut publisher, &config, &initial_state, None);
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

        let mut publisher = Publisher::new(&mut client, &config.topic_prefix);
        publish_game_state(&mut publisher, &config, &current_state, Some(&last_state));

        // Periodic full state update (every 1 second)
        if config.sensors.periodic_state
            && last_periodic_publish.elapsed() >= Duration::from_millis(1000)
        {
            if let Ok(json) = serde_json::to_string(&current_state) {
                publisher.publish("player1/state", json);
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
    let mut publisher = Publisher::new(&mut client, &config.topic_prefix);
    publisher.publish_retained("status", "offline");
    publisher.publish_retained("game/song_active", "idle");
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
