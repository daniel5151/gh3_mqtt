use serde::Serialize;
use std::ffi::c_void;
use windows::Win32::System::Memory::MEM_COMMIT;
use windows::Win32::System::Memory::MEMORY_BASIC_INFORMATION;
use windows::Win32::System::Memory::PAGE_EXECUTE_READ;
use windows::Win32::System::Memory::PAGE_EXECUTE_READWRITE;
use windows::Win32::System::Memory::PAGE_READONLY;
use windows::Win32::System::Memory::PAGE_READWRITE;
use windows::Win32::System::Memory::VirtualQuery;
use zerocopy::FromBytes;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

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

const ADDR_IS_PLAYING_SONG: usize = 0x00B4EB21;
const ADDR_SONG_PAUSED: usize = 0x00B4E320;
const ADDR_CURRENT_SONG_KEY: usize = 0x00A13314;
const ADDR_SONG_: usize = 0x00A1331C;
const ADDR_ITEM_MAP: usize = 0x00C03AA8;
const ADDR_P1_STATUS_PTR: usize = 0x00A12FCC;
const ADDR_SCORE_OBJECT: usize = 0x00A12FD0;

const KEY_PLAYER1_STATUS: u32 = qb_hash("player1_status");
const KEY_STAR_POWER_USED: u32 = qb_hash("star_power_used");
const KEY_STAR_POWER_AMOUNT: u32 = qb_hash("star_power_amount");
const KEY_STAR_POWER_USABLE: u32 = qb_hash("star_power_usable");
const KEY_CURRENT_RUN: u32 = qb_hash("current_run");
const KEY_SCORE: u32 = qb_hash("score");
#[expect(dead_code)]
const KEY_NUM_MULTIPLIER: u32 = qb_hash("num_multiplier");
const KEY_CURRENT_HEALTH: u32 = qb_hash("current_health");
const KEY_TOTAL_NOTES: u32 = qb_hash("total_notes");
const KEY_NOTES_HIT: u32 = qb_hash("notes_hit");

const KEY_CURRENT_DIFFICULTY: u32 = qb_hash("current_difficulty");
const KEY_CURRENT_SONG: u32 = qb_hash("current_song");
const KEY_FASTGH3_META: u32 = qb_hash("fastgh3_meta");
const KEY_GH3_SONGLIST_PROPS: u32 = qb_hash("gh3_songlist_props");
const KEY_PERMANENT_SONGLIST_PROPS: u32 = qb_hash("permanent_songlist_props");
const KEY_TITLE: u32 = qb_hash("title");
const KEY_ARTIST: u32 = qb_hash("artist");
const KEY_AUTHOR: u32 = qb_hash("author");

