use serde::Serialize;
use std::ffi::c_void;
use windows::Win32::System::Memory::{
    VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READ,
    PAGE_EXECUTE_READWRITE, PAGE_READONLY, PAGE_READWRITE,
};

pub const ADDR_IS_PLAYING_SONG: usize = 0x00B4EB21;
pub const ADDR_SONG_PAUSED: usize = 0x00B4E320;
pub const ADDR_CURRENT_SONG_KEY: usize = 0x00A13314;
pub const ADDR_SONG_: usize = 0x00A1331C;
pub const ADDR_ITEM_MAP: usize = 0x00C03AA8;
pub const ADDR_P1_STATUS_PTR: usize = 0x00A12FCC;
pub const ADDR_SCORE_OBJECT: usize = 0x00A12FD0;

pub const KEY_PLAYER1_STATUS: u32 = 0xD95930AC;
pub const KEY_STAR_POWER_USED: u32 = 0x92BE3D33;
pub const KEY_STAR_POWER_AMOUNT: u32 = 0x35DB08DE;
pub const KEY_STAR_POWER_USABLE: u32 = 0xD37CE106;
pub const KEY_CURRENT_RUN: u32 = 0xA675A409;
pub const KEY_SCORE: u32 = 0xCD66C8AE;
#[allow(dead_code)]
pub const KEY_NUM_MULTIPLIER: u32 = 0xD472162F;
pub const KEY_CURRENT_HEALTH: u32 = 0x36EC0682;
pub const KEY_TOTAL_NOTES: u32 = 0x3D96CF41;
pub const KEY_NOTES_HIT: u32 = 0x4C0D1319;

pub const KEY_CURRENT_DIFFICULTY: u32 = 0x9B2F5962;
pub const KEY_CURRENT_SONG: u32 = 0x03A17838;
pub const KEY_FASTGH3_META: u32 = 0xCE4258D2;
pub const KEY_GH3_SONGLIST_PROPS: u32 = 0x6F53CF2D;
pub const KEY_PERMANENT_SONGLIST_PROPS: u32 = 0x5A93AE17;
pub const KEY_TITLE: u32 = 0xD4C98794;
pub const KEY_ARTIST: u32 = 0xFEA66978;
pub const KEY_AUTHOR: u32 = 0x42502737;

#[repr(C, packed(4))]
pub struct HashMapNode {
    pub byte0: u8,
    pub byte1: u8,
    pub item_type: u8,
    pub byte3: u8,
    pub key: u32,
    pub dword8: u32,
    pub value: u32,
    pub next_item: *const HashMapNode,
}

#[repr(C, packed(2))]
pub struct QbStruct {
    pub unk0: u16,
    pub unk2: u8,
    pub unk3: u8,
    pub first: *const QbStructItem,
}

#[repr(C, packed(1))]
pub struct QbStructItem {
    pub unk_flag0: u8,
    pub flags: u8,
    pub word2: u16,
    pub key: u32,
    pub value: u32,
    pub next: *const QbStructItem,
}

#[repr(C, packed(4))]
pub struct QbMapNode {
    pub key: u32,
    pub value: u32,
    pub next_node: *const QbMapNode,
}

#[repr(C)]
pub struct QbMap {
    pub size: u32,
    pub nodes: *const QbMapNode,
    pub type_flags: u8,
    pub unk_c: u32,
    pub unk_10: u32,
    pub unk_14: u8,
    pub bool_flag: u8,
    pub keep_alive: u8,
    pub unk_17: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GameState {
    pub is_playing: bool,
    pub is_paused: bool,
    pub star_power_active: bool,
    pub star_power_gauge: f32,
    pub star_power_usable: bool,
    pub streak: u32,
    pub multiplier: u32,
    pub effective_multiplier: u32,
    pub score: u32,
    pub rock_meter: f32,
    pub notes_hit: u32,
    pub total_notes: u32,
    pub notes_missed: u32,
    pub current_song: String,
    pub difficulty: String,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            is_playing: false,
            is_paused: false,
            star_power_active: false,
            star_power_gauge: 0.0,
            star_power_usable: false,
            streak: 0,
            multiplier: 1,
            effective_multiplier: 1,
            score: 0,
            rock_meter: 1.0,
            notes_hit: 0,
            total_notes: 0,
            notes_missed: 0,
            current_song: "Idle".to_string(),
            difficulty: "Expert".to_string(),
        }
    }
}

