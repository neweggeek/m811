//! M811 packets. Ported 1:1 from m811.py, which was tested on the real mouse.
//! See PROTOCOL.md. Pure functions here, no device access, so tests can check bytes.

pub const VID: u16 = 0x04D9;
pub const PID: u16 = 0xFC6D;
pub const USAGE_PAGE: u16 = 0xFFA0;

/// Side key 1, profile 1. 4 bytes per key.
pub const SIDE_KEY_BASE: u16 = 0x00A2;
/// Slot 0 (0x0478) is David's "full auto" macro. We only ever write slot 1 (0x0540).
pub const MACRO_SLOT: u8 = 1;
/// LED record for profile n (1-5) lives at LED_ADDRS[n-1], 7 bytes.
pub const LED_ADDRS: [u16; 5] = [0x0449, 0x0451, 0x0459, 0x0461, 0x0469];
pub const BRIGHTNESS: u8 = 0x02;
/// DPI code for 100, 200, ... 6200 dpi. Copied from the vendor app's own table
/// (RDCfg.exe, file offset 0xB8BDC, used for model "2913-A" = M811). It matches all
/// 6 values we captured. 100 dpi maps to 0, so we start at 200.
pub const DPI_TABLE: [u8; 62] = [
    0, 4, 6, 9, 11, 13, 15, 18, 20, 22, 24, 27, 29, 31, 33, 36, 38, 40, 43, 45, 47, 49, 52, 54, 56, 58, 61, 63, 65, 67,
    70, 72, 74, 77, 79, 81, 83, 86, 88, 90, 92, 95, 97, 99, 102, 104, 106, 108, 111, 113, 115, 117, 120, 122, 124,
    127, 129, 131, 133, 135, 138, 140,
];
pub const DPI_MIN: u32 = 200;
pub const DPI_MAX: u32 = 6200;
/// Buttons in the window. Any multiple of 100 from DPI_MIN to DPI_MAX works on the CLI.
pub const DPI_PRESETS: [u32; 10] = [400, 800, 1000, 1200, 1600, 2000, 2400, 3200, 4000, 6200];

/// Code for a dpi value, or None if it is not a multiple of 100 in DPI_MIN..=DPI_MAX.
pub fn dpi_code(dpi: u32) -> Option<u8> {
    (dpi % 100 == 0 && (DPI_MIN..=DPI_MAX).contains(&dpi)).then(|| DPI_TABLE[dpi as usize / 100 - 1])
}

pub type Packet = Vec<u8>;

pub fn pkt(b: &[u8]) -> Packet {
    let mut v = b.to_vec();
    v.resize(16, 0);
    v
}

pub fn write(addr: u16, data: &[u8]) -> Packet {
    let mut v = vec![0x02, 0xF3, addr as u8, (addr >> 8) as u8, data.len() as u8, 0, 0, 0];
    v.extend_from_slice(data);
    pkt(&v)
}

pub fn read_req(addr: u16) -> Packet {
    pkt(&[0x02, 0xF2, addr as u8, (addr >> 8) as u8, 8])
}

pub fn enter() -> Packet {
    pkt(&[0x02, 0xF5, 0x00])
}

/// Always send after enter(), or mouse input stays frozen.
pub fn exit() -> Packet {
    pkt(&[0x02, 0xF5, 0x01])
}

fn apply(groups: &[u8]) -> Vec<Packet> {
    groups.iter().map(|&g| pkt(&[0x02, 0xF1, 0x02, g])).collect()
}

fn apply_all() -> Vec<Packet> {
    apply(&[0x04, 0x01, 0x02, 0x08, 0x10])
}