const KEY_EASY: u32 = qb_hash("easy");
const KEY_MEDIUM: u32 = qb_hash("medium");
const KEY_HARD: u32 = qb_hash("hard");
const KEY_EXPERT: u32 = qb_hash("expert");

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
struct HashMapNode {
    byte0: u8,
    byte1: u8,
    item_type: u8,
    byte3: u8,
    key: u32,
    dword8: u32,
    value: u32,
    next_item: u32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
struct QbStruct {
    unk0: u16,
    unk2: u8,
    unk3: u8,
    first: u32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
struct QbStructItem {
    unk_flag0: u8,
    flags: u8,
    word2: u16,
    key: u32,
    value: u32,
    next: u32,
}

impl QbStructItem {
    #[inline]
    fn item_type(&self) -> u8 {
        self.flags >> 1
    }
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
struct QbMapNode {
    key: u32,
    value: u32,
    next_node: u32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
struct QbMap {
    size: u32,
    nodes: u32,
    type_flags: u8,
    _pad: [u8; 3],
    unk_c: u32,
    unk_10: u32,
    unk_14: u8,
    bool_flag: u8,
    keep_alive: u8,
    unk_17: u8,
}

impl QbMap {
    #[expect(dead_code)]
    #[inline]
    fn item_type(&self) -> u8 {
        self.type_flags >> 1
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GameState {
    pub is_playing: bool,
    pub is_paused: bool,
    pub star_power_active: bool,
    pub star_power_gauge: f32,
    star_power_usable: bool,
    pub streak: u32,
    multiplier: u32,
    pub effective_multiplier: u32,
    pub score: u32,
    rock_meter: f32,
    pub notes_hit: u32,
    total_notes: u32,
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

/// A safe accessor for reading process memory within the Guitar Hero III
/// address space.
#[derive(Clone, Copy, Debug)]
pub struct MemoryView {
    _private: (),
}

impl MemoryView {
    /// Constructs a `MemoryView` for the current process.
    ///
    /// # Safety
    ///
    /// The caller must ensure that this is executed within the address space of
    /// a running Guitar Hero III process.
    pub const unsafe fn new() -> Self {
        Self { _private: () }
    }

    /// Checks if the memory range `[ptr, ptr + size)` is currently committed
    /// and readable in user space.
    fn is_readable(&self, ptr: *const c_void, size: usize) -> bool {
        let addr = ptr as usize;
        #[cfg(target_pointer_width = "32")]
        let is_user_addr = (0x00010000..=0x7FFF0000).contains(&addr);
        #[cfg(not(target_pointer_width = "32"))]
        let is_user_addr = (0x00010000..=0x7FFF_FFFF_FFFF).contains(&addr);
        if !is_user_addr {
            return false;
        }
        let mut mbi = MEMORY_BASIC_INFORMATION::default();
        // SAFETY: Querying memory page attributes via VirtualQuery with a
        // stack-allocated buffer.
        let res =
            unsafe { VirtualQuery(Some(ptr), &mut mbi, size_of::<MEMORY_BASIC_INFORMATION>()) };
        if res == 0 || mbi.State != MEM_COMMIT {
            return false;
        }
        let protect = mbi.Protect;
        let allowed = PAGE_READONLY | PAGE_READWRITE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE;
        (protect.0 & allowed.0) != 0 && (addr + size <= mbi.BaseAddress as usize + mbi.RegionSize)
    }

    /// Returns the number of readable bytes (up to `max_len`) available
    /// starting at `ptr`.
    fn readable_len(&self, ptr: *const c_void, max_len: usize) -> usize {
        let addr = ptr as usize;
        #[cfg(target_pointer_width = "32")]
        let is_user_addr = (0x00010000..=0x7FFF0000).contains(&addr);
        #[cfg(not(target_pointer_width = "32"))]
        let is_user_addr = (0x00010000..=0x7FFF_FFFF_FFFF).contains(&addr);
        if !is_user_addr {
            return 0;
        }
        let mut mbi = MEMORY_BASIC_INFORMATION::default();
        // SAFETY: Querying memory page attributes via VirtualQuery with a
        // stack-allocated buffer.
        let res =
            unsafe { VirtualQuery(Some(ptr), &mut mbi, size_of::<MEMORY_BASIC_INFORMATION>()) };
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

    /// Safely copies a value of type `T` from `addr` using zerocopy if
    /// readable.
    fn read_val<T: FromBytes>(&self, addr: usize) -> Option<T> {
        let size = size_of::<T>();
        if !self.is_readable(addr as *const c_void, size) {
            return None;
        }
        // SAFETY: is_readable verified that the address range is committed and
        // readable in user space.
        let slice = unsafe { std::slice::from_raw_parts(addr as *const u8, size) };
        T::read_from_bytes(slice).ok()
    }

    /// Extracts a null-terminated string (CString or UTF-16 WString) from a
    /// `QbStructItem`.
    fn extract_string_from_item(&self, item: &QbStructItem) -> Option<String> {
        let type_val = item.item_type();
        let addr = item.value as usize;
        let len = self.readable_len(addr as *const c_void, 256);
        if len == 0 {
            return None;
        }
        match type_val {
            3 => {
                // CString (1-byte characters)
                // SAFETY: addr was confirmed readable for len bytes by
                // readable_len.
                let slice = unsafe { std::slice::from_raw_parts(addr as *const u8, len) };
                let nul_pos = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
                if nul_pos == 0 {
                    return None;
                }
                Some(
                    String::from_utf8_lossy(&slice[..nul_pos])
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
                // SAFETY: addr was confirmed readable for at least u16_len * 2
                // bytes by readable_len.
                let slice = unsafe { std::slice::from_raw_parts(addr as *const u8, u16_len * 2) };
                let words: Vec<u16> = slice
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|chunk| u16::from_le_bytes(*chunk))
                    .take_while(|&w| w != 0)
                    .collect();
                if words.is_empty() {
                    return None;
                }
                Some(String::from_utf16_lossy(&words).trim().to_string())
            }
            _ => None,
        }
    }
}

/// A safe view into a Neversoft `QbStruct` linked list in engine memory.
struct QbStructView<'a> {
    mem: &'a MemoryView,
    addr: usize,
}

impl<'a> QbStructView<'a> {
    /// Attempts to wrap a `QbStruct` at `addr` after validating its
    /// readability.
    fn new(mem: &'a MemoryView, addr: usize) -> Option<Self> {
        if addr != 0 && mem.read_val::<QbStruct>(addr).is_some() {
            Some(Self { mem, addr })
        } else {
            None
        }
    }

    /// Finds a struct item matching `key` by traversing the linked list (and
    /// any nested `QbMap`).
    fn get_item(&self, key: u32) -> Option<QbStructItem> {
        let st = self.mem.read_val::<QbStruct>(self.addr)?;
        let mut item_addr = st.first as usize;
        let mut count = 0;
        while item_addr != 0 && count < 300 {
            let item = self.mem.read_val::<QbStructItem>(item_addr)?;

            // If this item is a QbMap (type 0x1B), search the nested map table
            if item.item_type() == 0x1B {
                let map_addr = item.value as usize;
                if map_addr != 0
                    && let Some(found) = self.find_in_map(map_addr, key)
                {
                    return Some(found);
                }
            }

            if item.key == key {
                return Some(item);
            }
            item_addr = item.next as usize;
            count += 1;
        }
        None
    }

    fn find_in_map(&self, map_addr: usize, key: u32) -> Option<QbStructItem> {
        let map = self.mem.read_val::<QbMap>(map_addr)?;
        if map.nodes == 0 || map.size > 16 {
            return None;
        }
        let len = 1 << map.size;
        let mask = len - 1;
        let index = (key as usize) & mask;
        let node_size = size_of::<QbMapNode>();
        let node_addr = (map.nodes as usize) + index * node_size;
        let mut node = self.mem.read_val::<QbMapNode>(node_addr)?;
        let mut count = 0;
        loop {
            if node.key == key {
                let item_addr = node.value as usize;
                if item_addr != 0
                    && let Some(item) = self.mem.read_val::<QbStructItem>(item_addr)
                {
                    return Some(item);
                }
            }
            if node.next_node == 0 || count >= 64 {
                break;
            }
            if let Some(next) = self.mem.read_val::<QbMapNode>(node.next_node as usize) {
                node = next;
                count += 1;
            } else {
                break;
            }
        }
        None
    }

    /// Reads a `u32` value for `key`.
    fn get_u32(&self, key: u32) -> Option<u32> {
        self.get_item(key).map(|item| item.value)
    }

    /// Reads a boolean value for `key`.
    fn get_bool(&self, key: u32) -> Option<bool> {
        self.get_item(key).map(|item| item.value != 0)
    }

    /// Reads an `f32` (or integer converted to `f32`) for `key`.
    fn get_f32(&self, key: u32) -> Option<f32> {
        self.get_item(key).map(|item| match item.item_type() {
            2 => f32::from_bits(item.value),
            1 => item.value as f32,
            _ => 0.0,
        })
    }

    /// Reads a string value for `key`.
    fn get_string(&self, key: u32) -> Option<String> {
        self.get_item(key)
            .and_then(|item| self.mem.extract_string_from_item(&item))
    }

    /// Resolves a nested `QbStruct` for `key`.
    fn get_struct(&self, key: u32) -> Option<QbStructView<'a>> {
        self.get_item(key)
            .and_then(|item| QbStructView::new(self.mem, item.value as usize))
    }

    /// Resolves score value, handling both integer and float representation.
    fn get_score(&self) -> Option<u32> {
        self.get_item(KEY_SCORE).map(|item| {
            if item.item_type() == 2 {
                f32::from_bits(item.value).max(0.0) as u32
            } else {
                item.value
            }
        })
    }
}

/// A safe view into the Neversoft GlobalMap hash bucket table.
struct GlobalMapView<'a> {
    mem: &'a MemoryView,
    table_addr: usize,
}

impl<'a> GlobalMapView<'a> {
    /// Attempts to locate and validate the GlobalMap bucket table in memory.
    fn locate(mem: &'a MemoryView) -> Option<Self> {
        let tbl_ptr = mem.read_val::<u32>(ADDR_ITEM_MAP)?;
        let table_addr = tbl_ptr as usize;
        if table_addr != 0 && mem.is_readable(table_addr as *const c_void, 0x8000 * 4) {
            Some(Self { mem, table_addr })
        } else {
            None
        }
    }

