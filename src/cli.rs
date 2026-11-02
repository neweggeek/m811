//! Command line + config files. One command per line, same words as the CLI.
//! Built for people and for LLMs: plain text, no JSON, one thing per line.

use crate::mouse::{Mouse, Status};
use crate::proto::{self, Effect};

pub const HELP: &str = "\
m811 - control the Redragon M811 mouse

  m811                         open the window
  m811 status                  print current settings (read only)
  m811 apply FILE              run every command in FILE (- = stdin)
  m811 check FILE              parse FILE, send nothing
  m811 unstick                 leave config mode if the mouse froze
  m811 prompt                  print a doc to paste into any LLM (+ current settings)
  m811 tray                    tray icon, switches settings per game (games.cfg)

Commands (also valid as lines in a config file):
  profile N                    switch to profile N (1-5)
  led N EFFECT [RRGGBB]        lighting for profile N
                               EFFECT: off solid breathing rainbow flash wave trace reactive
                               solid, breathing, flash need a color
  poll HZ                      500 or 1000 (profile 1)
  dpi STAGE VALUE              STAGE 1-5, VALUE 200-6200 in steps of 100 (profile 1)
  lift N                       lift-off distance 1 or 2 (untested)
  scroll N                     scroll speed 1-10 (profile 1, untested)
  key SIDE KEY                 SIDE 1-8, KEY a-z, 0-9 or volup (profile 1)
  macro SIDE SPEC              SPEC like a,50,b (letters tap, numbers wait ms)
                               writes macro slot 1, binds it to SIDE

Config files: one command per line, a # followed by a space starts a comment. Lines run top to bottom.
Side keys bound to macro slot 0 are never changed.";

#[derive(Debug, PartialEq)]
pub enum Cmd {
    Profile(usize),
    Led(usize, Effect, [u8; 3]),
    Poll(u32),
    Dpi(usize, u32),
    Lift(u8),
    Scroll(u8),
    Key(usize, String),
    Macro(usize, String),
}

fn num(s: Option<&str>, what: &str, lo: usize, hi: usize) -> Result<usize, String> {
    let s = s.ok_or(format!("missing {what}"))?;
    let n: usize = s.parse().map_err(|_| format!("{what} must be a number, got '{s}'"))?;
    if n < lo || n > hi {
        return Err(format!("{what} must be {lo}-{hi}, got {n}"));
    }
    Ok(n)
}

fn color(s: &str) -> Result<[u8; 3], String> {
    let s = s.trim_start_matches('#');
    let bad = || format!("color must be RRGGBB hex, got '{s}'");
    if s.len() != 6 {
        return Err(bad());
    }
    let b = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| bad());
    Ok([b(0)?, b(2)?, b(4)?])
}

pub fn effect_from_word(w: &str) -> Option<Effect> {
    Effect::ALL.into_iter().find(|e| e.word() == w)
}