#[allow(dead_code)]
pub fn qb_hash(s: &str) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &b in s.as_bytes() {
        let ch = match b {
            b'A'..=b'Z' => b + 32,
            b'/' => b'\\',
            _ => b,
        };
        crc = crc32_byte(ch, crc);
    }
    crc
}

#[allow(dead_code)]
fn crc32_byte(b: u8, crc: u32) -> u32 {
    let mut c = (crc ^ (b as u32)) & 0xFF;
    for _ in 0..8 {
        c = if (c & 1) != 0 {
            0xEDB88320 ^ (c >> 1)
        } else {
            c >> 1
        };
    }
    c ^ (crc >> 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qb_hashes() {
        assert_eq!(qb_hash("player1_status"), KEY_PLAYER1_STATUS);
        assert_eq!(qb_hash("star_power_used"), KEY_STAR_POWER_USED);
        assert_eq!(qb_hash("star_power_amount"), KEY_STAR_POWER_AMOUNT);
        assert_eq!(qb_hash("star_power_usable"), KEY_STAR_POWER_USABLE);
        assert_eq!(qb_hash("current_run"), KEY_CURRENT_RUN);
        assert_eq!(qb_hash("current_health"), KEY_CURRENT_HEALTH);
        assert_eq!(qb_hash("score"), KEY_SCORE);
        assert_eq!(qb_hash("num_multiplier"), KEY_NUM_MULTIPLIER);
        assert_eq!(qb_hash("total_notes"), KEY_TOTAL_NOTES);
        assert_eq!(qb_hash("notes_hit"), KEY_NOTES_HIT);
        assert_eq!(qb_hash("current_difficulty"), KEY_CURRENT_DIFFICULTY);
        assert_eq!(qb_hash("current_song"), KEY_CURRENT_SONG);
        assert_eq!(qb_hash("fastgh3_meta"), KEY_FASTGH3_META);
        assert_eq!(qb_hash("gh3_songlist_props"), KEY_GH3_SONGLIST_PROPS);
        assert_eq!(qb_hash("permanent_songlist_props"), KEY_PERMANENT_SONGLIST_PROPS);
        assert_eq!(qb_hash("title"), KEY_TITLE);
        assert_eq!(qb_hash("artist"), KEY_ARTIST);
        assert_eq!(qb_hash("author"), KEY_AUTHOR);
    }

    #[test]
    fn test_known_songs() {
        assert_eq!(known_song_name(0xFC3DB2E8), Some("Slow Ride - Foghat"));
        assert_eq!(known_song_name(0x8CF2B87A), Some("Cherub Rock - The Smashing Pumpkins"));
        assert_eq!(known_song_name(0x8593790E), Some("One - Metallica"));
        assert_eq!(known_song_name(0x40797ADD), Some("Through the Fire and Flames - DragonForce"));
        assert_eq!(known_song_name(0xB3AE1B7A), Some("Bulls on Parade - Rage Against the Machine"));
    }
}

pub unsafe fn is_readable(ptr: *const c_void, size: usize) -> bool {
    let addr = ptr as usize;
    if addr < 0x00010000 || addr > 0x7FFF0000 {
        return false;
    }
    let mut mbi = MEMORY_BASIC_INFORMATION::default();
    let res = VirtualQuery(
        Some(ptr),
        &mut mbi,
        std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
    );
    if res == 0 {
        return false;
    }
    if mbi.State != MEM_COMMIT {
        return false;
    }
    let protect = mbi.Protect;
    let allowed = PAGE_READONLY | PAGE_READWRITE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE;
    (protect.0 & allowed.0) != 0 && (addr + size <= mbi.BaseAddress as usize + mbi.RegionSize)
}

pub unsafe fn read_game_state() -> GameState {
    let mut state = GameState::default();

    // 1. Check if game is currently playing a song
    if is_readable(ADDR_IS_PLAYING_SONG as *const c_void, 1) {
        state.is_playing = *(ADDR_IS_PLAYING_SONG as *const u8) != 0;
    }

    // 2. Check if song is paused (only valid while playing)
    if state.is_playing && is_readable(ADDR_SONG_PAUSED as *const c_void, 1) {
        state.is_paused = *(ADDR_SONG_PAUSED as *const u8) != 0;
    }

    // 3. Locate GlobalMap table
    let mut map_table: *const *const HashMapNode = std::ptr::null();
    if is_readable(ADDR_ITEM_MAP as *const c_void, 4) {
        let tbl = *(ADDR_ITEM_MAP as *const *const *const HashMapNode);
        if !tbl.is_null() && is_readable(tbl as *const c_void, 0x8000 * 4) {
            map_table = tbl;
        }
    }

    // 4. Read current difficulty if GlobalMap is valid
    if !map_table.is_null() {
        let diff_node = find_global_node(map_table, KEY_CURRENT_DIFFICULTY);
        if !diff_node.is_null() && is_readable(diff_node as *const c_void, std::mem::size_of::<HashMapNode>()) {
            state.difficulty = resolve_difficulty((*diff_node).value).to_string();
        }
    }

    // 5. Read song info if playing
    if state.is_playing {
        state.current_song = read_song_info(map_table).unwrap_or_else(|| "Playing".to_string());
    } else {
        state.current_song = "Idle".to_string();
    }

    // 4. Locate player1_status struct pointer
    let mut p1_struct_ptr: *const QbStruct = std::ptr::null();

    // Try ADDR_P1_STATUS_PTR (0x00A12FCC) first
    if is_readable(ADDR_P1_STATUS_PTR as *const c_void, 4) {
        let direct_ptr = *(ADDR_P1_STATUS_PTR as *const *const QbStruct);
        if !direct_ptr.is_null() && is_readable(direct_ptr as *const c_void, std::mem::size_of::<QbStruct>()) {
            p1_struct_ptr = direct_ptr;
        }
    }

    // Fall back to GlobalMap lookup
    if p1_struct_ptr.is_null() && !map_table.is_null() {
        let node = find_global_node(map_table, KEY_PLAYER1_STATUS);
        if !node.is_null() && is_readable(node as *const c_void, std::mem::size_of::<HashMapNode>()) {
            let ptr = (*node).value as *const QbStruct;
            if !ptr.is_null() && is_readable(ptr as *const c_void, std::mem::size_of::<QbStruct>()) {
                p1_struct_ptr = ptr;
            }
        }
    }

    if p1_struct_ptr.is_null() {
        return state;
    }

    // 5. Extract player1_status fields
    if let Some(item) = find_struct_item(p1_struct_ptr, KEY_STAR_POWER_USED) {
        state.star_power_active = (*item).value != 0;
    }

    if let Some(item) = find_struct_item(p1_struct_ptr, KEY_STAR_POWER_AMOUNT) {
        let type_val = (*item).flags >> 1;
        state.star_power_gauge = if type_val == 2 {
            f32::from_bits((*item).value).clamp(0.0, 100.0)
        } else if type_val == 1 {
            ((*item).value as f32).clamp(0.0, 100.0)
        } else {
            0.0
        };
    }

    if let Some(item) = find_struct_item(p1_struct_ptr, KEY_STAR_POWER_USABLE) {
        state.star_power_usable = (*item).value != 0;
    }

    if let Some(item) = find_struct_item(p1_struct_ptr, KEY_CURRENT_RUN) {
        state.streak = (*item).value;
    }

    // Multiplier climbs 1x -> 2x -> 3x -> 4x with streak (0-9, 10-19, 20-29, 30+ notes)
    // Doubles to 2x, 4x, 6x, 8x during Star Power!
    let base_mult = match state.streak {
        0..=9 => 1,
        10..=19 => 2,
        20..=29 => 3,
        _ => 4,
    };
    state.multiplier = base_mult;
    state.effective_multiplier = if state.star_power_active {
        base_mult * 2
    } else {
        base_mult
    };

    if let Some(item) = find_struct_item(p1_struct_ptr, KEY_SCORE) {
        let type_val = (*item).flags >> 1;
        state.score = if type_val == 2 {
            f32::from_bits((*item).value).max(0.0) as u32
        } else {
            (*item).value
        };
    }

    // If score was not in p1_struct, check ADDR_SCORE_OBJECT
    if state.score == 0 && is_readable(ADDR_SCORE_OBJECT as *const c_void, 4) {
        let score_obj_ptr = *(ADDR_SCORE_OBJECT as *const *const QbStruct);
        if !score_obj_ptr.is_null() && is_readable(score_obj_ptr as *const c_void, std::mem::size_of::<QbStruct>()) {
            if let Some(item) = find_struct_item(score_obj_ptr, KEY_SCORE) {
                let type_val = (*item).flags >> 1;
                state.score = if type_val == 2 {
                    f32::from_bits((*item).value).max(0.0) as u32
                } else {
                    (*item).value
                };
            }
        }
    }

    if let Some(item) = find_struct_item(p1_struct_ptr, KEY_CURRENT_HEALTH) {
        let type_val = (*item).flags >> 1;
        state.rock_meter = if type_val == 2 {
            f32::from_bits((*item).value)
        } else {
            (*item).value as f32
        };
    }

    if let Some(item) = find_struct_item(p1_struct_ptr, KEY_NOTES_HIT) {
        state.notes_hit = (*item).value;
    }

    if let Some(item) = find_struct_item(p1_struct_ptr, KEY_TOTAL_NOTES) {
        state.total_notes = (*item).value;
    }

    state.notes_missed = state.total_notes.saturating_sub(state.notes_hit);

    state
}

pub fn resolve_difficulty(diff_key: u32) -> &'static str {
    match diff_key {
        0xB69D6568 => "Easy",
        0x398CBA48 => "Medium",
        0x3EEAE02D => "Hard",
        0xB0E46CBD => "Expert",
        _ => "Expert",
    }
}