/// enter, writes, apply all, exit. Same shape the vendor app uses.
fn config(writes: Vec<Packet>) -> Vec<Packet> {
    let mut v = vec![enter()];
    v.extend(writes);
    v.extend(apply_all());
    v.push(exit());
    v
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Effect {
    Off,
    Full,
    Breathing,
    Rainbow,
    Flash,
    Wave,
    Trace,
    Reactive,
}

impl Effect {
    pub const ALL: [Effect; 8] = [
        Effect::Off,
        Effect::Full,
        Effect::Breathing,
        Effect::Rainbow,
        Effect::Flash,
        Effect::Wave,
        Effect::Trace,
        Effect::Reactive,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Effect::Off => "Off",
            Effect::Full => "Solid",
            Effect::Breathing => "Breathing",
            Effect::Rainbow => "Rainbow",
            Effect::Flash => "Flash",
            Effect::Wave => "Wave",
            Effect::Trace => "Trace",
            Effect::Reactive => "Reactive",
        }
    }

    /// Word used in the CLI and config files.
    pub fn word(self) -> &'static str {
        match self {
            Effect::Off => "off",
            Effect::Full => "solid",
            Effect::Breathing => "breathing",
            Effect::Rainbow => "rainbow",
            Effect::Flash => "flash",
            Effect::Wave => "wave",
            Effect::Trace => "trace",
            Effect::Reactive => "reactive",
        }
    }

    /// (type, speed, flags). From the vendor capture.
    fn bytes(self) -> (u8, u8, u8) {
        match self {
            Effect::Off => (0x00, 0x00, 0x00),
            Effect::Full => (0x01, 0x00, 0x02),
            Effect::Breathing => (0x01, 0x04, 0x04),
            Effect::Rainbow => (0x01, 0x04, 0x08),
            Effect::Flash => (0x01, 0x04, 0x10),
            Effect::Wave => (0x02, 0x04, 0x00),
            Effect::Trace => (0x06, 0x04, 0x00),
            Effect::Reactive => (0x07, 0x04, 0x00),
        }
    }

    pub fn needs_color(self) -> bool {
        matches!(self, Effect::Full | Effect::Breathing | Effect::Flash)
    }

    pub fn from_record(rec: &[u8; 7]) -> Option<Effect> {
        Effect::ALL.into_iter().find(|e| {
            let (t, _, f) = e.bytes();
            t == rec[3] && f == rec[5]
        })
    }
}

/// LED live preview. `others` = current records of all 5 profiles (only used for the
/// profiles we don't change). profile and active are 1-5. `active` is the profile the
/// mouse is on now: the sequence ends by writing it back, so the mouse stays there.
pub fn led(profile: usize, effect: Effect, rgb: [u8; 3], others: &[[u8; 7]; 5], active: usize) -> Vec<Packet> {
    assert!((1..=5).contains(&profile) && (1..=5).contains(&active));
    let rgb = if effect.needs_color() { rgb } else { [0xFF, 0x00, 0x00] };
    let (t, s, f) = effect.bytes();
    let mut v = vec![write(0x0446, &[0, 0])];
    for (i, &a) in LED_ADDRS.iter().enumerate() {
        if i == profile - 1 {
            v.push(write(a, &[rgb[0], rgb[1], rgb[2], t, s, f, BRIGHTNESS]));
        } else {
            v.push(write(a, &others[i]));
        }
    }
    v.push(pkt(&[0x02, 0xF1, 0x02, 0x04]));
    // Fixed value from a 1-5 profile number. Never copy the raw word back:
    // that left 83 00 in it once. Writing the LED's profile here switched the mouse to it
    // (apply of a full status file ended on profile 5), so write the active one.
    v.push(write(0x002C, &[active as u8 - 1, 0]));
    v
}

/// Exact order from the app. No enter/exit.
pub fn profile(n: usize) -> Vec<Packet> {
    assert!((1..=5).contains(&n));
    let mut v = vec![write(0x002C, &[n as u8 - 1, 0])];
    v.extend(apply(&[0x01, 0x04, 0x01, 0x02, 0x08, 0x10]));
    v
}

/// hz is 500 or 1000. Writes profile 1's word.
pub fn poll(hz: u32) -> Vec<Packet> {
    let code = if hz == 500 { 0x02 } else { 0x01 };
    config(vec![write(0x0032, &[code, 0x00])])
}

/// stage 1-5, dpi see dpi_code. Profile 1.
pub fn dpi(stage: usize, dpi: u32) -> Option<Vec<Packet>> {
    assert!((1..=5).contains(&stage));
    let v = dpi_code(dpi)?;
    Some(config(vec![write(0x0044 + 6 * (stage as u16 - 1), &[0x01, v, 0x00, v, 0x00])]))
}

pub fn side_key_addr(side: usize) -> u16 {
    assert!((1..=8).contains(&side));
    SIDE_KEY_BASE + 4 * (side as u16 - 1)
}

/// True if this 4-byte button entry points at macro slot 0 ("full auto").
pub fn is_slot0_macro(entry: &[u8]) -> bool {
    entry[0] == 0x91 && entry[1] & 0x7F == 0
}

