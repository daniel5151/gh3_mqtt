# GH3 MQTT Plugin (`gh3_mqtt.dll`)

A lightweight, zero-hitch native plugin for **Guitar Hero III (PC / BetterGH3 / GH3+)** written in **Rust** that monitors gameplay and Star Power state in real-time and broadcasts them over MQTT to drive Home Assistant automations (e.g. flashing smart lights when Star Power is active!).

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

## ⚙️ Configuration (`PLUGINS/gh3_mqtt.ini`)

Edit `PLUGINS/gh3_mqtt.ini` to configure your MQTT broker connection:

```ini
[mqtt]
# MQTT broker host/IP address
host = 192.168.1.100
port = 1883

# Optional MQTT broker credentials (leave blank if not needed)
username =
password =

# Base topic prefix (default: gh3)
topic_prefix = gh3

# Home Assistant MQTT Auto-Discovery (creates the Device & Entities in HA automatically)
ha_discovery = true
ha_discovery_prefix = homeassistant

# Polling interval in milliseconds (16ms = ~60 FPS)
poll_interval_ms = 16

# Minimum change in Star Power gauge % before sending an MQTT message (avoids flooding)
gauge_threshold_pct = 1.0

# Write diagnostic logs to PLUGINS\gh3_mqtt.log
debug_log = true
```

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

To rebuild the plugin:

```powershell
cd gh3_mqtt
cargo build --release --target i686-pc-windows-msvc
Copy-Item target\i686-pc-windows-msvc\release\gh3_mqtt.dll ..\PLUGINS\gh3_mqtt.dll -Force
```
