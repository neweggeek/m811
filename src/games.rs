#![cfg_attr(not(windows), allow(dead_code))] // Watcher is only used by the Windows tray.
//! Per-game profiles. games.cfg holds blocks:
//!
//!   game valorant.exe
//!   profile 2
//!   default
//!   profile 1
//!
//! Lines under `game X` run when X comes to the front. Lines under `default` run when the
//! front window is none of the games. Only runs on a change, never every tick.

use crate::cli::{self, Cmd};
use std::path::PathBuf;

pub const EXAMPLE: &str = "\
# Per-game profiles. Lines under `game NAME.exe` run when that program is in front.
# Lines under `default` run when you leave every listed game.
# Same commands as config files. Example:
#
# game valorant.exe
# led 2 solid ff0000
# profile 2
#
# default
# profile 1
";

#[derive(Debug, Default)]
pub struct Rules {
    pub games: Vec<(String, Vec<Cmd>)>,
    pub default: Vec<Cmd>,
}

pub fn parse_rules(text: &str) -> Result<Rules, String> {
    let mut r = Rules::default();
    // None = before any header, Some(None) = default block, Some(Some(i)) = game i
    let mut block: Option<Option<usize>> = None;
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let body = line.split("# ").next().unwrap().trim();
        let mut words = body.split_whitespace();
        match words.next().map(|w| w.to_lowercase()).as_deref() {
            Some("game") => {
                let exe = words.next().ok_or(format!("line {n}: game needs an exe name"))?;
                r.games.push((exe.to_lowercase(), vec![]));
                block = Some(Some(r.games.len() - 1));
            }
            Some("default") => block = Some(None),
            _ => {
                let Some(cmd) = cli::parse(line).map_err(|e| format!("line {n}: {e}"))? else { continue };
                match block {
                    None => return Err(format!("line {n}: put `game NAME.exe` or `default` above it")),
                    Some(None) => r.default.push(cmd),
                    Some(Some(g)) => r.games[g].1.push(cmd),
                }
            }
        }
    }
    Ok(r)
}

/// Which game block matches this exe file name (case does not matter).
pub fn pick(rules: &Rules, exe: &str) -> Option<usize> {
    let exe = exe.to_lowercase();
    rules.games.iter().position(|(g, _)| *g == exe)
}

pub fn path() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    base.join("m811").join("games.cfg")
}

pub fn load_text() -> String {
    std::fs::read_to_string(path()).unwrap_or_else(|_| EXAMPLE.to_string())
}

pub fn save_text(text: &str) -> Result<(), String> {
    parse_rules(text)?;
    let p = path();
    std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))
}

/// Remembers what ran last so commands only go out on a change.
#[derive(Default)]
pub struct Watcher {
    last: Option<Option<usize>>,
}

impl Watcher {
    /// Call with the front exe name every tick. Returns a label and the commands to run,
    /// or None when nothing changed.
    pub fn tick<'a>(&mut self, rules: &'a Rules, exe: &str) -> Option<(String, &'a [Cmd])> {
        let now = pick(rules, exe);
        if self.last == Some(now) {
            return None;
        }
        self.last = Some(now);
        match now {
            Some(g) => Some((rules.games[g].0.clone(), &rules.games[g].1)),
            None => Some(("default".into(), &rules.default)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: &str = "\
# my games
game Valorant.exe
led 2 solid ff0000   # red
profile 2

game cs2.exe
profile 3

default
profile 1
";

    #[test]
    fn parses() {
        let r = parse_rules(RULES).unwrap();
        assert_eq!(r.games.len(), 2);
        assert_eq!(r.games[0].0, "valorant.exe");
        assert_eq!(r.games[0].1.len(), 2);
        assert_eq!(r.default, vec![Cmd::Profile(1)]);
        assert_eq!(pick(&r, "VALORANT.EXE"), Some(0));
        assert_eq!(pick(&r, "notepad.exe"), None);
        assert!(parse_rules(EXAMPLE).unwrap().games.is_empty());
    }

    #[test]
    fn errors() {
        assert!(parse_rules("profile 1").unwrap_err().starts_with("line 1"));
        assert!(parse_rules("game\n").is_err());
        assert!(parse_rules("game a.exe\npoll 7").unwrap_err().starts_with("line 2"));
    }

    #[test]
    fn fires_only_on_change() {
        let r = parse_rules(RULES).unwrap();
        let mut w = Watcher::default();
        assert_eq!(w.tick(&r, "explorer.exe").unwrap().0, "default");
        assert!(w.tick(&r, "notepad.exe").is_none());
        let (name, cmds) = w.tick(&r, "cs2.exe").unwrap();
        assert_eq!((name.as_str(), cmds), ("cs2.exe", &[Cmd::Profile(3)][..]));
        assert!(w.tick(&r, "cs2.exe").is_none());
        assert_eq!(w.tick(&r, "chrome.exe").unwrap().0, "default");
    }
}
