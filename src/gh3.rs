use serde::Serialize;
use std::ffi::c_void;
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_READONLY,
    PAGE_READWRITE, VirtualQuery,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

const fn qb_hash(s: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut crc: u32 = 0xFFFFFFFF;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let ch = match b {
            b'A'..=b'Z' => b + 32,
            b'/' => b'\\',
            _ => b,
        };
        crc = crc32_byte(ch, crc);
        i += 1;
    }
    crc
}

const fn crc32_byte(b: u8, crc: u32) -> u32 {
    let mut c = (crc ^ (b as u32)) & 0xFF;
    let mut i = 0;
    while i < 8 {
        c = if (c & 1) != 0 {
            0xEDB88320 ^ (c >> 1)
        } else {
            c >> 1
        };
        i += 1;
    }
    c ^ (crc >> 8)
}

pub const ADDR_IS_PLAYING_SONG: usize = 0x00B4EB21;
pub const ADDR_SONG_PAUSED: usize = 0x00B4E320;
pub const ADDR_CURRENT_SONG_KEY: usize = 0x00A13314;
pub const ADDR_SONG_: usize = 0x00A1331C;
pub const ADDR_ITEM_MAP: usize = 0x00C03AA8;
pub const ADDR_P1_STATUS_PTR: usize = 0x00A12FCC;
pub const ADDR_SCORE_OBJECT: usize = 0x00A12FD0;

pub const KEY_PLAYER1_STATUS: u32 = qb_hash("player1_status");
pub const KEY_STAR_POWER_USED: u32 = qb_hash("star_power_used");
pub const KEY_STAR_POWER_AMOUNT: u32 = qb_hash("star_power_amount");
pub const KEY_STAR_POWER_USABLE: u32 = qb_hash("star_power_usable");
pub const KEY_CURRENT_RUN: u32 = qb_hash("current_run");
pub const KEY_SCORE: u32 = qb_hash("score");
#[allow(dead_code)]
pub const KEY_NUM_MULTIPLIER: u32 = qb_hash("num_multiplier");
pub const KEY_CURRENT_HEALTH: u32 = qb_hash("current_health");
pub const KEY_TOTAL_NOTES: u32 = qb_hash("total_notes");
pub const KEY_NOTES_HIT: u32 = qb_hash("notes_hit");

pub const KEY_CURRENT_DIFFICULTY: u32 = qb_hash("current_difficulty");
pub const KEY_CURRENT_SONG: u32 = qb_hash("current_song");
pub const KEY_FASTGH3_META: u32 = qb_hash("fastgh3_meta");
pub const KEY_GH3_SONGLIST_PROPS: u32 = qb_hash("gh3_songlist_props");
pub const KEY_PERMANENT_SONGLIST_PROPS: u32 = qb_hash("permanent_songlist_props");
pub const KEY_TITLE: u32 = qb_hash("title");
pub const KEY_ARTIST: u32 = qb_hash("artist");
pub const KEY_AUTHOR: u32 = qb_hash("author");

