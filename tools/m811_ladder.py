#!/usr/bin/env python
"""Side number buttons set a solid LED color. They still type their numbers.

  m811_ladder.py [MINUTES]     default runs until Ctrl+C, then sets wave

Listens to Raw Input and only reacts to the M811, not the real keyboard.
Keys 5 and 8 hold the "full auto" macro, so they send no number here.
Windows only. Keep next to m811.py.
"""
import ctypes, sys, time
from ctypes import wintypes as w

import m811

COLORS = {
    0x31: ("red", "ff0000"),
    0x32: ("orange", "ff8000"),
    0x33: ("yellow", "ffff00"),
    0x34: ("green", "00ff00"),
    0x36: ("blue", "0000ff"),
    0x37: ("purple", "8000ff"),
}
MOUSE_ID = "VID_04D9&PID_FC6D"
DEBOUNCE_S = 0.1
WM_INPUT = 0x00FF
RIDEV_INPUTSINK = 0x100
RID_INPUT = 0x10000003
RIDI_DEVICENAME = 0x20000007
HWND_MESSAGE = w.HWND(-3)
LRESULT = ctypes.c_ssize_t
WNDPROC = ctypes.WINFUNCTYPE(LRESULT, w.HWND, w.UINT, w.WPARAM, w.LPARAM)

user32 = ctypes.WinDLL("user32", use_last_error=True)
user32.DefWindowProcW.argtypes = (w.HWND, w.UINT, w.WPARAM, w.LPARAM)
user32.DefWindowProcW.restype = LRESULT
user32.GetRawInputData.argtypes = (w.LPARAM, w.UINT, w.LPVOID, ctypes.POINTER(w.UINT), w.UINT)
user32.GetRawInputDeviceInfoW.argtypes = (w.HANDLE, w.UINT, w.LPVOID, ctypes.POINTER(w.UINT))
user32.CreateWindowExW.argtypes = (
    w.DWORD, w.LPCWSTR, w.LPCWSTR, w.DWORD, ctypes.c_int, ctypes.c_int, ctypes.c_int,
    ctypes.c_int, w.HWND, w.HMENU, w.HINSTANCE, w.LPVOID)
user32.CreateWindowExW.restype = w.HWND


class WNDCLASS(ctypes.Structure):
    _fields_ = [("style", w.UINT), ("proc", WNDPROC), ("cbcls", ctypes.c_int),
                ("cbwnd", ctypes.c_int), ("inst", w.HINSTANCE), ("icon", w.HANDLE),
                ("cursor", w.HANDLE), ("brush", w.HANDLE), ("menu", w.LPCWSTR),
                ("name", w.LPCWSTR)]


class RAWINPUTDEVICE(ctypes.Structure):
    _fields_ = [("page", w.USHORT), ("usage", w.USHORT), ("flags", w.DWORD), ("target", w.HWND)]


names = {}
last_press = 0.0
dev = None


def is_mouse(handle):
    if handle not in names:
        size = w.UINT(512)
        buf = ctypes.create_unicode_buffer(512)
        user32.GetRawInputDeviceInfoW(handle, RIDI_DEVICENAME, buf, ctypes.byref(size))
        names[handle] = MOUSE_ID in buf.value.upper()
    return names[handle]


def on_key(vkey):
    global last_press
    now = time.monotonic()
    if vkey not in COLORS or now - last_press < DEBOUNCE_S:
        return
    last_press = now
    name, rgb = COLORS[vkey]
    m811.send_all(dev, m811.led_packets(["full", rgb], dev))
    print(f"{vkey - 0x30} {name}", flush=True)


def proc(hwnd, msg, wparam, lparam):
    if msg == WM_INPUT:
        size = w.UINT(64)
        buf = ctypes.create_string_buffer(64)
        if user32.GetRawInputData(lparam, RID_INPUT, buf, ctypes.byref(size), 24) != w.UINT(-1).value:
            handle = int.from_bytes(buf.raw[8:16], "little")
            flags = int.from_bytes(buf.raw[26:28], "little")
            vkey = int.from_bytes(buf.raw[30:32], "little")
            if flags & 1 == 0 and is_mouse(handle):  # bit 0 set means key up
                on_key(vkey)
    return user32.DefWindowProcW(hwnd, msg, wparam, lparam)


def main():
    global dev
    minutes = float(sys.argv[1]) if len(sys.argv) > 1 else None
    deadline = time.monotonic() + minutes * 60 if minutes else None
    dev = m811.open_mouse()
    wndproc = WNDPROC(proc)
    cls = WNDCLASS(proc=wndproc, name="m811ladder")
    user32.RegisterClassW(ctypes.byref(cls))
    hwnd = user32.CreateWindowExW(0, "m811ladder", "m811", 0, 0, 0, 0, 0, HWND_MESSAGE, None, None, None)
    rid = RAWINPUTDEVICE(1, 6, RIDEV_INPUTSINK, hwnd)
    if not user32.RegisterRawInputDevices(ctypes.byref(rid), 1, ctypes.sizeof(rid)):
        raise ctypes.WinError(ctypes.get_last_error())
    print("listening", flush=True)
    msg = w.MSG()
    try:
        while deadline is None or time.monotonic() < deadline:
            while user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, 1):
                user32.TranslateMessage(ctypes.byref(msg))
                user32.DispatchMessageW(ctypes.byref(msg))
            time.sleep(0.01)
    except KeyboardInterrupt:
        pass
    finally:
        m811.send_all(dev, m811.led_packets(["wave"], dev))
        dev.close()
        print("wave restored", flush=True)


if __name__ == "__main__":
    main()
