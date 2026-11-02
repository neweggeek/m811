//! Button presets: the same function list the vendor app offers in its button menu.
//! Entries come from the vendor app's own table (RDCfg.exe 0x4BB320), names from its
//! menu (same order as the table ids). See PROTOCOL.md "Button function table".

pub struct Preset {
    /// Word used on the command line and in config files.
    pub word: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub entry: [u8; 4],
}

const fn p(word: &'static str, label: &'static str, group: &'static str, entry: [u8; 4]) -> Preset {
    Preset { word, label, group, entry }
}

pub const GROUPS: [&str; 7] = ["Mouse", "Windows", "Browser", "Media", "DPI", "Profile", "Other"];

pub const PRESETS: &[Preset] = &[
    p("left", "Left click", "Mouse", [0x81, 0, 0, 0]),
    p("right", "Right click", "Mouse", [0x82, 0, 0, 0]),
    p("middle", "Middle click", "Mouse", [0x83, 0, 0, 0]),
    p("back", "Back", "Mouse", [0x84, 0, 0, 0]),
    p("forward", "Forward", "Mouse", [0x85, 0, 0, 0]),
    p("scrollup", "Scroll up", "Mouse", [0x8B, 0, 0, 0]),
    p("scrolldown", "Scroll down", "Mouse", [0x8C, 0, 0, 0]),
    p("off", "Button off", "Mouse", [0x00, 0, 0, 0]),
    p("cut", "Cut (Ctrl+X)", "Windows", [0x8F, 0x01, 0x1B, 0]),
    p("copy", "Copy (Ctrl+C)", "Windows", [0x8F, 0x01, 0x06, 0]),
    p("paste", "Paste (Ctrl+V)", "Windows", [0x8F, 0x01, 0x19, 0]),
    p("selectall", "Select all (Ctrl+A)", "Windows", [0x8F, 0x01, 0x04, 0]),
    p("find", "Find (Ctrl+F)", "Windows", [0x8F, 0x01, 0x09, 0]),
    p("new", "New (Ctrl+N)", "Windows", [0x8F, 0x01, 0x11, 0]),
    p("print", "Print (Ctrl+P)", "Windows", [0x8F, 0x01, 0x13, 0]),
    p("save", "Save (Ctrl+S)", "Windows", [0x8F, 0x01, 0x16, 0]),
    p("switchwindow", "Switch window (Alt+Tab)", "Windows", [0x8F, 0x04, 0x2B, 0]),
    p("closewindow", "Close window (Alt+F4)", "Windows", [0x8F, 0x04, 0x3D, 0]),
    p("explorer", "My computer (Win+E)", "Windows", [0x8F, 0x08, 0x08, 0]),
    p("run", "Run (Win+R)", "Windows", [0x8F, 0x08, 0x15, 0]),
    p("desktop", "Show desktop (Win+D)", "Windows", [0x8F, 0x08, 0x07, 0]),
    p("lock", "Lock PC (Win+L)", "Windows", [0x8F, 0x08, 0x0F, 0]),
    p("browserhome", "Browser home", "Browser", [0x8E, 0x01, 0xFF, 0x1F]),
    p("browserback", "Browser back", "Browser", [0x8E, 0x01, 0xFF, 0x20]),
    p("browserforward", "Browser forward", "Browser", [0x8E, 0x01, 0xFF, 0x21]),
    p("browserstop", "Browser stop", "Browser", [0x8E, 0x01, 0xFF, 0x22]),
    p("browserrefresh", "Browser refresh", "Browser", [0x8E, 0x01, 0xFF, 0x23]),
    p("browsersearch", "Browser search", "Browser", [0x8E, 0x01, 0xFF, 0x24]),
    p("favorites", "Browser favorites", "Browser", [0x8E, 0x01, 0xFF, 0x25]),
    p("mail", "Mail", "Browser", [0x8E, 0x01, 0x8A, 0x01]),
    p("playpause", "Play / pause", "Media", [0x8E, 0x01, 0xCD, 0]),
    p("stop", "Stop", "Media", [0x8E, 0x01, 0xB7, 0]),
    p("prev", "Previous track", "Media", [0x8E, 0x01, 0xB6, 0]),
    p("next", "Next track", "Media", [0x8E, 0x01, 0xB5, 0]),
    p("volup", "Volume up", "Media", [0x8E, 0x01, 0xE9, 0]),
    p("voldown", "Volume down", "Media", [0x8E, 0x01, 0xEA, 0]),
    p("mute", "Mute", "Media", [0x8E, 0x01, 0xE2, 0]),
    p("player", "Media player", "Media", [0x8E, 0x01, 0x83, 0x01]),
    p("dpicycle", "DPI switch", "DPI", [0x88, 0, 0, 0]),
    p("dpi+", "DPI +", "DPI", [0x8A, 0, 0, 0]),
    p("dpi-", "DPI -", "DPI", [0x89, 0, 0, 0]),
    p("profilecycle", "Profile switch", "Profile", [0x8D, 0, 0, 0]),
    p("profile+", "Profile +", "Profile", [0x94, 0, 0, 0]),
    p("profile-", "Profile -", "Profile", [0x95, 0, 0, 0]),
    p("rate+", "Polling rate +", "Other", [0x97, 0, 0, 0]),
    p("rate-", "Polling rate -", "Other", [0x98, 0, 0, 0]),
    p("ledswitch", "Lighting on/off", "Other", [0x9A, 0x01, 0x01, 0]),
];

