# Redragon M811 Aatrox protocol notes

Device: 04D9:FC6D, vendor interface usage page FFA0. The app calls it 2913-A.
Sources: vendor_features_packets.txt (full list), vendor_timeline2.txt (actions).

Status words: **tested** = worked from our script. **captured** = seen from the app, not
yet tried from our script. **static** = read from the vendor app's code (RDCfg.exe, radare2,
2026-11-02), not captured or tried. **guess** = pattern only.

## Reports and commands

| Packet | Meaning | Status |
|---|---|---|
| 02 F5 00 | Enter config mode, mouse input stops | tested |
| 02 F5 01 | Exit config mode. Always send it after F5 00 | tested |
| 02 F2 lo hi len | Read, data at reply bytes 8-15 | tested, works without F5 |
| 02 F3 lo hi len 00 00 00 data | Write, report 2 (16 bytes) | tested |
| 03 F3 20 00 0A 00 00 00 data | Write 10 bytes, report 3 (64 bytes) | captured |
| 04 F3 lo hi C8 00 00 00 data | Write 200-byte macro slot, report 4 (256 bytes) | tested |
| 02 F1 02 xx | Apply a group: 01, 02, 04, 08, 10 | tested |

## Memory map

| Address | Meaning | Status |
|---|---|---|
| 0x0020, 10 bytes | Scroll speed, 5 words, one per profile. Written with report 3 | captured, meaning static |
| 0x002A, 2 bytes | Lift-off distance [n, 00], then F1 02 08. App default n = 1 | static |
| 0x002C, 2 bytes | Active profile: 00 = MODE 1, 01 = MODE 2 | tested |
| 0x0032-0x003B | 5 words, polling rate per profile: 01 = 1000 Hz, 02 = 500 Hz | tested |
| 0x003C, 0x003E | "SendKeyMapMode": 3C = [00 01 ..], 3E = [00 00]. Purpose unclear | static |
| 0x0040, 0x0100, 0x01B0, 0x0260, 0x0310 | Profile block base, profiles 1-5 (table in app) | static |
| base + 2 (0x0042 ...) | Active DPI stage of that profile, 1 byte | static |
| base + 4 (0x0044 ...) | DPI stages, 5 entries of 6 bytes: enable X xflag Y yflag + pad | tested |
| 0x0082+ | Button table, 4 bytes per button | tested |
| 0x0449, 51, 59, 61, 69 | LED record per profile, 7 bytes | tested |
| 0x0478 | Macro slot 0, 200 bytes ("full auto", do not touch) | captured |
| 0x0540 | Macro slot 1, 200 bytes | tested |

## LED record

R G B, type, speed, flags, brightness.

| Effect | Type | Speed | Flags | Bright |
|---|---|---|---|---|
| OFF | 00 | 00 | 00 | 02 |
| FULL LIGHTED | 01 | 00 | 02 | 02 |
| BREATHING | 01 | 04 | 04 | 02 |
| RAINBOW | 01 | 04 | 08 | 02 |
| FLASH | 01 | 04 | 10 | 02 |
| WAVE | 02 | 04 | 00 | 02 |
| GO WITHOUT TRACE | 06 | 04 | 00 | 02 |
| REACTIVE | 07 | 04 | 00 | 02 |

The effect is the pair type + flags. Speed and brightness values beyond these: guess until tested.

## DPI

The app has a lookup table per model. M811 ("2913-A") uses model group 3327 (fn 0x431FD0).
Index i = dpi / 100 - 1. All 6 captured values match (500 = 0B ... 6200 = 8C).

| DPI | Code (xflag 00) |
|---|---|
| 100-1000 | 00 04 06 09 0B 0D 0F 12 14 16 |
| 1100-2000 | 18 1B 1D 1F 21 24 26 28 2B 2D |
| 2100-3000 | 2F 31 34 36 38 3A 3D 3F 41 43 |
| 3100-4000 | 46 48 4A 4D 4F 51 53 56 58 5A |
| 4100-5000 | 5C 5F 61 63 66 68 6A 6C 6F 71 |
| 5100-6000 | 73 75 78 7A 7C 7F 81 83 85 87 |
| 6100-6200 | 8A 8C |

Above 6200 the app sets the flag byte (static, untested): 6300-9300 use flag 01 with codes
48 4A ... 8C (same steps again), 9400-11400 use flag 04 with codes 5F 61 ... 8C. The table
ends at 11400. Our tool stops at 6200.

## Buttons