pub unsafe fn read_song_info(map_table: *const *const HashMapNode) -> Option<String> {
    // 1. Check fastgh3_meta struct (FastGH3 custom songs)
    if !map_table.is_null() {
        let meta_node = find_global_node(map_table, KEY_FASTGH3_META);
        if !meta_node.is_null() && is_readable(meta_node as *const c_void, std::mem::size_of::<HashMapNode>()) {
            let meta_struct = (*meta_node).value as *const QbStruct;
            if !meta_struct.is_null() && is_readable(meta_struct as *const c_void, std::mem::size_of::<QbStruct>()) {
                let title = find_struct_item(meta_struct, KEY_TITLE).and_then(|it| extract_string_from_item(it));
                let author = find_struct_item(meta_struct, KEY_AUTHOR)
                    .or_else(|| find_struct_item(meta_struct, KEY_ARTIST))
                    .and_then(|it| extract_string_from_item(it));
                if let Some(t) = title {
                    if !t.is_empty() {
                        return Some(if let Some(a) = author {
                            if !a.is_empty() {
                                format!("{} - {}", t, a)
                            } else {
                                t
                            }
                        } else {
                            t
                        });
                    }
                }
            }
        }
    }

    // 2. Read native active song QbKey
    let mut song_key: u32 = 0;
    if is_readable(ADDR_CURRENT_SONG_KEY as *const c_void, 4) {
        song_key = *(ADDR_CURRENT_SONG_KEY as *const u32);
    }
    if song_key == 0 && is_readable(ADDR_SONG_ as *const c_void, 4) {
        song_key = *(ADDR_SONG_ as *const u32);
    }
    if song_key == 0 && !map_table.is_null() {
        let song_node = find_global_node(map_table, KEY_CURRENT_SONG);
        if !song_node.is_null() && is_readable(song_node as *const c_void, std::mem::size_of::<HashMapNode>()) {
            song_key = (*song_node).value;
        }
    }

    if song_key == 0 {
        return None;
    }

    // 3. Check known song lookup table (canonical Neversoft GH3 tracklist)
    if let Some(name) = known_song_name(song_key) {
        return Some(name.to_string());
    }

    // 4. Check gh3_songlist_props & permanent_songlist_props in GlobalMap
    if !map_table.is_null() {
        for &props_key in &[KEY_GH3_SONGLIST_PROPS, KEY_PERMANENT_SONGLIST_PROPS] {
            let props_node = find_global_node(map_table, props_key);
            if !props_node.is_null() && is_readable(props_node as *const c_void, std::mem::size_of::<HashMapNode>()) {
                let props_struct = (*props_node).value as *const QbStruct;
                if !props_struct.is_null() && is_readable(props_struct as *const c_void, std::mem::size_of::<QbStruct>()) {
                    if let Some(song_item) = find_struct_item(props_struct, song_key) {
                        let song_props = (*song_item).value as *const QbStruct;
                        if !song_props.is_null() && is_readable(song_props as *const c_void, std::mem::size_of::<QbStruct>()) {
                            let title = find_struct_item(song_props, KEY_TITLE).and_then(|it| extract_string_from_item(it));
                            let artist = find_struct_item(song_props, KEY_ARTIST).and_then(|it| extract_string_from_item(it));
                            if let Some(t) = title {
                                if !t.is_empty() {
                                    return Some(if let Some(a) = artist {
                                        if !a.is_empty() {
                                            format!("{} - {}", t, a)
                                        } else {
                                            t
                                        }
                                    } else {
                                        t
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Fallback for unmapped custom song
    Some(format!("Song 0x{:08X}", song_key))
}

pub fn known_song_name(key: u32) -> Option<&'static str> {
    match key {
        // Career / Setlist (Tier 1-8)
        0xFC3DB2E8 => Some("Slow Ride - Foghat"),
        0xD10828C8 => Some("Talk Dirty to Me - Poison"),
        0x68E8ACF5 => Some("Hit Me with Your Best Shot - Pat Benatar"),
        0x24E5E579 => Some("Story of My Life - Social Distortion"),
        0xDA21ECBA => Some("Rock and Roll All Nite - Kiss"),
        0x71B14112 => Some("Sabotage - Beastie Boys"),
        0xB2BA98AD => Some("Mississippi Queen - Mountain"),
        0x80540F4B => Some("School's Out - Alice Cooper"),
        0x9CBFE5E3 => Some("Sunshine of Your Love - Cream"),
        0xAEE0DC58 => Some("Barracuda - Heart"),
        0xB3AE1B7A => Some("Bulls on Parade - Rage Against the Machine"),
        0x233F951A => Some("Reptilia - The Strokes"),
        0xE8BAF4E5 => Some("When You Were Young - The Killers"),
        0x9AC28451 => Some("Miss Murder - AFI"),
        0x5CA68A94 => Some("The Seeker - The Who"),
        0x24541776 => Some("Lay Down - Priestess"),
        0x614910AE => Some("Paint It, Black - The Rolling Stones"),
        0x4A7D84EB => Some("Suck My Kiss - Red Hot Chili Peppers"),
        0x9F667DE2 => Some("Paranoid - Black Sabbath"),
        0xA86A21FF => Some("Anarchy in the U.K. - Sex Pistols"),
        0x51AD6E8C => Some("Kool Thing - Sonic Youth"),
        0xDDEBC0CE => Some("My Name Is Jonas - Weezer"),
        0x80D135F1 => Some("Even Flow - Pearl Jam"),
        0x2D77BC7D => Some("Cities on Flame with Rock and Roll - Blue Öyster Cult"),
        0xF6A3347D => Some("Holiday in Cambodia - Dead Kennedys"),
        0x2B3B8EC0 => Some("Rock You Like a Hurricane - Scorpions"),
        0x4771DA76 => Some("Same Old Song and Dance - Aerosmith"),
        0x700C4085 => Some("La Grange - ZZ Top"),
        0x3ADA655D => Some("Welcome to the Jungle - Guns N' Roses"),
        0x00D5E836 => Some("Black Magic Woman - Santana"),
        0xA9F17F64 => Some("Black Sunshine - White Zombie"),
        0x54E72043 => Some("The Metal - Tenacious D"),
        0x8EBC7218 => Some("Pride and Joy - Stevie Ray Vaughan"),
        0xE8AB11E2 => Some("Before I Forget - Slipknot"),
        0xAD7C54BA => Some("Stricken - Disturbed"),
        0x5E055837 => Some("Monsters - Matchbook Romance"),
        0x52F25C10 => Some("3's & 7's - Queens of the Stone Age"),
        0x335A5A9F => Some("Knights of Cydonia - Muse"),
        0x23FC5B5F => Some("Cult of Personality - Living Colour"),
        0x1CBD9BE7 => Some("Raining Blood - Slayer"),
        0xCE75E130 => Some("Cliffs of Dover - Eric Johnson"),
        0x4EE17225 => Some("The Number of the Beast - Iron Maiden"),
        0x8593790E => Some("One - Metallica"),
        0x40797ADD => Some("Through the Fire and Flames - DragonForce"),

        // Boss Battles
        0xC27EFFAE => Some("Guitar Battle vs. Tom Morello - Tom Morello"),
        0xF372F551 => Some("Guitar Battle vs. Slash - Slash"),
        0xF2937A7E => Some("The Devil Went Down to Georgia - Steve Ouimette"),

        // Bonus Tracks
        0xC88D7504 => Some("Avalancha - Héroes del Silencio"),
        0x0F590888 => Some("In the Belly of a Shark - Gallows"),
        0xA401F325 => Some("Can't Be Saved - Senses Fail"),
        0x0FEC074A => Some("Closer - Lacuna Coil"),
        0x2FCD2D86 => Some("Don't Hold Back - The Sleeping"),
        0x17221CBD => Some("Down 'N Dirty - LA Slum Lords"),
        0x1821171E => Some("F.C.P.R.E.M.I.X. - The Fall of Troy"),
        0x725509F3 => Some("Generation Rock - Revolverheld"),
        0x1A075876 => Some("Go That Far - Bret Michaels Band"),
        0xC3189706 => Some("Hier Kommt Alex - Die Toten Hosen"),
        0x6A9010DD => Some("I'm in the Band - The Hellacopters"),
        0x15E308F4 => Some("Impulse - An Endless Sporadic"),
        0x310246E3 => Some("In Love - Scouts of St. Sebastian"),
        0x55D68BD3 => Some("Mauvais Garçon - Naast"),
        0x55E6D8D4 => Some("Minus Celsius - Backyard Babies"),
        0x48130F42 => Some("My Curse - Killswitch Engage"),
        0x17C2BF3D => Some("Nothing for Me Here - Dope"),
        0xB7F75EAB => Some("Prayer of the Refugee - Rise Against"),
        0x822B4762 => Some("Radio Song - Superbus"),
        0x33A522F2 => Some("Ruby - Kaiser Chiefs"),
        0x468C655D => Some("She Bangs the Drums - The Stone Roses"),
        0x348C4908 => Some("Take This Life - In Flames"),
        0x2A49D730 => Some("The Way It Ends - Prototype"),
        0x8CF2B87A => Some("Cherub Rock - The Smashing Pumpkins"),

        _ => None,
    }
}

unsafe fn extract_string_from_item(item: *const QbStructItem) -> Option<String> {
    if item.is_null() {
        return None;
    }
    let type_val = (*item).flags >> 1;
    let ptr = (*item).value as *const c_void;
    if !is_readable(ptr, 2) {
        return None;
    }
    match type_val {
        3 => {
            // CString
            let cstr_ptr = ptr as *const u8;
            let mut bytes = Vec::new();
            let mut offset = 0;
            while offset < 128 && is_readable(cstr_ptr.add(offset) as *const c_void, 1) {
                let b = *cstr_ptr.add(offset);
                if b == 0 {
                    break;
                }
                bytes.push(b);
                offset += 1;
            }
            if !bytes.is_empty() {
                Some(String::from_utf8_lossy(&bytes).trim().to_string())
            } else {
                None
            }
        }
        4 => {
            // WString (UTF-16)
            let wstr_ptr = ptr as *const u16;
            let mut chars = Vec::new();
            let mut offset = 0;
            while offset < 128 && is_readable(wstr_ptr.add(offset) as *const c_void, 2) {
                let c = *wstr_ptr.add(offset);
                if c == 0 {
                    break;
                }
                chars.push(c);
                offset += 1;
            }
            if !chars.is_empty() {
                Some(String::from_utf16_lossy(&chars).trim().to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

unsafe fn find_global_node(table: *const *const HashMapNode, key: u32) -> *const HashMapNode {
    let index = (key & 0x7FFF) as usize;
    let mut curr = *table.add(index);
    let mut count = 0;
    while !curr.is_null() && count < 256 {
        if !is_readable(curr as *const c_void, std::mem::size_of::<HashMapNode>()) {
            break;
        }
        if (*curr).key == key {
            return curr;
        }
        curr = (*curr).next_item;
        count += 1;
    }
    std::ptr::null()
}

unsafe fn find_struct_item(st: *const QbStruct, key: u32) -> Option<*const QbStructItem> {
    let mut item = (*st).first;
    let mut count = 0;
    while !item.is_null() && count < 300 {
        if !is_readable(item as *const c_void, std::mem::size_of::<QbStructItem>()) {
            break;
        }

        // If this item is a QbMap, check the map table
        let type_val = (*item).flags >> 1;
        if type_val == 0x1B {
            let map_ptr = (*item).value as *const QbMap;
            if !map_ptr.is_null() && is_readable(map_ptr as *const c_void, std::mem::size_of::<QbMap>()) {
                if let Some(found_in_map) = find_map_item(map_ptr, key) {
                    return Some(found_in_map);
                }
            }
        }

        if (*item).key == key {
            return Some(item);
        }
        item = (*item).next;
        count += 1;
    }
    None
}

unsafe fn find_map_item(map: *const QbMap, key: u32) -> Option<*const QbStructItem> {
    if (*map).nodes.is_null() || (*map).size > 16 {
        return None;
    }
    let mask = (1 << (*map).size) - 1;
    let index = (key & mask) as usize;
    let table_ptr = (*map).nodes;
    if !is_readable(table_ptr.add(index) as *const c_void, std::mem::size_of::<QbMapNode>()) {
        return None;
    }
    let mut node: *const QbMapNode = table_ptr.add(index);
    let mut count = 0;
    while !node.is_null() && count < 64 {
        if !is_readable(node as *const c_void, std::mem::size_of::<QbMapNode>()) {
            break;
        }
        if (*node).key == key {
            let item_ptr = (*node).value as *const QbStructItem;
            if !item_ptr.is_null() && is_readable(item_ptr as *const c_void, std::mem::size_of::<QbStructItem>()) {
                return Some(item_ptr);
            }
        }
        node = (*node).next_node;
        count += 1;
    }
    None
}