    /// Finds a `HashMapNode` matching `key` in the GlobalMap buckets.
    fn get_node(&self, key: u32) -> Option<HashMapNode> {
        let index = (key & 0x7FFF) as usize;
        let bucket_addr = self.table_addr + index * 4;
        let mut curr_addr = self.mem.read_val::<u32>(bucket_addr)? as usize;
        let mut count = 0;
        while curr_addr != 0 && count < 256 {
            let node = self.mem.read_val::<HashMapNode>(curr_addr)?;
            if node.key == key {
                return Some(node);
            }
            curr_addr = node.next_item as usize;
            count += 1;
        }
        None
    }

    /// Resolves a `QbStruct` stored in the GlobalMap for `key`.
    fn get_struct(&self, key: u32) -> Option<QbStructView<'a>> {
        let node = self.get_node(key)?;
        QbStructView::new(self.mem, node.value as usize)
    }

    /// Reads a `u32` value stored in the GlobalMap for `key`.
    fn get_u32(&self, key: u32) -> Option<u32> {
        self.get_node(key).map(|node| node.value)
    }
}

/// Reads the current game and player status from Guitar Hero III engine memory.
pub fn read_game_state(mem: &MemoryView) -> GameState {
    let mut state = GameState::default();

    // 1. Check if game is currently playing a song
    if let Some(val) = mem.read_val::<u8>(ADDR_IS_PLAYING_SONG) {
        state.is_playing = val != 0;
    }

    // 2. Check if song is paused (only valid while playing)
    if state.is_playing
        && let Some(val) = mem.read_val::<u8>(ADDR_SONG_PAUSED)
    {
        state.is_paused = val != 0;
    }

    // 3. Locate GlobalMap table
    let global_map = GlobalMapView::locate(mem);

    // 4. Read current difficulty if GlobalMap is valid
    if let Some(map) = &global_map
        && let Some(diff_val) = map.get_u32(KEY_CURRENT_DIFFICULTY)
    {
        state.difficulty = match diff_val {
            KEY_EASY => "Easy",
            KEY_MEDIUM => "Medium",
            KEY_HARD => "Hard",
            KEY_EXPERT => "Expert",
            _ => "Expert",
        }
        .to_string();
    }

    // 5. Read song info if playing
    if state.is_playing {
        state.current_song =
            read_song_info(mem, global_map.as_ref()).unwrap_or_else(|| "Playing".to_string());
    } else {
        state.current_song = "Idle".to_string();
    }

    // 6. Locate player1_status struct view
    let p1_struct = mem
        .read_val::<u32>(ADDR_P1_STATUS_PTR)
        .and_then(|ptr| QbStructView::new(mem, ptr as usize))
        .or_else(|| {
            global_map
                .as_ref()
                .and_then(|map| map.get_struct(KEY_PLAYER1_STATUS))
        });

    let Some(p1) = p1_struct else {
        return state;
    };

    // 7. Extract player1_status fields
    if let Some(val) = p1.get_bool(KEY_STAR_POWER_USED) {
        state.star_power_active = val;
    }

    if let Some(gauge) = p1.get_f32(KEY_STAR_POWER_AMOUNT) {
        state.star_power_gauge = gauge.clamp(0.0, 100.0);
    }

    if let Some(usable) = p1.get_bool(KEY_STAR_POWER_USABLE) {
        state.star_power_usable = usable;
    }

    if let Some(run) = p1.get_u32(KEY_CURRENT_RUN) {
        state.streak = run;
    }

    // Multiplier climbs 1x -> 2x -> 3x -> 4x with streak (0-9, 10-19, 20-29,
    // 30+ notes) Doubles to 2x, 4x, 6x, 8x during Star Power!
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

    // Read score from player1_status, fallback to ADDR_SCORE_OBJECT
    if let Some(score) = p1.get_score() {
        state.score = score;
    }
    if state.score == 0
        && let Some(score_obj_ptr) = mem.read_val::<u32>(ADDR_SCORE_OBJECT)
        && let Some(score_obj) = QbStructView::new(mem, score_obj_ptr as usize)
        && let Some(score) = score_obj.get_score()
    {
        state.score = score;
    }

    if let Some(health) = p1.get_f32(KEY_CURRENT_HEALTH) {
        state.rock_meter = health;
    }

    if let Some(hit) = p1.get_u32(KEY_NOTES_HIT) {
        state.notes_hit = hit;
    }

    if let Some(total) = p1.get_u32(KEY_TOTAL_NOTES) {
        state.total_notes = total;
    }

    state.notes_missed = state.total_notes.saturating_sub(state.notes_hit);

    state
}

/// Resolves the current song title and artist from FastGH3 metadata or songlist
/// properties.
fn read_song_info(mem: &MemoryView, global_map: Option<&GlobalMapView<'_>>) -> Option<String> {
    // 1. Check fastgh3_meta struct (FastGH3 custom songs)
    if let Some(meta) = global_map.and_then(|map| map.get_struct(KEY_FASTGH3_META)) {
        let title = meta.get_string(KEY_TITLE);
        let author = meta
            .get_string(KEY_AUTHOR)
            .or_else(|| meta.get_string(KEY_ARTIST));
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

    // 2. Read native active song QbKey
    let mut song_key = mem.read_val::<u32>(ADDR_CURRENT_SONG_KEY).unwrap_or(0);
    if song_key == 0 {
        song_key = mem.read_val::<u32>(ADDR_SONG_).unwrap_or(0);
    }
    if song_key == 0
        && let Some(map) = global_map
        && let Some(k) = map.get_u32(KEY_CURRENT_SONG)
    {
        song_key = k;
    }

    if song_key == 0 {
        return None;
    }

    // 3. Check known song lookup table (canonical Neversoft GH3 tracklist)
    if let Some(name) = known_song_name(song_key) {
        return Some(name.to_string());
    }

    // 4. Check gh3_songlist_props & permanent_songlist_props in GlobalMap
    if let Some(map) = global_map {
        for &props_key in &[KEY_GH3_SONGLIST_PROPS, KEY_PERMANENT_SONGLIST_PROPS] {
            if let Some(props_struct) = map.get_struct(props_key)
                && let Some(song_props) = props_struct.get_struct(song_key)
            {
                let title = song_props.get_string(KEY_TITLE);
                let artist = song_props.get_string(KEY_ARTIST);
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

    // 5. Fallback for unmapped custom song
    Some(format!("Song 0x{:08X}", song_key))
}

#[rustfmt::skip]
const KNOWN_SONGS: &[(u32, &str)] = &[
    // Career / Setlist (Tier 1-8)
    (qb_hash("slowride"),              "Slow Ride - Foghat"),
    (qb_hash("talkdirtytome"),         "Talk Dirty to Me - Poison"),
    (qb_hash("hitmewithyourbestshot"), "Hit Me with Your Best Shot - Pat Benatar"),
    (qb_hash("storyofmylife"),         "Story of My Life - Social Distortion"),
    (qb_hash("rocknrollallnite"),      "Rock and Roll All Nite - Kiss"),
    (qb_hash("sabotage"),              "Sabotage - Beastie Boys"),
    (qb_hash("mississippiqueen"),      "Mississippi Queen - Mountain"),
    (qb_hash("schoolsout"),            "School's Out - Alice Cooper"),
    (qb_hash("sunshineofyourlove"),    "Sunshine of Your Love - Cream"),
    (qb_hash("barracuda"),             "Barracuda - Heart"),
    (qb_hash("bullsonparade"),         "Bulls on Parade - Rage Against the Machine"),
    (qb_hash("reptilia"),              "Reptilia - The Strokes"),
    (qb_hash("whenyouwereyoung"),      "When You Were Young - The Killers"),
    (qb_hash("missmurder"),            "Miss Murder - AFI"),
    (qb_hash("theseeker"),             "The Seeker - The Who"),
    (qb_hash("laydown"),               "Lay Down - Priestess"),
    (qb_hash("paintitblack"),          "Paint It, Black - The Rolling Stones"),
    (qb_hash("suckmykiss"),            "Suck My Kiss - Red Hot Chili Peppers"),
    (qb_hash("paranoid"),              "Paranoid - Black Sabbath"),
    (qb_hash("anarchyintheuk"),        "Anarchy in the U.K. - Sex Pistols"),
    (qb_hash("koolthing"),             "Kool Thing - Sonic Youth"),
    (qb_hash("mynameisjonas"),         "My Name Is Jonas - Weezer"),
    (qb_hash("evenflow"),              "Even Flow - Pearl Jam"),
    (qb_hash("citiesonflame"),         "Cities on Flame with Rock and Roll - Blue Öyster Cult"),
    (qb_hash("holidayincambodia"),     "Holiday in Cambodia - Dead Kennedys"),
    (qb_hash("rockulikeahurricane"),   "Rock You Like a Hurricane - Scorpions"),
    (qb_hash("sameoldsonganddance"),   "Same Old Song and Dance - Aerosmith"),
    (qb_hash("lagrange"),              "La Grange - ZZ Top"),
    (qb_hash("welcometothejungle"),    "Welcome to the Jungle - Guns N' Roses"),
    (qb_hash("blackmagicwoman"),       "Black Magic Woman - Santana"),
    (qb_hash("blacksunshine"),         "Black Sunshine - White Zombie"),
    (qb_hash("themetal"),              "The Metal - Tenacious D"),
    (qb_hash("pridenjoy"),             "Pride and Joy - Stevie Ray Vaughan"),
    (qb_hash("beforeiforget"),         "Before I Forget - Slipknot"),
    (qb_hash("stricken"),              "Stricken - Disturbed"),
    (qb_hash("monsters"),              "Monsters - Matchbook Romance"),
    (qb_hash("threesandsevens"),       "3's & 7's - Queens of the Stone Age"),
    (qb_hash("knightsofcydonia"),      "Knights of Cydonia - Muse"),
    (qb_hash("cultofpersonality"),     "Cult of Personality - Living Colour"),
    (qb_hash("rainingblood"),          "Raining Blood - Slayer"),
    (qb_hash("cliffsofdover"),         "Cliffs of Dover - Eric Johnson"),
    (qb_hash("numberofthebeast"),      "The Number of the Beast - Iron Maiden"),
    (qb_hash("one"),                   "One - Metallica"),
    (qb_hash("thrufireandflames"),     "Through the Fire and Flames - DragonForce"),

    // Boss Battles
    (qb_hash("bosstom"),               "Guitar Battle vs. Tom Morello - Tom Morello"),
    (qb_hash("bossslash"),             "Guitar Battle vs. Slash - Slash"),
    (qb_hash("bossdevil"),             "The Devil Went Down to Georgia - Steve Ouimette"),

    // Bonus Tracks
    (qb_hash("avalancha"),             "Avalancha - Héroes del Silencio"),
    (qb_hash("bellyofashark"),         "In the Belly of a Shark - Gallows"),
    (qb_hash("cantbesaved"),           "Can't Be Saved - Senses Fail"),
    (qb_hash("closer"),                "Closer - Lacuna Coil"),
    (qb_hash("dontholdback"),          "Don't Hold Back - The Sleeping"),
    (qb_hash("downndirty"),            "Down 'N Dirty - LA Slum Lords"),
    (qb_hash("fcpremix"),              "F.C.P.R.E.M.I.X. - The Fall of Troy"),
    (qb_hash("generationrock"),        "Generation Rock - Revolverheld"),
    (qb_hash("gothatfar"),             "Go That Far - Bret Michaels Band"),
    (qb_hash("hierkommtalex"),         "Hier Kommt Alex - Die Toten Hosen"),
    (qb_hash("imintheband"),           "I'm in the Band - The Hellacopters"),
    (qb_hash("impulse"),               "Impulse - An Endless Sporadic"),
    (qb_hash("inlove"),                "In Love - Scouts of St. Sebastian"),
    (qb_hash("mauvaisgarcon"),         "Mauvais Garçon - Naast"),
    (qb_hash("minuscelsius"),          "Minus Celsius - Backyard Babies"),
    (qb_hash("mycurse"),               "My Curse - Killswitch Engage"),
    (qb_hash("nothingformehere"),      "Nothing for Me Here - Dope"),
    (qb_hash("prayeroftherefugee"),    "Prayer of the Refugee - Rise Against"),
    (qb_hash("radiosong"),             "Radio Song - Superbus"),
    (qb_hash("ruby"),                  "Ruby - Kaiser Chiefs"),
    (qb_hash("shebangsadrum"),         "She Bangs the Drums - The Stone Roses"),
    (qb_hash("takethislife"),          "Take This Life - In Flames"),
    (qb_hash("thewayitends"),          "The Way It Ends - Prototype"),
    (qb_hash("cherubrock"),            "Cherub Rock - The Smashing Pumpkins"),
];

fn known_song_name(key: u32) -> Option<&'static str> {
    KNOWN_SONGS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, title)| *title)
}