/// Button entry for a preset word ("copy", "volup", "dpi+"), a key ("a", "f5") or a
/// combo ("ctrl+shift+t"). See buttons.rs. Never a macro.
pub fn key_entry(key: &str) -> Option<[u8; 4]> {
    crate::buttons::entry(key)
}

fn hid_usage(k: &str) -> Option<u8> {
    let mut chars = k.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    match c {
        'a'..='z' => Some(0x04 + (c as u8 - b'a')),
        '1'..='9' => Some(0x1E + (c as u8 - b'1')),
        '0' => Some(0x27),
        _ => None,
    }
}

/// Side key remap. Caller must check the key does not hold the slot 0 macro.
pub fn key(side: usize, entry: [u8; 4]) -> Vec<Packet> {
    config(vec![write(side_key_addr(side), &entry)])
}

/// Macro spec like "a,50,b": letters are key taps, numbers are ms delays.
/// Writes slot 1 and binds it to `side`. Caller must check the side key is not slot 0.
pub fn macro_(side: usize, spec: &str) -> Result<Vec<Packet>, String> {
    let mut events = vec![];
    for tok in spec.split(',').map(|t| t.trim().to_lowercase()) {
        if !tok.is_empty() && tok.chars().all(|c| c.is_ascii_digit()) {
            let ms: u32 = tok.parse().map_err(|_| format!("bad number {tok}"))?;
            events.extend([0x06, (ms / 10).min(255) as u8, 0x00]);
        } else if let Some(u) = hid_usage(&tok).filter(|_| tok.chars().all(|c| c.is_ascii_lowercase())) {
            events.extend([0x84, u, 0x00, 0x04, u, 0x00]);
        } else {
            return Err(format!("'{tok}': use letters and millisecond numbers, like a,50,b"));
        }
    }
    if events.len() > 200 {
        return Err("macro too long for one slot".into());
    }
    events.resize(200, 0);
    let mut report = vec![0x04, 0xF3, 0x40, 0x05, 0xC8, 0, 0, 0];
    report.extend(events);
    report.resize(256, 0);
    // Same order as m811.py (tested): bind, apply, slot data, exit.
    let mut v = vec![enter(), write(side_key_addr(side), &[0x91, MACRO_SLOT, 0x01, 0x00])];
    v.extend(apply_all());
    v.push(report);
    v.push(exit());
    Ok(v)
}

/// Human text for a 4-byte button entry.
pub fn describe_entry(e: &[u8; 4]) -> String {
    match e {
        _ if crate::buttons::word(e).is_some() => crate::buttons::word(e).unwrap(),
        [0x90, 0x00, u, 0x00] => format!("key 0x{u:02x}"),
        [0x99, ..] => format!("fire key {}", hex4(e)),
        [0x9B, ..] => format!("sniper {}", hex4(e)),
        [0x91, s, ..] if is_slot0_macro(e) => format!("macro slot 0 (full auto){}", if s & 0x80 != 0 { ", repeat" } else { "" }),
        [0x91, s, ..] => format!("macro slot {}{}", s & 0x7F, if s & 0x80 != 0 { ", repeat" } else { "" }),
        _ => hex4(e),
    }
}

