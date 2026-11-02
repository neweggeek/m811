//! Windows tray icon + per-game watcher + start with Windows. Raw Win32, no crates.

#[cfg(not(windows))]
mod imp {
    pub fn run() -> Result<(), String> {
        Err("tray is Windows only".into())
    }
    pub fn startup_enabled() -> bool {
        false
    }
    pub fn set_startup(_on: bool) -> Result<(), String> {
        Err("start with Windows is Windows only".into())
    }
}

#[cfg(windows)]
mod imp {
    use crate::{cli, games, mouse::Mouse};
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use std::ptr::null_mut;

    type H = *mut c_void;

    #[repr(C)]
    struct WndClass {
        style: u32,
        proc_: extern "system" fn(H, u32, usize, isize) -> isize,
        cls_extra: i32,
        wnd_extra: i32,
        instance: H,
        icon: H,
        cursor: H,
        background: H,
        menu_name: *const u16,
        class_name: *const u16,
    }

    #[repr(C)]
    struct NotifyIconData {
        size: u32,
        wnd: H,
        id: u32,
        flags: u32,
        callback_msg: u32,
        icon: H,
        tip: [u16; 128],
        state: u32,
        state_mask: u32,
        info: [u16; 256],
        version: u32,
        info_title: [u16; 64],
        info_flags: u32,
        guid: [u8; 16],
        balloon_icon: H,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct Msg {
        wnd: H,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        pt: Point,
        private: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn RegisterClassW(c: *const WndClass) -> u16;
        fn CreateWindowExW(ex: u32, class: *const u16, name: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: H, menu: H, inst: H, param: H) -> H;
        fn DefWindowProcW(w: H, m: u32, wp: usize, lp: isize) -> isize;
        fn GetMessageW(m: *mut Msg, w: H, min: u32, max: u32) -> i32;
        fn TranslateMessage(m: *const Msg) -> i32;
        fn DispatchMessageW(m: *const Msg) -> isize;
        fn PostQuitMessage(code: i32);
        fn SetTimer(w: H, id: usize, ms: u32, f: H) -> usize;
        fn LoadIconW(inst: H, name: *const u16) -> H;
        fn CreatePopupMenu() -> H;
        fn AppendMenuW(menu: H, flags: u32, id: usize, text: *const u16) -> i32;
        fn TrackPopupMenu(menu: H, flags: u32, x: i32, y: i32, r: i32, w: H, rect: H) -> i32;
        fn DestroyMenu(menu: H) -> i32;
        fn GetCursorPos(p: *mut Point) -> i32;
        fn SetForegroundWindow(w: H) -> i32;
        fn GetForegroundWindow() -> H;
        fn GetWindowThreadProcessId(w: H, pid: *mut u32) -> u32;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn Shell_NotifyIconW(msg: u32, data: *mut NotifyIconData) -> i32;
    }

    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> H;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> H;
        fn QueryFullProcessImageNameW(p: H, flags: u32, buf: *mut u16, len: *mut u32) -> i32;
        fn CloseHandle(h: H) -> i32;
        fn CreateMutexW(attr: H, owner: i32, name: *const u16) -> H;
        fn GetLastError() -> u32;
    }

    const WM_TIMER: u32 = 0x0113;
    const WM_APP_TRAY: u32 = 0x8001;
    const WM_LBUTTONDBLCLK: u32 = 0x0203;
    const WM_RBUTTONUP: u32 = 0x0205;
    const NIM_ADD: u32 = 0;
    const NIM_MODIFY: u32 = 1;
    const NIM_DELETE: u32 = 2;
    const NIF_MESSAGE: u32 = 1;
    const NIF_ICON: u32 = 2;
    const NIF_TIP: u32 = 4;
    const MF_STRING: u32 = 0;
    const MF_CHECKED: u32 = 8;
    const MF_SEPARATOR: u32 = 0x800;
    const TPM_RETURNCMD: u32 = 0x100;
    const TPM_RIGHTBUTTON: u32 = 2;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const ERROR_ALREADY_EXISTS: u32 = 183;
    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