/// Parse one line. Ok(None) for blank or comment lines.
pub fn parse(line: &str) -> Result<Option<Cmd>, String> {
    // "# " starts a comment, so colors like #00ff00 still work.
    let line = line.split("# ").next().unwrap().trim();
    if line.starts_with('#') {
        return Ok(None);
    }
    let mut w = line.split_whitespace();
    let Some(cmd) = w.next() else { return Ok(None) };
    let c = match cmd.to_lowercase().as_str() {
        "profile" => Cmd::Profile(num(w.next(), "profile", 1, 5)?),
        "led" => {
            let p = num(w.next(), "profile", 1, 5)?;
            let name = w.next().ok_or("missing effect")?.to_lowercase();
            let e = effect_from_word(&name).ok_or(format!("unknown effect '{name}'"))?;
            let rgb = match (e.needs_color(), w.next()) {
                (true, Some(c)) => color(c)?,
                (true, None) => return Err(format!("{name} needs a color like ff0000")),
                (false, _) => [0xFF, 0, 0],
            };
            Cmd::Led(p, e, rgb)
        }
        "poll" => match w.next() {
            Some("500") => Cmd::Poll(500),
            Some("1000") => Cmd::Poll(1000),
            _ => return Err("poll must be 500 or 1000".into()),
        },
        "dpi" => {
            let stage = num(w.next(), "stage", 1, 5)?;
            let v = num(w.next(), "dpi", 1, 100_000)? as u32;
            if proto::dpi(stage, v).is_none() {
                return Err(format!("dpi must be 200-6200 in steps of 100, got {v}"));
            }
            Cmd::Dpi(stage, v)
        }
        "lift" => Cmd::Lift(num(w.next(), "lift", 1, 2)? as u8),
        "scroll" => Cmd::Scroll(num(w.next(), "scroll", 1, 10)? as u8),
        "key" => {
            let side = num(w.next(), "side key", 1, 8)?;
            let k = w.next().ok_or("missing key")?.to_string();
            proto::key_entry(&k).ok_or(format!("key must be a-z, 0-9 or volup, got '{k}'"))?;
            Cmd::Key(side, k)
        }
        "macro" => {
            let side = num(w.next(), "side key", 1, 8)?;
            let spec = w.next().ok_or("missing macro spec")?.to_string();
            proto::macro_(side, &spec)?;
            Cmd::Macro(side, spec)
        }
        other => return Err(format!("unknown command '{other}'")),
    };
    if let Some(extra) = w.next() {
        return Err(format!("unexpected '{extra}'"));
    }
    Ok(Some(c))
}

/// Parse a whole file. All lines must be good before anything is sent.
pub fn parse_all(text: &str) -> Result<Vec<Cmd>, String> {
    let mut out = vec![];
    for (i, line) in text.lines().enumerate() {
        match parse(line) {
            Ok(Some(c)) => out.push(c),
            Ok(None) => {}
            Err(e) => return Err(format!("line {}: {e}", i + 1)),
        }
    }
    Ok(out)
}

pub fn run(m: &Mouse, c: &Cmd) -> Result<(), String> {
    match c {
        Cmd::Profile(n) => m.send_all(&proto::profile(*n)),
        Cmd::Led(p, e, rgb) => m.send_all(&proto::led(*p, *e, *rgb, &m.leds()?, m.active_profile()?)),
        Cmd::Poll(hz) => m.send_all(&proto::poll(*hz)),
        Cmd::Dpi(s, v) => m.send_all(&proto::dpi(*s, *v).unwrap()),
        Cmd::Lift(n) => m.send_all(&proto::lift(*n)),
        Cmd::Scroll(n) => {
            let cur: [u8; 10] = m.read_n(0x0020, 10)?.try_into().unwrap();
            m.send_all(&proto::scroll(*n, &cur))
        }
        Cmd::Key(side, k) => {
            m.guard_side_key(*side)?;
            m.send_all(&proto::key(*side, proto::key_entry(k).unwrap()))
        }
        Cmd::Macro(side, spec) => {
            m.guard_side_key(*side)?;
            m.send_all(&proto::macro_(*side, spec)?)
        }
    }
}

/// Status as config lines, so the output can be edited and fed back to `apply`.
pub fn status_text(s: &Status) -> String {
    let mut t = "# M811 status. Valid config: edit and run `m811 apply`.\n".to_string();
    for (i, rec) in s.leds.iter().enumerate() {
        match Effect::from_record(rec) {
            Some(e) if e.needs_color() => {
                t += &format!("led {} {} {:02x}{:02x}{:02x}\n", i + 1, e.word(), rec[0], rec[1], rec[2])
            }
            Some(e) => t += &format!("led {} {}\n", i + 1, e.word()),
            None => t += &format!("# led {} unknown record {:02x?}\n", i + 1, rec),
        }
    }
    match s.poll_hz {
        Some(hz) => t += &format!("poll {hz}\n"),
        None => t += "# poll unknown\n",
    }
    for (i, &c) in s.dpi.iter().enumerate() {
        match proto::dpi_from_code(c) {
            Some(d) => t += &format!("dpi {} {d}\n", i + 1),
            None => t += &format!("# dpi {} unknown code {c:02x}\n", i + 1),
        }
    }
    for (i, e) in s.side_keys.iter().enumerate() {
        let d = proto::describe_entry(e);
        if proto::key_entry(&d).is_some() {
            t += &format!("key {} {d}\n", i + 1);
        } else {
            t += &format!("# key {} = {d}\n", i + 1);
        }
    }
    // Last, so the mouse ends on this profile whatever the lines above do.
    t += &format!("profile {}\n", s.profile);
    t
}