fn hex4(e: &[u8; 4]) -> String {
    e.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

pub fn dpi_from_code(code: u8) -> Option<u32> {
    let i = DPI_TABLE.iter().position(|&c| c == code && c != 0)?;
    Some((i as u32 + 1) * 100)
}

/// Lift-off distance, 1 or 2. One byte at 0x002A (vendor fn 0x458E20).
/// Tested 2026-11-02 with this exact sequence: reads back 02 00, mouse fine.
/// (The vendor app applies only group 08 here; full apply is what we tested.)
pub fn lift(n: u8) -> Vec<Packet> {
    assert!((1..=2).contains(&n));
    config(vec![write(0x002A, &[n, 0x00])])
}

pub const SCROLL_MAX: u8 = 10;

/// Scroll speed for profile 1. `current` = the 10 bytes at 0x0020 (one word per profile),
/// so the other profiles keep their value. Vendor app sends all 5 in one 64-byte report 3
/// (fn SendScrollSpeed 0x458580). Tested 2026-11-02: reads back, other profiles kept.
/// Refuses if any profile word is not 1-SCROLL_MAX with a zero high byte, so odd state
/// is never copied back (the same trap as the profile word in led()).
pub fn scroll(n: u8, current: &[u8; 10]) -> Result<Vec<Packet>, String> {
    if !(1..=SCROLL_MAX).contains(&n) {
        return Err(format!("scroll must be 1-{SCROLL_MAX}, got {n}"));
    }
    if let Some(i) = (0..5).find(|&i| !(1..=SCROLL_MAX).contains(&current[2 * i]) || current[2 * i + 1] != 0) {
        return Err(format!(
            "profile {} scroll word reads {:02x} {:02x}, expected 01-{SCROLL_MAX:02x} 00. Not writing.",
            i + 1,
            current[2 * i],
            current[2 * i + 1]
        ));
    }
    let mut words = *current;
    words[0] = n;
    words[1] = 0;
    let mut report = vec![0x03, 0xF3, 0x20, 0x00, 0x0A, 0, 0, 0];
    report.extend(words);
    report.resize(64, 0);
    Ok(config(vec![report]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAVE_RECORD: [u8; 7] = [0xFF, 0x00, 0x00, 0x02, 0x04, 0x00, 0x02];

    fn hex(ps: &[Packet]) -> Vec<String> {
        ps.iter()
            .map(|p| p.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "))
            .collect()
    }

    const APPLY: [&str; 5] = [
        "02 f1 02 04 00 00 00 00 00 00 00 00 00 00 00 00",
        "02 f1 02 01 00 00 00 00 00 00 00 00 00 00 00 00",
        "02 f1 02 02 00 00 00 00 00 00 00 00 00 00 00 00",
        "02 f1 02 08 00 00 00 00 00 00 00 00 00 00 00 00",
        "02 f1 02 10 00 00 00 00 00 00 00 00 00 00 00 00",
    ];
    const ENTER: &str = "02 f5 00 00 00 00 00 00 00 00 00 00 00 00 00 00";
    const EXIT: &str = "02 f5 01 00 00 00 00 00 00 00 00 00 00 00 00 00";

    fn cfg(write: &str) -> Vec<String> {
        let mut v = vec![ENTER.to_string(), write.to_string()];
        v.extend(APPLY.iter().map(|s| s.to_string()));
        v.push(EXIT.into());
        v
    }

    // Expected bytes below are `m811.py ... --dry` output.
    #[test]
    fn led_full_green() {
        let got = hex(&led(1, Effect::Full, [0, 0xFF, 0], &[WAVE_RECORD; 5], 1));
        assert_eq!(
            got,
            [
                "02 f3 46 04 02 00 00 00 00 00 00 00 00 00 00 00",
                "02 f3 49 04 07 00 00 00 00 ff 00 01 00 02 02 00",
                "02 f3 51 04 07 00 00 00 ff 00 00 02 04 00 02 00",
                "02 f3 59 04 07 00 00 00 ff 00 00 02 04 00 02 00",
                "02 f3 61 04 07 00 00 00 ff 00 00 02 04 00 02 00",
                "02 f3 69 04 07 00 00 00 ff 00 00 02 04 00 02 00",
                "02 f1 02 04 00 00 00 00 00 00 00 00 00 00 00 00",
                "02 f3 2c 00 02 00 00 00 00 00 00 00 00 00 00 00",
            ]
        );
    }

    #[test]
    fn led_wave_profile3() {
        let got = hex(&led(3, Effect::Wave, [1, 2, 3], &[WAVE_RECORD; 5], 3));
        assert_eq!(got[3], "02 f3 59 04 07 00 00 00 ff 00 00 02 04 00 02 00");
        assert_eq!(got[7], "02 f3 2c 00 02 00 00 00 02 00 00 00 00 00 00 00");
    }

    #[test]
    fn led_other_profile_keeps_active() {
        // Setting profile 5's lighting while on profile 1 must not switch to 5.
        let got = hex(&led(5, Effect::Wave, [0; 3], &[WAVE_RECORD; 5], 1));
        assert_eq!(got[7], "02 f3 2c 00 02 00 00 00 00 00 00 00 00 00 00 00");
    }

    #[test]
    fn poll_500() {
        assert_eq!(hex(&poll(500)), cfg("02 f3 32 00 02 00 00 00 02 00 00 00 00 00 00 00"));
    }

    #[test]
    fn dpi_2_3000() {
        assert_eq!(hex(&dpi(2, 3000).unwrap()), cfg("02 f3 4a 00 05 00 00 00 01 43 00 43 00 00 00 00"));
        assert!(dpi(1, 1234).is_none());
        assert!(dpi(1, 100).is_none());
        assert!(dpi(1, 6300).is_none());
    }

    #[test]
    fn dpi_table_matches_captures() {
        for (d, c) in [(500, 0x0B), (600, 0x0D), (1000, 0x16), (2000, 0x2D), (3000, 0x43), (6200, 0x8C)] {
            assert_eq!(dpi_code(d), Some(c));
            assert_eq!(dpi_from_code(c), Some(d));
        }
        assert!(DPI_TABLE.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(dpi_from_code(0), None);
    }

    #[test]
    fn lift_2() {
        assert_eq!(hex(&lift(2)), cfg("02 f3 2a 00 02 00 00 00 02 00 00 00 00 00 00 00"));
    }

    #[test]
    fn scroll_keeps_other_profiles() {
        let p = scroll(3, &[1, 0, 2, 0, 1, 0, 1, 0, 1, 0]).unwrap();
        assert_eq!(p[1].len(), 64);
        assert_eq!(p[1][..18], [0x03, 0xF3, 0x20, 0x00, 0x0A, 0, 0, 0, 3, 0, 2, 0, 1, 0, 1, 0, 1, 0]);
        assert_eq!(hex(&p[2..7]), APPLY);
        assert!(scroll(0, &[1, 0, 1, 0, 1, 0, 1, 0, 1, 0]).is_err());
        assert!(scroll(11, &[1, 0, 1, 0, 1, 0, 1, 0, 1, 0]).is_err());
        assert!(scroll(3, &[1, 0, 1, 0, 0x83, 0, 1, 0, 1, 0]).is_err());
        assert!(scroll(3, &[1, 0, 1, 0, 1, 0, 1, 1, 1, 0]).is_err());
    }

    #[test]
    fn keys() {
        assert_eq!(hex(&key(1, key_entry("a").unwrap())), cfg("02 f3 a2 00 04 00 00 00 90 00 04 00 00 00 00 00"));
        assert_eq!(hex(&key(3, key_entry("volup").unwrap())), cfg("02 f3 aa 00 04 00 00 00 8e 01 e9 00 00 00 00 00"));
        assert_eq!(hex(&key(2, key_entry("0").unwrap())), cfg("02 f3 a6 00 04 00 00 00 90 00 27 00 00 00 00 00"));
        assert!(key_entry("ab").is_none());
        assert_eq!(hex(&key(4, key_entry("copy").unwrap())), cfg("02 f3 ae 00 04 00 00 00 8f 01 06 00 00 00 00 00"));
        assert_eq!(describe_entry(&[0x8F, 0x01, 0x06, 0x00]), "copy");
        assert_eq!(describe_entry(&[0x91, 0x00, 0x01, 0x00]), "macro slot 0 (full auto)");
    }

    #[test]
    fn profile_2() {
        let got = hex(&profile(2));
        assert_eq!(got[0], "02 f3 2c 00 02 00 00 00 01 00 00 00 00 00 00 00");
        assert_eq!(got[1], "02 f1 02 01 00 00 00 00 00 00 00 00 00 00 00 00");
        assert_eq!(got[2..], APPLY);
    }

    #[test]
    fn macro_ab() {
        let p = macro_(2, "a,50,b").unwrap();
        let got = hex(&p);
        assert_eq!(got[1], "02 f3 a6 00 04 00 00 00 91 01 01 00 00 00 00 00");
        assert_eq!(got[2..7], APPLY);
        assert_eq!(p[7].len(), 256);
        assert_eq!(p[7][..23], [0x04, 0xf3, 0x40, 0x05, 0xc8, 0, 0, 0, 0x84, 0x04, 0, 0x04, 0x04, 0, 0x06, 0x05, 0, 0x84, 0x05, 0, 0x04, 0x05, 0]);
        assert!(p[7][23..].iter().all(|&b| b == 0));
        assert_eq!(got[8], EXIT);
        assert!(macro_(2, "a,!").is_err());
        assert!(macro_(2, "1").is_ok()); // digits = delay
    }

    #[test]
    fn slot0_guard() {
        assert!(is_slot0_macro(&[0x91, 0x00, 0x01, 0x00]));
        assert!(is_slot0_macro(&[0x91, 0x80, 0xFF, 0xFF]));
        assert!(!is_slot0_macro(&[0x91, 0x81, 0xFF, 0xFF]));
        assert!(!is_slot0_macro(&[0x90, 0x00, 0x04, 0x00]));
    }
}