Four bytes each. Side key 1 is at 0x00A2, +4 per key.

| Bytes | Meaning | Status |
|---|---|---|
| 90 00 uu 00 | Keyboard key, uu = HID usage (04 = A, 1E = 1) | tested |
| 8E 01 E9 00 | Volume up (consumer usage 0xE9) | captured |
| 99 81 03 01 | Fire key: left, 3 times, delay 10 | captured |
| 91 00 01 00 | Macro slot 0, 1 loop | captured |
| 91 81 FF FF | Macro slot 1, repeat flag 0x80, loops FFFF | captured |
| 81 82 83 84 85 | Left, right, middle, back, forward | dump |
| 89 8A 8B 8C 8D | DPI-, DPI+, scroll up, scroll down, profile | dump |

### Button function table (static)

The app turns each menu function into a 4-byte entry from a table (0x4BB320, 55 rows).
Names come from the data, not from the menu, unless marked.

| Entry | Meaning |
|---|---|
| 81 / 82 / 83 / 84 / 85 | Left / right / middle / back / forward |
| 00 00 00 00 | Disabled |
| 30 00 00 00 | Unknown (function id 0x0D) |
| 88 00 00 00 | Unknown (id 0x2F) |
| 89 / 8A | DPI- / DPI+ |
| 8B / 8C | Scroll up / scroll down |
| 8D | Profile cycle |
| 94 / 95 | Probably profile + / profile - (menu has both) |
| 97 / 98 | Unknown (ids 0x32 / 0x33) |
| 9A 01 01 | Unknown (id 0x37) |
| 9B 04 X Y | Sniper: while held, DPI = codes X Y from the DPI table |
| 9D | Unknown (id 0x1FE) |
| 8F mm kk 00 | Key with modifiers. mm: 01 Ctrl, 04 Alt, 08 Win. kk = HID usage |
| 8E 01 lo hi | Consumer usage 0xhilo (E9 vol+, EA vol-, E2 mute, CD play, B5 next, B6 prev, B7 stop, 018A mail, 0183 player) |
| 8E 01 FF 1F-25 | Unknown, 7 entries (ids 0x1F-0x25) |
| 90 00 uu 00 | Keyboard key |
| 91 | Macro |
| 99 | Fire key |

Built-in shortcuts in the table: Ctrl+X C V A F N P S, Alt+Tab, Alt+F4, Win+E R D L.

## Macro

Slot is 3-byte events, zero padded.

| Event | Meaning |
|---|---|
| 81 01 00 | Left mouse down |
| 01 01 00 | Left mouse up |
| 84 uu 00 | Keyboard key uu down |
| 04 uu 00 | Keyboard key uu up |
| 06 nn 00 | Delay of nn x 10 ms |

## LED live preview (no F5, tested)

1. 02 F3 46 04 02 00 00
2. 02 F3 49 04 07 00 00 00 + 7 record bytes
3. Same for 51, 59, 61, 69
4. 02 F1 02 04
5. 02 F3 2C 00 02 + active profile word (always write fixed n-1, 00; never read back)

Sending F1 02 04 alone turned the LEDs dark once. Keep the full sequence.

## Profile switch (tested)

F3 002C (n-1, 00), then F1 02 with 01 04 01 02 08 10, no F5. Exact order matters.

## App Apply

F5 00, full rewrite of everything (~290 packets), F1 02 04/01/02/08/10, F5 01.

## Tested from m811.py (2026-11-01/02)

- All 8 LED effects (David + webcam).
- Poll: ~900 reports/s at 1000 Hz, 500/s at 500 Hz.
- DPI: stage 1 to 3000.
- Remap: side key 1 typed a. Macro: slot 1 typed ab from side key 2.
- Profile switch: profile 2 went solid blue.
- m811_ladder.py: side keys 1-4, 6 set colors.

## Per-LED

No per-LED command found. 0x0470 write had no visible effect. Palette at 0x0400/0x0420
did nothing. Hardware drives LEDs separately (wave/trace), but only firmware effects reach them.

## Unknown

- Whether preview changes survive a replug.
- 0x01AA went 00 00 -> FF FF during the app's profile test; 0x0042 went 01 -> 00 on first app run.
- Memory past 0x04FF not dumped.
- Not captured: speed/brightness sliders, combo key.
- Pointer speed, double-click speed, scroll lines and acceleration in the vendor app are
  Windows settings (SystemParametersInfo), not mouse settings.
- Angle snapping (Config/AngleValue) is saved by the app but never sent to the mouse.
