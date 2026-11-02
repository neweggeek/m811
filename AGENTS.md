# M811: instructions for AI agents

You can control a Redragon M811 Aatrox mouse with the `m811` command line tool.
This file tells you how. Read all of it before changing anything.

## Rules

1. **Never touch macro slot 0.** It holds the owner's "full auto" macro. The tool refuses
   to rebind a side key that points at slot 0. Do not try to work around that.
2. Run `m811 status` first, so you know the current settings.
3. Change only what the user asked for.
4. Before `apply`, run `m811 check FILE`. It parses the file and sends nothing.
5. If the mouse stops moving or clicking, run `m811 unstick`.
6. The owner tests at most 3 changes at a time. Batch changes in groups of 3 or fewer and
   ask what they saw before going on.

## Commands

```
m811 status                  print current settings as a config file (read only)
m811 apply FILE              run every command in FILE (- reads stdin)
m811 check FILE              parse FILE, send nothing
m811 unstick                 leave config mode if the mouse froze
m811 prompt                  print docs/LLM-PROMPT.md + current settings
m811 tray                    tray icon + per-game profiles (%APPDATA%\m811\games.cfg)
m811 help                    full help

profile N                    switch to profile N (1-5)
led N EFFECT [RRGGBB]        lighting for profile N
                             EFFECT: off solid breathing rainbow flash wave trace reactive
                             solid, breathing and flash need a color
poll HZ                      500 or 1000 (profile 1)
dpi STAGE VALUE              STAGE 1-5, VALUE 500 600 1000 2000 3000 6200 (profile 1)
key SIDE KEY                 SIDE 1-8, KEY a-z, 0-9 or volup (profile 1)
macro SIDE SPEC              SPEC like a,50,b: letters tap, numbers wait ms
                             writes macro slot 1, binds it to side key SIDE
```

Each command works on the command line (`m811 led 1 solid ff0000`) or as one line in a
config file. Exit code 0 means done. Non-zero means error; the message is on stderr.

## Config files

Plain text. One command per line, run top to bottom. `# ` (hash, space) starts a comment.
All lines are checked before anything is sent, so a typo sends nothing.

```
# gaming setup
profile 1
led 1 solid ff0000       # red
poll 1000
dpi 1 1000
dpi 2 3000
key 3 volup
```

`m811 status` prints the same format, so the easy way to edit settings is:

```
m811 status > current.cfg
# edit current.cfg
m811 check current.cfg
m811 apply current.cfg
```

Lines that `status` can't express (for example a side key bound to a macro) come out as
`# comments`, so applying the file leaves them alone.

## Limits

- Poll rate, DPI and side keys write profile 1 only. Other profiles are not mapped yet.
- DPI accepts only the six values seen in USB captures. The formula is unknown.
- Speed and brightness of effects are fixed (protocol not captured yet).
- Macros: letters and delays only, slot 1 only.

## How it works

USB HID feature reports. See [PROTOCOL.md](PROTOCOL.md). Packet builders are in
`src/proto.rs` and match the tested Python tool in `tools/m811.py` byte for byte.
