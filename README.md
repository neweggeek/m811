# m811

Small desktop app to control the Redragon M811 Aatrox mouse (USB 04D9:FC6D) without
the vendor app. Rust, two dependencies: `eframe` (window) and `hidapi` (USB).

Protocol was worked out from USB captures of the vendor app. See [PROTOCOL.md](PROTOCOL.md).
Every packet the app sends is a copy of what the tested Python tool in `tools/m811.py`
sends. Unit tests check the bytes match.

## What it does

- Switch profile 1-5
- Lighting per profile: off, solid, breathing, rainbow, flash, wave, go without trace, reactive
- Poll rate 500 / 1000 Hz (profile 1)
- DPI stages 1-5 (profile 1, only DPI values seen in captures)
- Remap side keys 1-8 to a letter, digit or volume up (profile 1)
- Write macro slot 1 and bind it to a side key

Safety:

- Macro slot 0 is never written. A side key bound to slot 0 shows as locked.
- Every config-mode write ends with "config mode off", even on errors.
  If the mouse ever stops moving, press **Unstick**.

## Get it

Download `m811-windows` from the latest
[build run](../../actions/workflows/build.yml), unzip, run `m811.exe`.

Or build it yourself:

```
cargo build --release
target\release\m811.exe
```

## Code

- `src/proto.rs` packet builders, no I/O, with tests
- `src/mouse.rs` HID open / read / send
- `src/main.rs` the window
