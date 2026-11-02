#!/usr/bin/env python
"""Control the Redragon M811 without the vendor app. See PROTOCOL.md.

  m811.py status
  m811.py dump FILE                read-only dump of 0x0000-0x04FF
  m811.py unstick                  send F5 01 if mouse input is stuck
  m811.py profile 1-5
  m811.py led wave | rainbow | off | reactive | trace
  m811.py led full | breathing | flash RRGGBB
          led takes --profile N (default 1)
  m811.py poll 500 | 1000
  m811.py dpi STAGE VALUE          STAGE 1-5, VALUE one of the known list
  m811.py key SIDE KEY             SIDE 1-8, KEY a letter, a digit or volup
  m811.py macro SIDE SPEC          SPEC like a,50,b. Writes macro slot 1, binds SIDE.

Add --dry to print packets without opening the mouse.
Needs: pip install hidapi   (Windows tested: Win11, Python 3.14, hidapi 0.15)
"""
import sys, time

VID, PID, USAGE_PAGE = 0x04D9, 0xFC6D, 0xFFA0

# effect: (type, speed, flags, needs color). From vendor_features_packets.txt.
EFFECTS = {
    "off": (0x00, 0x00, 0x00, False),
    "full": (0x01, 0x00, 0x02, True),
    "breathing": (0x01, 0x04, 0x04, True),
    "rainbow": (0x01, 0x04, 0x08, False),
    "flash": (0x01, 0x04, 0x10, True),
    "wave": (0x02, 0x04, 0x00, False),
    "trace": (0x06, 0x04, 0x00, False),
    "reactive": (0x07, 0x04, 0x00, False),
}
BRIGHTNESS = 0x02
WAVE_RECORD = (0xFF, 0x00, 0x00, 0x02, 0x04, 0x00, 0x02)

DPI_VALUES = {500: 0x0B, 600: 0x0D, 1000: 0x16, 2000: 0x2D, 3000: 0x43, 6200: 0x8C}
POLL_CODES = {1000: 0x01, 500: 0x02}
SIDE_KEY_BASE = 0x00A2  # side key 1, profile 1. 4 bytes per key.
MACRO_SLOT = 1  # slot 0 at 0x0478 is David's "full auto". Only slot 1 (0x0540) is written.


def pkt(*b):
    return bytes(b).ljust(16, b"\0")


def write(addr, *data):
    return pkt(0x02, 0xF3, addr & 0xFF, addr >> 8, len(data), 0, 0, 0, *data)


ENTER = pkt(0x02, 0xF5, 0x00)
EXIT = pkt(0x02, 0xF5, 0x01)
APPLY_ALL = [pkt(0x02, 0xF1, 0x02, g) for g in (0x04, 0x01, 0x02, 0x08, 0x10)]


def led_packets(args, dev=None):
    name = args[0]
    kind, speed, flags, needs_color = EFFECTS[name]
    if needs_color:
        rgb = bytes.fromhex(args[1])
        if len(rgb) != 3:
            raise ValueError("color must be RRGGBB")
    else:
        rgb = bytes((0xFF, 0x00, 0x00))
    n = int(args[args.index("--profile") + 1]) if "--profile" in args else 1
    if not 1 <= n <= 5:
        raise ValueError("profile is 1-5")
    packets = [write(0x0446, 0, 0)]
    for i, a in enumerate((0x0449, 0x0451, 0x0459, 0x0461, 0x0469)):
        if i == n - 1:
            packets.append(write(a, *rgb, kind, speed, flags, BRIGHTNESS))
        else:  # keep what the other profiles hold
            packets.append(write(a, *(read(dev, a - 1)[1:8] if dev else WAVE_RECORD)))
    # Fixed value from the profile number. Never read this word back and rewrite it:
    # that left 83 00 in it once.
    return packets + [pkt(0x02, 0xF1, 0x02, 0x04), write(0x002C, n - 1, 0)]


def profile_packets(args, dev=None):
    n = int(args[0])
    if not 1 <= n <= 5:
        raise ValueError("profile is 1-5")
    # Exact order from the app on MODE 2, frames 4529-4541. No F5.
    return [write(0x002C, n - 1, 0)] + [pkt(0x02, 0xF1, 0x02, g) for g in (0x01, 0x04, 0x01, 0x02, 0x08, 0x10)]


def poll_packets(args, dev=None):
    return [ENTER, write(0x0032, POLL_CODES[int(args[0])], 0x00), *APPLY_ALL, EXIT]


def dpi_packets(args, dev=None):
    stage, dpi = int(args[0]), int(args[1])
    if not 1 <= stage <= 5:
        raise ValueError("stage is 1-5")
    v = DPI_VALUES[dpi]
    return [ENTER, write(0x0044 + 6 * (stage - 1), 0x01, v, 0x00, v, 0x00), *APPLY_ALL, EXIT]