pub const KEY_EASY: u32 = qb_hash("easy");
pub const KEY_MEDIUM: u32 = qb_hash("medium");
pub const KEY_HARD: u32 = qb_hash("hard");
pub const KEY_EXPERT: u32 = qb_hash("expert");

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
pub struct HashMapNode {
    pub byte0: u8,
    pub byte1: u8,
    pub item_type: u8,
    pub byte3: u8,
    pub key: u32,
    pub dword8: u32,
    pub value: u32,
    pub next_item: u32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
pub struct QbStruct {
    pub unk0: u16,
    pub unk2: u8,
    pub unk3: u8,
    pub first: u32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
pub struct QbStructItem {
    pub unk_flag0: u8,
    pub flags: u8,
    pub word2: u16,
    pub key: u32,
    pub value: u32,
    pub next: u32,
}

impl QbStructItem {
    #[inline]
    pub fn item_type(&self) -> u8 {
        self.flags >> 1
    }
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
pub struct QbMapNode {
    pub key: u32,
    pub value: u32,
    pub next_node: u32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
pub struct QbMap {
    pub size: u32,
    pub nodes: u32,
    pub type_flags: u8,
    pub _pad: [u8; 3],
    pub unk_c: u32,
    pub unk_10: u32,
    pub unk_14: u8,
    pub bool_flag: u8,
    pub keep_alive: u8,
    pub unk_17: u8,
}

impl QbMap {
    #[allow(dead_code)]
    #[inline]
    pub fn item_type(&self) -> u8 {
        self.type_flags >> 1
    }
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

pub unsafe fn is_readable(ptr: *const c_void, size: usize) -> bool {
    let addr = ptr as usize;
    #[cfg(target_pointer_width = "32")]
    let is_user_addr = (0x00010000..=0x7FFF0000).contains(&addr);
    #[cfg(not(target_pointer_width = "32"))]
    let is_user_addr = (0x00010000..=0x7FFF_FFFF_FFFF).contains(&addr);
    if !is_user_addr {
        return false;
    }
    let mut mbi = MEMORY_BASIC_INFORMATION::default();
    let res = unsafe {
        VirtualQuery(
            Some(ptr),
            &mut mbi,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    if res == 0 || mbi.State != MEM_COMMIT {
        return false;
    }
    let protect = mbi.Protect;
    let allowed = PAGE_READONLY | PAGE_READWRITE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE;
    (protect.0 & allowed.0) != 0 && (addr + size <= mbi.BaseAddress as usize + mbi.RegionSize)
}

pub unsafe fn readable_len(ptr: *const c_void, max_len: usize) -> usize {
    let addr = ptr as usize;
    #[cfg(target_pointer_width = "32")]
    let is_user_addr = (0x00010000..=0x7FFF0000).contains(&addr);
    #[cfg(not(target_pointer_width = "32"))]
    let is_user_addr = (0x00010000..=0x7FFF_FFFF_FFFF).contains(&addr);
    if !is_user_addr {
        return 0;
    }
    let mut mbi = MEMORY_BASIC_INFORMATION::default();
    let res = unsafe {
        VirtualQuery(
            Some(ptr),
            &mut mbi,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    if res == 0 || mbi.State != MEM_COMMIT {
        return 0;
    }
    let protect = mbi.Protect;
    let allowed = PAGE_READONLY | PAGE_READWRITE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE;
    if (protect.0 & allowed.0) == 0 {
        return 0;
    }
    let page_end = mbi.BaseAddress as usize + mbi.RegionSize;
    if addr >= page_end {
        return 0;
    }
    (page_end - addr).min(max_len)
}

/// Safely read a value of type `T` from `addr` using zerocopy after validating memory readability.
pub unsafe fn read_val<T: FromBytes>(addr: usize) -> Option<T> {
    let size = std::mem::size_of::<T>();
    if !unsafe { is_readable(addr as *const c_void, size) } {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(addr as *const u8, size) };
    T::read_from_bytes(slice).ok()
}

/// Safely read a zero-copy reference to `T` from `addr` using zerocopy if readable and aligned.
#[allow(dead_code)]
pub unsafe fn read_ref<'a, T: FromBytes + KnownLayout + Immutable>(addr: usize) -> Option<&'a T> {
    let size = std::mem::size_of::<T>();
    if !unsafe { is_readable(addr as *const c_void, size) } {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(addr as *const u8, size) };
    T::ref_from_bytes(slice).ok()
}

/// Safely read a zero-copy slice of `T` from `addr` using zerocopy if readable and aligned.
pub unsafe fn read_slice<'a, T: FromBytes + KnownLayout + Immutable>(
    addr: usize,
    count: usize,
) -> Option<&'a [T]> {
    let size = std::mem::size_of::<T>() * count;
    if !unsafe { is_readable(addr as *const c_void, size) } {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(addr as *const u8, size) };
    <[T]>::ref_from_bytes(slice).ok()
}

pub unsafe fn read_game_state() -> GameState {
    unsafe {
        let mut state = GameState::default();

        // 1. Check if game is currently playing a song
        if let Some(val) = read_val::<u8>(ADDR_IS_PLAYING_SONG) {
            state.is_playing = val != 0;
        }

        // 2. Check if song is paused (only valid while playing)
        if state.is_playing
            && let Some(val) = read_val::<u8>(ADDR_SONG_PAUSED)
        {
            state.is_paused = val != 0;
        }

        // 3. Locate GlobalMap table
        let mut map_table_addr: usize = 0;
        if let Some(tbl) = read_val::<u32>(ADDR_ITEM_MAP) {
            let tbl_addr = tbl as usize;
            if read_slice::<u32>(tbl_addr, 0x8000).is_some() {
                map_table_addr = tbl_addr;
            }
        }

        // 4. Read current difficulty if GlobalMap is valid
        if map_table_addr != 0
            && let Some(diff_node) = find_global_node(map_table_addr, KEY_CURRENT_DIFFICULTY)
        {
            state.difficulty = resolve_difficulty(diff_node.value).to_string();
        }

        // 5. Read song info if playing
        if state.is_playing {
            state.current_song =
                read_song_info(map_table_addr).unwrap_or_else(|| "Playing".to_string());
        } else {
            state.current_song = "Idle".to_string();
        }

        // 6. Locate player1_status struct address
        let mut p1_struct_addr: usize = 0;

        // Try ADDR_P1_STATUS_PTR (0x00A12FCC) first
        if let Some(direct_ptr) = read_val::<u32>(ADDR_P1_STATUS_PTR) {
            let direct_addr = direct_ptr as usize;
            if direct_addr != 0 && read_val::<QbStruct>(direct_addr).is_some() {
                p1_struct_addr = direct_addr;
            }
        }

        // Fall back to GlobalMap lookup
        if p1_struct_addr == 0
            && map_table_addr != 0
            && let Some(node) = find_global_node(map_table_addr, KEY_PLAYER1_STATUS)
        {
            let ptr_addr = node.value as usize;
            if ptr_addr != 0 && read_val::<QbStruct>(ptr_addr).is_some() {
                p1_struct_addr = ptr_addr;
            }
        }

        if p1_struct_addr == 0 {
            return state;
        }

        // 7. Extract player1_status fields
        if let Some(item) = find_struct_item(p1_struct_addr, KEY_STAR_POWER_USED) {
            state.star_power_active = item.value != 0;
        }

        if let Some(item) = find_struct_item(p1_struct_addr, KEY_STAR_POWER_AMOUNT) {
            let type_val = item.item_type();
            state.star_power_gauge = if type_val == 2 {
                f32::from_bits(item.value).clamp(0.0, 100.0)
            } else if type_val == 1 {
                (item.value as f32).clamp(0.0, 100.0)
            } else {
                0.0
            };
        }

        if let Some(item) = find_struct_item(p1_struct_addr, KEY_STAR_POWER_USABLE) {
            state.star_power_usable = item.value != 0;
        }

        if let Some(item) = find_struct_item(p1_struct_addr, KEY_CURRENT_RUN) {
            state.streak = item.value;
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

        if let Some(item) = find_struct_item(p1_struct_addr, KEY_SCORE) {
            let type_val = item.item_type();
            state.score = if type_val == 2 {
                f32::from_bits(item.value).max(0.0) as u32
            } else {
                item.value
            };
        }

        // If score was not in p1_struct, check ADDR_SCORE_OBJECT
        if state.score == 0
            && let Some(score_obj_ptr) = read_val::<u32>(ADDR_SCORE_OBJECT)
        {
            let score_obj_addr = score_obj_ptr as usize;
            if score_obj_addr != 0
                && read_val::<QbStruct>(score_obj_addr).is_some()
                && let Some(item) = find_struct_item(score_obj_addr, KEY_SCORE)
            {
                let type_val = item.item_type();
                state.score = if type_val == 2 {
                    f32::from_bits(item.value).max(0.0) as u32
                } else {
                    item.value
                };
            }
        }

        if let Some(item) = find_struct_item(p1_struct_addr, KEY_CURRENT_HEALTH) {
            let type_val = item.item_type();
            state.rock_meter = if type_val == 2 {
                f32::from_bits(item.value)
            } else {
                item.value as f32
            };
        }

        if let Some(item) = find_struct_item(p1_struct_addr, KEY_NOTES_HIT) {
            state.notes_hit = item.value;
        }

        if let Some(item) = find_struct_item(p1_struct_addr, KEY_TOTAL_NOTES) {
            state.total_notes = item.value;
        }

        state.notes_missed = state.total_notes.saturating_sub(state.notes_hit);

        state
    }
}

pub fn resolve_difficulty(diff_key: u32) -> &'static str {
    match diff_key {
        KEY_EASY => "Easy",
        KEY_MEDIUM => "Medium",
        KEY_HARD => "Hard",
        KEY_EXPERT => "Expert",
        _ => "Expert",
    }
}

pub unsafe fn read_song_info(map_table_addr: usize) -> Option<String> {
    unsafe {
        // 1. Check fastgh3_meta struct (FastGH3 custom songs)
        if map_table_addr != 0
            && let Some(meta_node) = find_global_node(map_table_addr, KEY_FASTGH3_META)
        {
            let meta_struct_addr = meta_node.value as usize;
            if meta_struct_addr != 0 && read_val::<QbStruct>(meta_struct_addr).is_some() {
                let title = find_struct_item(meta_struct_addr, KEY_TITLE)
                    .and_then(|it| extract_string_from_item(&it));
                let author = find_struct_item(meta_struct_addr, KEY_AUTHOR)
                    .or_else(|| find_struct_item(meta_struct_addr, KEY_ARTIST))
                    .and_then(|it| extract_string_from_item(&it));
                if let Some(t) = title
                    && !t.is_empty()
                {
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

        // 2. Read native active song QbKey
        let mut song_key: u32 = 0;
        if let Some(k) = read_val::<u32>(ADDR_CURRENT_SONG_KEY) {
            song_key = k;
        }
        if song_key == 0
            && let Some(k) = read_val::<u32>(ADDR_SONG_)
        {
            song_key = k;
        }
        if song_key == 0
            && map_table_addr != 0
            && let Some(song_node) = find_global_node(map_table_addr, KEY_CURRENT_SONG)
        {
            song_key = song_node.value;
        }

        if song_key == 0 {
            return None;
        }

        // 3. Check known song lookup table (canonical Neversoft GH3 tracklist)
        if let Some(name) = known_song_name(song_key) {
            return Some(name.to_string());
        }

        // 4. Check gh3_songlist_props & permanent_songlist_props in GlobalMap
        if map_table_addr != 0 {
            for &props_key in &[KEY_GH3_SONGLIST_PROPS, KEY_PERMANENT_SONGLIST_PROPS] {
                if let Some(props_node) = find_global_node(map_table_addr, props_key) {
                    let props_struct_addr = props_node.value as usize;
                    if props_struct_addr != 0
                        && read_val::<QbStruct>(props_struct_addr).is_some()
                        && let Some(song_item) = find_struct_item(props_struct_addr, song_key)
                    {
                        let song_props_addr = song_item.value as usize;
                        if song_props_addr != 0 && read_val::<QbStruct>(song_props_addr).is_some() {
                            let title = find_struct_item(song_props_addr, KEY_TITLE)
                                .and_then(|it| extract_string_from_item(&it));
                            let artist = find_struct_item(song_props_addr, KEY_ARTIST)
                                .and_then(|it| extract_string_from_item(&it));
                            if let Some(t) = title
                                && !t.is_empty()
                            {
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

        // 5. Fallback for unmapped custom song
        Some(format!("Song 0x{:08X}", song_key))
    }
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

pub unsafe fn extract_string_from_item(item: &QbStructItem) -> Option<String> {
    unsafe {
        let type_val = item.item_type();
        let addr = item.value as usize;
        let len = readable_len(addr as *const c_void, 256);
        if len == 0 {
            return None;
        }
        match type_val {
            3 => {
                // CString (1-byte characters)
                let bytes = read_slice::<u8>(addr, len)?;
                let nul_pos = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                if nul_pos == 0 {
                    return None;
                }
                Some(
                    String::from_utf8_lossy(&bytes[..nul_pos])
                        .trim()
                        .to_string(),
                )
            }
            4 => {
                // WString (UTF-16, 2-byte characters)
                let u16_len = len / 2;
                if u16_len == 0 {
                    return None;
                }
                let words = read_slice::<u16>(addr, u16_len)?;
                let nul_pos = words.iter().position(|&w| w == 0).unwrap_or(words.len());
                if nul_pos == 0 {
                    return None;
                }
                Some(
                    String::from_utf16_lossy(&words[..nul_pos])
                        .trim()
                        .to_string(),
                )
            }
            _ => None,
        }
    }
}

pub unsafe fn find_global_node(table_addr: usize, key: u32) -> Option<HashMapNode> {
    unsafe {
        let buckets = read_slice::<u32>(table_addr, 0x8000)?;
        let index = (key & 0x7FFF) as usize;
        let mut curr_addr = buckets[index] as usize;
        let mut count = 0;
        while curr_addr != 0 && count < 256 {
            let node = read_ref::<HashMapNode>(curr_addr)?;
            if node.key == key {
                return Some(*node);
            }
            curr_addr = node.next_item as usize;
            count += 1;
        }
        None
    }
}

pub unsafe fn find_struct_item(st_addr: usize, key: u32) -> Option<QbStructItem> {
    unsafe {
        let st = read_ref::<QbStruct>(st_addr)?;
        let mut item_addr = st.first as usize;
        let mut count = 0;
        while item_addr != 0 && count < 300 {
            let item = read_ref::<QbStructItem>(item_addr)?;

            // If this item is a QbMap, check the map table
            let type_val = item.item_type();
            if type_val == 0x1B {
                let map_addr = item.value as usize;
                if map_addr != 0
                    && let Some(found_in_map) = find_map_item(map_addr, key)
                {
                    return Some(found_in_map);
                }
            }

            if item.key == key {
                return Some(*item);
            }
            item_addr = item.next as usize;
            count += 1;
        }
        None
    }
}

pub unsafe fn find_map_item(map_addr: usize, key: u32) -> Option<QbStructItem> {
    unsafe {
        let map = read_ref::<QbMap>(map_addr)?;
        if map.nodes == 0 || map.size > 16 {
            return None;
        }
        let len = 1 << map.size;
        let nodes_slice = read_slice::<QbMapNode>(map.nodes as usize, len)?;
        let mask = len - 1;
        let index = (key as usize) & mask;
        let mut node = nodes_slice[index];
        let mut count = 0;
        loop {
            if node.key == key {
                let item_addr = node.value as usize;
                if item_addr != 0
                    && let Some(item) = read_ref::<QbStructItem>(item_addr)
                {
                    return Some(*item);
                }
            }
            if node.next_node == 0 || count >= 64 {
                break;
            }
            if let Some(next) = read_ref::<QbMapNode>(node.next_node as usize) {
                node = *next;
                count += 1;
            } else {
                break;
            }
        }
        None
    }
}