    const ID_OPEN: usize = 1;
    const ID_STARTUP: usize = 2;
    const ID_EDIT: usize = 3;
    const ID_QUIT: usize = 4;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// File name of the program whose window is in front, like "valorant.exe".
    fn foreground_exe() -> Option<String> {
        unsafe {
            let w = GetForegroundWindow();
            if w.is_null() {
                return None;
            }
            let mut pid = 0;
            GetWindowThreadProcessId(w, &mut pid);
            let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if p.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(p, 0, buf.as_mut_ptr(), &mut len);
            CloseHandle(p);
            if ok == 0 {
                return None;
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            path.rsplit('\\').next().map(|s| s.to_string())
        }
    }

    fn reg(args: &[&str]) -> bool {
        Command::new("reg").args(args).creation_flags(CREATE_NO_WINDOW).output().is_ok_and(|o| o.status.success())
    }

    pub fn startup_enabled() -> bool {
        reg(&["query", RUN_KEY, "/v", "M811"])
    }

    pub fn set_startup(on: bool) -> Result<(), String> {
        let ok = if on {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            reg(&["add", RUN_KEY, "/v", "M811", "/t", "REG_SZ", "/d", &format!("\"{}\" tray", exe.display()), "/f"])
        } else {
            reg(&["delete", RUN_KEY, "/v", "M811", "/f"])
        };
        if ok {
            Ok(())
        } else {
            Err("reg command failed".into())
        }
    }

    struct State {
        nid: NotifyIconData,
        watcher: games::Watcher,
    }

    thread_local! {
        static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    }

    fn set_tip(st: &mut State, text: &str) {
        let t = wide(&format!("M811: {text}"));
        let n = t.len().min(127);
        st.nid.tip = [0; 128];
        st.nid.tip[..n].copy_from_slice(&t[..n]);
        unsafe { Shell_NotifyIconW(NIM_MODIFY, &mut st.nid) };
    }

    fn tick(st: &mut State) {
        let Some(exe) = foreground_exe() else { return };
        let rules = match games::parse_rules(&games::load_text()) {
            Ok(r) => r,
            Err(e) => return set_tip(st, &format!("games.cfg {e}")),
        };
        let Some((name, cmds)) = st.watcher.tick(&rules, &exe) else { return };
        if cmds.is_empty() {
            return set_tip(st, &name);
        }
        let r = Mouse::open().and_then(|m| cmds.iter().try_for_each(|c| cli::run(&m, c)));
        match r {
            Ok(()) => set_tip(st, &name),
            Err(e) => set_tip(st, &format!("{name}: {e}")),
        }
    }

    fn spawn(args: &[&str]) {
        if let Ok(exe) = std::env::current_exe() {
            let _ = Command::new(exe).args(args).spawn();
        }
    }

    fn menu(w: H) {
        unsafe {
            let m = CreatePopupMenu();
            let item = |id, flags, text: &str| AppendMenuW(m, flags, id, wide(text).as_ptr());
            item(ID_OPEN, MF_STRING, "Open M811");
            item(ID_EDIT, MF_STRING, "Edit game profiles...");
            let checked = if startup_enabled() { MF_CHECKED } else { 0 };
            item(ID_STARTUP, MF_STRING | checked, "Start with Windows");
            AppendMenuW(m, MF_SEPARATOR, 0, null_mut());
            item(ID_QUIT, MF_STRING, "Quit");
            let mut p = Point::default();
            GetCursorPos(&mut p);
            SetForegroundWindow(w);
            let id = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON, p.x, p.y, 0, w, null_mut()) as usize;
            DestroyMenu(m);
            match id {
                ID_OPEN => spawn(&[]),
                ID_EDIT => {
                    if !games::path().exists() {
                        let _ = games::save_text(games::EXAMPLE);
                    }
                    let _ = Command::new("notepad").arg(games::path()).spawn();
                }
                ID_STARTUP => {
                    let _ = set_startup(checked == 0);
                }
                ID_QUIT => PostQuitMessage(0),
                _ => {}
            }
        }
    }

    extern "system" fn wndproc(w: H, msg: u32, wp: usize, lp: isize) -> isize {
        match msg {
            WM_TIMER => STATE.with(|s| {
                if let Some(st) = s.borrow_mut().as_mut() {
                    tick(st);
                }
            }),
            WM_APP_TRAY => match lp as u32 & 0xFFFF {
                WM_RBUTTONUP => menu(w),
                WM_LBUTTONDBLCLK => spawn(&[]),
                _ => {}
            },
            _ => return unsafe { DefWindowProcW(w, msg, wp, lp) },
        }
        0
    }

    pub fn run() -> Result<(), String> {
        unsafe {
            CreateMutexW(null_mut(), 0, wide("m811-tray").as_ptr());
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return Err("tray is already running".into());
            }
            let inst = GetModuleHandleW(null_mut());
            let class = wide("m811tray");
            let wc = WndClass {
                style: 0,
                proc_: wndproc,
                cls_extra: 0,
                wnd_extra: 0,
                instance: inst,
                icon: null_mut(),
                cursor: null_mut(),
                background: null_mut(),
                menu_name: null_mut(),
                class_name: class.as_ptr(),
            };
            RegisterClassW(&wc);
            // Hidden top-level window: gets tray and timer messages.
            let w = CreateWindowExW(0, class.as_ptr(), class.as_ptr(), 0, 0, 0, 0, 0, null_mut(), null_mut(), inst, null_mut());
            if w.is_null() {
                return Err("could not create tray window".into());
            }
            let mut nid: NotifyIconData = std::mem::zeroed();
            nid.size = std::mem::size_of::<NotifyIconData>() as u32;
            nid.wnd = w;
            nid.id = 1;
            nid.flags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.callback_msg = WM_APP_TRAY;
            nid.icon = LoadIconW(null_mut(), 32512 as *const u16); // IDI_APPLICATION
            let tip = wide("M811");
            nid.tip[..tip.len()].copy_from_slice(&tip);
            if Shell_NotifyIconW(NIM_ADD, &mut nid) == 0 {
                return Err("could not add tray icon".into());
            }
            STATE.with(|s| *s.borrow_mut() = Some(State { nid, watcher: games::Watcher::default() }));
            SetTimer(w, 1, 2000, null_mut());
            let mut msg: Msg = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            STATE.with(|s| {
                if let Some(st) = s.borrow_mut().as_mut() {
                    Shell_NotifyIconW(NIM_DELETE, &mut st.nid);
                }
            });
        }
        Ok(())
    }
}

pub use imp::{run, set_startup, startup_enabled};