def key_packets(args, dev=None):
    side, key = int(args[0]), args[1].lower()
    if not 1 <= side <= 8:
        raise ValueError("side key is 1-8")
    addr = SIDE_KEY_BASE + 4 * (side - 1)
    if dev:
        lo = addr & ~7
        current = read(dev, lo)[addr - lo:][:4]
        if current[0] == 0x91 and current[1] & 0x7F == 0:  # slot 0 is David's "full auto"
            raise ValueError(f"side key {side} holds slot 0 macro, refusing to change it")
    if key == "volup":
        entry = (0x8E, 0x01, 0xE9, 0x00)
    elif len(key) == 1 and "a" <= key <= "z":
        entry = (0x90, 0x00, 0x04 + ord(key) - ord("a"), 0x00)
    elif len(key) == 1 and "1" <= key <= "9":
        entry = (0x90, 0x00, 0x1E + ord(key) - ord("1"), 0x00)
    elif key == "0":
        entry = (0x90, 0x00, 0x27, 0x00)
    else:
        raise ValueError("key is a letter, a digit or volup")
    return [ENTER, write(addr, *entry), *APPLY_ALL, EXIT]


def macro_packets(args, dev=None):
    """macro SIDE SPEC: write SPEC to slot 1 and bind it. SPEC is like a,50,b (ms between)."""
    side, spec = int(args[0]), args[1].lower().split(",")
    if not 1 <= side <= 8:
        raise ValueError("side key is 1-8")
    events = []
    for token in spec:
        if token.isdigit():
            events += [0x06, min(int(token) // 10, 255), 0x00]
        elif len(token) == 1 and "a" <= token <= "z":
            usage = 0x04 + ord(token) - ord("a")
            events += [0x84, usage, 0x00, 0x04, usage, 0x00]
        else:
            raise ValueError("spec tokens are letters and millisecond numbers")
    if len(events) > 200:
        raise ValueError("macro is too long for one slot")
    slot = bytes(events).ljust(200, b"\0")
    report = bytes((0x04, 0xF3, 0x40, 0x05, 0xC8, 0, 0, 0)) + slot
    bind = write(SIDE_KEY_BASE + 4 * (side - 1), 0x91, MACRO_SLOT, 0x01, 0x00)
    return [ENTER, bind, *APPLY_ALL, report.ljust(256, b"\0"), EXIT]


COMMANDS = {
    "macro": macro_packets,
    "led": led_packets,
    "poll": poll_packets,
    "dpi": dpi_packets,
    "key": key_packets,
    "profile": profile_packets,
}


def open_mouse():
    import hid

    path = next(d["path"] for d in hid.enumerate(VID, PID) if d["usage_page"] == USAGE_PAGE)
    dev = hid.device()
    dev.open_path(path)
    return dev


def send_all(dev, packets):
    entered = False
    try:
        for p in packets:
            if dev.send_feature_report(p) < 0:
                raise OSError(f"send failed on {p.hex(' ')}: {dev.error()}")
            entered = entered or p == ENTER
            if p == EXIT:
                entered = False
            time.sleep(0.03)
    finally:
        if entered:  # never leave the mouse with input paused
            print("error path: F5 01 returned", dev.send_feature_report(EXIT))
            time.sleep(0.03)


def read(dev, addr):
    dev.send_feature_report(pkt(0x02, 0xF2, addr & 0xFF, addr >> 8, 8))
    time.sleep(0.01)
    return bytes(dev.get_feature_report(2, 16))[8:16]


def status(dev):
    head = read(dev, 0x0028)
    rec = read(dev, 0x0448)
    poll = read(dev, 0x0030)
    print(f"profile word 0x002C: {head[4]:02x} {head[5]:02x}")
    print(f"poll word 0x0032: {poll[2]:02x} {poll[3]:02x}")
    print(f"led record 0x0449: {rec[1:8].hex(' ')}")
    print(f"dpi 0x0040: {read(dev, 0x0040).hex(' ')}")
    print(f"side keys 0x00A0: {read(dev, 0x00A0).hex(' ')} {read(dev, 0x00A8).hex(' ')}")


def dump(dev, name):
    with open(name, "w") as f:
        for addr in range(0x0000, 0x0500, 8):
            dev.send_feature_report(pkt(0x02, 0xF2, addr & 0xFF, addr >> 8, 8))
            time.sleep(0.01)
            f.write(f"{addr:04x}: {bytes(dev.get_feature_report(2, 16)).hex(' ')}\n")
    print("saved", name)


def main(argv):
    dry = "--dry" in argv
    argv = [a for a in argv if a != "--dry"]
    if not argv or argv[0] not in (*COMMANDS, "status", "dump", "unstick"):
        sys.exit(__doc__)
    if dry:
        for p in COMMANDS[argv[0]](argv[1:]):
            print(p.hex(" "))
        return
    dev = open_mouse()
    try:
        if argv[0] == "status":
            status(dev)
        elif argv[0] == "dump":
            dump(dev, argv[1])
        elif argv[0] == "unstick":
            print("F5 01 returned", dev.send_feature_report(EXIT))
        else:
            packets = COMMANDS[argv[0]](argv[1:], dev)
            send_all(dev, packets)
            print(f"sent {len(packets)} packets")
    finally:
        dev.close()


if __name__ == "__main__":
    main(sys.argv[1:])