/// Modifier bits, same as HID: ctrl 01, shift 02, alt 04, win 08.
const MODS: [(&str, u8); 4] = [("ctrl", 0x01), ("shift", 0x02), ("alt", 0x04), ("win", 0x08)];

const NAMED: [(&str, u8); 20] = [
    ("enter", 0x28),
    ("esc", 0x29),
    ("backspace", 0x2A),
    ("tab", 0x2B),
    ("space", 0x2C),
    ("minus", 0x2D),
    ("equal", 0x2E),
    ("printscreen", 0x46),
    ("pause", 0x48),
    ("insert", 0x49),
    ("home", 0x4A),
    ("pageup", 0x4B),
    ("delete", 0x4C),
    ("end", 0x4D),
    ("pagedown", 0x4E),
    ("arrowright", 0x4F),
    ("arrowleft", 0x50),
    ("arrowdown", 0x51),
    ("arrowup", 0x52),
    ("capslock", 0x39),
];

/// HID usage for one key name: a-z, 0-9, f1-f12, or a NAMED key.
pub fn usage(k: &str) -> Option<u8> {
    let mut chars = k.chars();
    let c = chars.next()?;
    if chars.as_str().is_empty() {
        return match c {
            'a'..='z' => Some(0x04 + (c as u8 - b'a')),
            '1'..='9' => Some(0x1E + (c as u8 - b'1')),
            '0' => Some(0x27),
            _ => None,
        };
    }
    if let Some(n) = k.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()).filter(|n| (1..=12).contains(n)) {
        return Some(0x3A + n - 1);
    }
    NAMED.iter().find(|(w, _)| *w == k).map(|&(_, u)| u)
}

fn usage_name(u: u8) -> Option<String> {
    match u {
        0x04..=0x1D => Some(((b'a' + u - 0x04) as char).to_string()),
        0x1E..=0x26 => Some(((b'1' + u - 0x1E) as char).to_string()),
        0x27 => Some("0".into()),
        0x3A..=0x45 => Some(format!("f{}", u - 0x3A + 1)),
        _ => NAMED.iter().find(|&&(_, n)| n == u).map(|(w, _)| w.to_string()),
    }
}

/// Entry for a preset word, a key ("a", "f5", "enter") or a combo ("ctrl+shift+t").
pub fn entry(spec: &str) -> Option<[u8; 4]> {
    let s = spec.trim().to_lowercase();
    if let Some(p) = PRESETS.iter().find(|p| p.word == s) {
        return Some(p.entry);
    }
    let mut parts: Vec<&str> = s.split('+').collect();
    let key = usage(parts.pop()?)?;
    let mut mods = 0u8;
    for m in parts {
        let bit = MODS.iter().find(|(w, _)| *w == m)?.1;
        if mods & bit != 0 {
            return None;
        }
        mods |= bit;
    }
    Some(if mods == 0 { [0x90, 0x00, key, 0x00] } else { [0x8F, mods, key, 0x00] })
}

/// Word that `entry` turns back into `e`, if there is one.
pub fn word(e: &[u8; 4]) -> Option<String> {
    if let Some(p) = PRESETS.iter().find(|p| p.entry == *e) {
        return Some(p.word.into());
    }
    match *e {
        [0x90, 0x00, u, 0x00] => usage_name(u),
        [0x8F, m, u, 0x00] if m != 0 && m & !0x0F == 0 => {
            let mut s: String = MODS.iter().filter(|(_, b)| m & b != 0).map(|(w, _)| format!("{w}+")).collect();
            s += &usage_name(u)?;
            Some(s)
        }
        _ => None,
    }
}

pub fn label(e: &[u8; 4]) -> Option<&'static str> {
    PRESETS.iter().find(|p| p.entry == *e).map(|p| p.label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_round_trip() {
        for p in PRESETS {
            assert_eq!(entry(p.word), Some(p.entry), "{}", p.word);
            assert_eq!(word(&p.entry).as_deref(), Some(p.word));
            assert!(GROUPS.contains(&p.group));
            assert_ne!(p.entry[0], 0x91, "presets must never be macros");
        }
        let mut words: Vec<_> = PRESETS.iter().map(|p| p.word).collect();
        words.sort();
        words.dedup();
        assert_eq!(words.len(), PRESETS.len());
    }

    #[test]
    fn keys_and_combos() {
        assert_eq!(entry("a"), Some([0x90, 0, 0x04, 0]));
        assert_eq!(entry("F5"), Some([0x90, 0, 0x3E, 0]));
        assert_eq!(entry("enter"), Some([0x90, 0, 0x28, 0]));
        assert_eq!(entry("ctrl+c"), Some([0x8F, 0x01, 0x06, 0])); // same bytes as "copy"
        assert_eq!(entry("ctrl+shift+t"), Some([0x8F, 0x03, 0x17, 0]));
        assert_eq!(word(&[0x8F, 0x03, 0x17, 0]).as_deref(), Some("ctrl+shift+t"));
        assert_eq!(word(&[0x8F, 0x01, 0x06, 0]).as_deref(), Some("copy"));
        assert_eq!(entry("ctrl+ctrl+c"), None);
        assert_eq!(entry("hyper+c"), None);
        assert_eq!(entry("f13"), None);
        assert_eq!(entry("ab"), None);
        assert_eq!(word(&[0x91, 0x00, 0x01, 0x00]), None);
    }
}