pub fn main(args: &[String]) -> i32 {
    match cli(args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn read_file(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut s = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut s).map_err(|e| e.to_string())?;
        return Ok(s);
    }
    std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
}

fn cli(args: &[String]) -> Result<(), String> {
    let first = args[0].as_str();
    match first {
        "help" | "-h" | "--help" => println!("{HELP}"),
        "tray" => crate::tray::run()?,
        "prompt" => {
            println!("{}", include_str!("../docs/LLM-PROMPT.md"));
            match Mouse::open().and_then(|m| m.status()) {
                Ok(s) => println!("## Current settings\n\n```\n{}```", status_text(&s)),
                Err(e) => println!("## Current settings\n\nUnknown ({e})."),
            }
        }
        "status" => print!("{}", status_text(&Mouse::open()?.status()?)),
        "unstick" => Mouse::open()?.unstick()?,
        "check" | "apply" => {
            let path = args.get(1).ok_or("missing FILE")?;
            let cmds = parse_all(&read_file(path)?)?;
            if first == "check" {
                println!("ok, {} commands", cmds.len());
                return Ok(());
            }
            let m = Mouse::open()?;
            for c in &cmds {
                run(&m, c).map_err(|e| format!("{c:?}: {e}"))?;
                println!("done {c:?}");
            }
        }
        _ => {
            let c = parse(&args.join(" "))?.ok_or(HELP)?;
            run(&Mouse::open()?, &c)?;
            println!("done");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        assert_eq!(parse("led 2 solid #00ff00 # green").unwrap(), Some(Cmd::Led(2, Effect::Full, [0, 255, 0])));
        assert_eq!(parse("LED 1 wave").unwrap(), Some(Cmd::Led(1, Effect::Wave, [255, 0, 0])));
        assert_eq!(parse("  # just a comment").unwrap(), None);
        assert_eq!(parse("dpi 3 2000").unwrap(), Some(Cmd::Dpi(3, 2000)));
        assert_eq!(parse("dpi 3 2100").unwrap(), Some(Cmd::Dpi(3, 2100)));
        assert!(parse("dpi 3 2150").is_err());
        assert_eq!(parse("lift 2").unwrap(), Some(Cmd::Lift(2)));
        assert!(parse("lift 3").is_err());
        assert_eq!(parse("scroll 3").unwrap(), Some(Cmd::Scroll(3)));
        assert!(parse("scroll 0").is_err());
        assert!(parse("led 1 solid").is_err());
        assert!(parse("key 9 a").is_err());
        assert!(parse("key 1 a b").is_err());
        assert!(parse("macro 2 a,!").is_err());
        assert!(parse("jump").is_err());
    }

    #[test]
    fn bad_line_number() {
        let e = parse_all("profile 1\n\npoll 750\n").unwrap_err();
        assert!(e.starts_with("line 3"), "{e}");
    }

    #[test]
    fn status_round_trips() {
        let s = Status {
            profile: 1,
            poll_hz: Some(500),
            leds: [[0, 255, 0, 1, 0, 2, 2], [255, 0, 0, 2, 4, 0, 2], [0; 7], [0; 7], [0; 7]],
            dpi: [0x0B, 0x16, 0x2D, 0x43, 0x8C],
            side_keys: [[0x90, 0, 0x1E, 0], [0x91, 0, 1, 0], [0x8E, 1, 0xE9, 0], [0x90, 0, 4, 0], [0; 4], [0; 4], [0; 4], [0; 4]],
        };
        let t = status_text(&s);
        assert!(t.contains("led 1 solid 00ff00\n"));
        assert!(t.contains("# key 2 = macro slot 0"));
        assert!(t.contains("key 3 volup\n"));
        assert!(t.ends_with("profile 1\n"));
        assert_eq!(parse_all(&t).unwrap().len(), 1 + 5 + 1 + 5 + 3);
    }
}
