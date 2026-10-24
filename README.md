# GH3 MQTT Plugin (`gh3_mqtt.dll`)

A lightweight plugin for **Guitar Hero III (PC / BetterGH3 / GH3+)** that monitors gameplay and Star Power state in real-time and broadcasts them over MQTT to drive Home Assistant automations (e.g. flashing smart lights when Star Power is active!).

> [!NOTE]
>
> In case you can't tell by the emojis in the README, this this is 100% free range Gemini 3.8 vibe code.
>
> That said, if you check the git history - you'll find that I did take quite some time to un-slop it, and IMO, the code is now in solid shape, and what I would consider to be fairly idiomatic Rust.
>
> Not strictly necessary, given the "weekend shitpost" nature of this project... but hey - gotta still have _some_ pride in your dying craft, right?

[![GitHub Release](https://img.shields.io/github/v/release/daniel5151/gh3_mqtt?logo=github&label=Release)](https://github.com/daniel5151/gh3_mqtt/releases/latest)
[![Download gh3_mqtt.dll](https://img.shields.io/badge/download-gh3__mqtt.dll-success?logo=windows)](https://github.com/daniel5151/gh3_mqtt/releases/latest/download/gh3_mqtt.dll)

---

## 🎬 Demo

https://github.com/user-attachments/assets/c7dda198-50af-4947-80f1-c5d4be6b58f6

---

## ⚡ Features

- **Live Star Power Tracking**:
  - `gh3/player1/star_power/state`: `ON` / `OFF`
  - `gh3/player1/star_power/gauge`: Float percentage (`0.0` - `100.0`%)
  - `gh3/player1/event`: Edge-triggered events (`star_power_activated`, `star_power_deactivated`, `song_started`, `song_ended`)
  - `gh3/player1/star_power/attributes`: JSON metadata with current gauge, streak, multiplier, score
- **Game & Song Entities**:
  - `gh3/game/current_song`: Reports `"Song Title - Artist"` when playing, or `"Idle"` in menus
  - `gh3/game/difficulty`: Current selected difficulty (`"Easy"`, `"Medium"`, `"Hard"`, `"Expert"`)
  - `gh3/game/paused/state`: `binary_sensor.gh3_paused` (`ON` / `OFF`) tracking in-song pause state
- **Gameplay-Tied Entities (Available only while playing a song)**:
  - `gh3/player1/star_power/state`: `ON` / `OFF`
  - `gh3/player1/star_power/gauge`: Float percentage (`0.0` - `100.0`%)
  - `gh3/player1/streak`: Note streak / combo count
  - `gh3/player1/multiplier`: Dynamic multiplier (`1x`, `2x`, `3x`, `4x` normally, doubling to `2x`, `4x`, `6x`, `8x` during Star Power!)
  - `gh3/player1/score`: Current song score
  - `gh3/player1/notes_missed`: Counter of missed notes in current song
  - `gh3/player1/notes_hit`: Counter of successfully hit notes
  - `gh3/player1/state`: Full synchronized JSON state published periodically
- **Instant Event Triggers (`gh3/player1/event`)**:
  - `star_power_activated` / `star_power_deactivated`
  - `note_missed` (instantly fires on note miss or broken streak)
  - `song_started` / `song_ended`
- **Smart Home Assistant Availability**:
  - Automatically registers all entities under a unified device: **Guitar Hero III**
  - **In-Game Availability**: Gameplay sensors (Star Power, Gauge, Streak, Multiplier, Score, Missed Notes) cleanly transition to **`Unavailable`** when in menus, and become active the moment a song starts!
  - **Game Availability**: Last-Will-and-Testament (LWT) at `gh3/status` (`online`/`offline`) marks all entities as unavailable when the game closes.
- **Engine Safety**:
  - Non-blocking asynchronous network worker thread.
  - Zero hitching or stuttering during gameplay even on network drops or reconnects.
  - Safe memory page bounds checks before dereferencing game pointers.

---

## 📥 Installation

1. **Download [`gh3_mqtt.dll`](https://github.com/daniel5151/gh3_mqtt/releases/latest/download/gh3_mqtt.dll)** from the [Latest GitHub Release](https://github.com/daniel5151/gh3_mqtt/releases/latest).
2. Copy `gh3_mqtt.dll` into your Guitar Hero III `PLUGINS\` folder (e.g., `E:\Guitar Hero III\PLUGINS\` or alongside your BetterGH3 / GH3+ plugins).
3. Create/edit `PLUGINS/gh3_mqtt.toml` to configure your MQTT broker connection (see [Configuration](#-configuration-pluginsgh3_mqtttoml) below).
4. Launch Guitar Hero III!

---

## ⚙️ Configuration (`PLUGINS/gh3_mqtt.toml`)

Edit `PLUGINS/gh3_mqtt.toml` to configure your MQTT broker connection and telemetry reporting:

```toml
# MQTT broker host/IP address
host = "192.168.1.100"
port = 1883

# Optional MQTT broker credentials (leave commented or unset if not needed)
# username = "my_user"
# password = "my_password"

# Base topic prefix (default: gh3)
topic_prefix = "gh3"

# Home Assistant MQTT Auto-Discovery (creates the Device & Entities in HA automatically)
ha_discovery = true
ha_discovery_prefix = "homeassistant"

# Polling interval in milliseconds (16ms = ~60 FPS)
poll_interval_ms = 16

# Minimum change in Star Power gauge % before sending an MQTT message (avoids flooding)
gauge_threshold_pct = 1.0

# Write diagnostic logs to PLUGINS\gh3_mqtt.log
debug_log = true

# Master toggle for high-frequency telemetry and events (score, streak,
# multiplier, notes_hit, notes_missed, star_power_gauge, periodic state, and note_missed event).
# Off (false) by default to prevent flooding MQTT brokers and Home Assistant databases.
hifreq = false

[sensors]
# Individual sensor and event toggles (uncomment to override the hifreq default)
# star_power = true
# current_song = true
# difficulty = true
# paused = true
# events = true
# star_power_gauge = true
# streak = true
# multiplier = true
# score = true
# notes_hit = true
# notes_missed = true
# note_missed_event = true
# periodic_state = true
```

### ⚡ Low-Frequency vs High-Frequency Telemetry

To avoid flooding MQTT brokers and Home Assistant recorder databases with rapid note-by-note telemetry during intense songs (e.g. 15-30 note changes per second):

* **`hifreq` is disabled (`false`) by default**: Rapid updates (`score`, `streak`, `multiplier`, `notes_hit`, `notes_missed`, `star_power_gauge`, periodic full state, and `note_missed` edge events) are suppressed out of the box.
* Essential low-frequency state updates (`star_power` active, `current_song`, `difficulty`, `paused`, `song_started`/`ended`, `star_power_activated`/`deactivated`) are published.
* Set **`hifreq = true`** at the top level to enable all high-frequency sensors and events at once.
* Any individual sensor or event can be selectively toggled inside the `[sensors]` table by uncommenting its line (e.g. leaving `hifreq = false` while enabling `score = true`).

---

## 💡 Home Assistant Automation Example

Once connected, Home Assistant automatically discovers `binary_sensor.gh3_star_power`. Here is an example automation to flash your lights when Star Power is activated and restore them when depleted:

```yaml
alias: Guitar Hero III - Smooth Pulse Living Room Lights on Star Power
description: >-
  Smoothly pulses all lights while Star Power is active, then individually
  restores each light's prior state.
triggers:
  - trigger: state
    entity_id: binary_sensor.guitar_hero_iii_star_power
    to: 'on'
actions:
  - variables:
      target_lights:
        - light.your_light_1
        - light.your_light_2
      light_states: |-
        {% set ns = namespace(states=[]) %} {% for l in target_lights %}
          {% set ns.states = ns.states + [{
            'entity_id': l,
            'was_on': is_state(l, 'on'),
            'brightness': state_attr(l, 'brightness') | default(128, true)
          }] %}
        {% endfor %} {{ ns.states }}
  - repeat:
      while:
        - condition: state
          entity_id: binary_sensor.guitar_hero_iii_star_power
          state: 'on'
      sequence:
        - action: light.turn_on
          target:
            entity_id:
              - light.your_light_1
              - light.your_light_2
          data:
            brightness: 255
        - delay:
            milliseconds: 500
        - action: light.turn_on
          target:
            entity_id:
              - light.your_light_1
              - light.your_light_2
          data:
            brightness: 60
        - delay:
            milliseconds: 500
  - repeat:
      for_each: '{{ light_states }}'
      sequence:
        - if:
            - condition: template
              value_template: '{{ repeat.item.was_on }}'
          then:
            - action: light.turn_on
              target:
                entity_id: '{{ repeat.item.entity_id }}'
              data:
                brightness: '{{ repeat.item.brightness }}'
          else:
            - action: light.turn_off
              target:
                entity_id: '{{ repeat.item.entity_id }}'
mode: single

```

### 🔴 Flash Red on Missed Note Example

```yaml
alias: "GH3 - Flash Red on Missed Note"
description: "Flashes lights red for 300ms whenever a note is missed in Guitar Hero III"
trigger:
  - platform: mqtt
    topic: "gh3/player1/event"
    payload: "note_missed"
action:
  - service: light.turn_on
    target:
      entity_id: light.gaming_room
    data:
      rgb_color: [255, 0, 0]
      brightness: 255
      flash: short
mode: restart
```

---

## 🛠️ Building From Source

### Standard Release Build (Stable)

Configured with `min-sized-rust` release profile flags (`opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`) and default `i686-pc-windows-msvc` target:

```powershell
cd gh3_mqtt
cargo build --release
```
*Output size: **~399 KB**.*

### Ultra-Crunched Build (Nightly + `build-std`)

For maximum size crunching with `build-std`, `panic = "immediate-abort"`, and location/debug stripped:

```powershell
cd gh3_mqtt
.\build-min.ps1
```
*Output size: **~296 KB** (~25% reduction).*

> **Tip:** If you prefer not to build from source, pre-compiled Ultra-Crunched binaries are available under [GitHub Releases](https://github.com/daniel5151/gh3_mqtt/releases).
