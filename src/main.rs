// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod buttons;
mod cli;
mod games;
mod mouse;
mod proto;
mod tray;

use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};
use mouse::{Mouse, Status};
use proto::Effect;

const ACCENT: Color32 = Color32::from_rgb(0xE5, 0x39, 0x35);
const BG: Color32 = Color32::from_rgb(0x14, 0x15, 0x18);
const PANEL: Color32 = Color32::from_rgb(0x1C, 0x1D, 0x21);
const CARD: Color32 = Color32::from_rgb(0x25, 0x26, 0x2B);
const CHIP: Color32 = Color32::from_rgb(0x32, 0x33, 0x3A);
const MUTED: Color32 = Color32::from_rgb(0x9A, 0x9C, 0xA3);
const OK: Color32 = Color32::from_rgb(0x4C, 0xC2, 0x6A);
const PRESETS: [[u8; 3]; 8] = [
    [255, 0, 0],
    [255, 128, 0],
    [255, 255, 0],
    [0, 255, 0],
    [0, 255, 255],
    [0, 0, 255],
    [128, 0, 255],
    [255, 255, 255],
];

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        attach_console();
        std::process::exit(cli::main(&args));
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([860.0, 600.0]).with_min_inner_size([700.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "M811",
        options,
        Box::new(|cc| {
            style(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}

/// Release builds have no console. Borrow the parent's so CLI output shows.
#[cfg(windows)]
fn attach_console() {
    extern "system" {
        fn AttachConsole(pid: u32) -> i32;
    }
    unsafe {
        AttachConsole(u32::MAX);
    }
}

#[cfg(not(windows))]
fn attach_console() {}

fn style(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::dark());
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8.0, 8.0);
        s.spacing.button_padding = Vec2::new(12.0, 6.0);
        s.visuals.panel_fill = BG;
        s.visuals.window_fill = PANEL;
        s.visuals.selection.bg_fill = ACCENT;
        s.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
        s.visuals.hyperlink_color = ACCENT;
        for w in [&mut s.visuals.widgets.inactive, &mut s.visuals.widgets.hovered, &mut s.visuals.widgets.active] {
            w.corner_radius = CornerRadius::same(6);
        }
        s.visuals.widgets.inactive.weak_bg_fill = CHIP;
        s.visuals.widgets.inactive.bg_fill = CHIP;
        s.visuals.extreme_bg_color = Color32::from_rgb(0x10, 0x10, 0x13);
        s.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(22.0));
        s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
        s.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(12.5));
        s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(15.0));
    });
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Lighting,
    Dpi,
    Buttons,
    Macro,
    Performance,
    Games,
}

impl Page {
    const ALL: [(Page, &'static str, &'static str); 6] = [
        (Page::Lighting, "Lighting", "Effects and color"),
        (Page::Dpi, "DPI", "Sensitivity stages"),
        (Page::Buttons, "Buttons", "Side key mapping"),
        (Page::Macro, "Macro", "Record key taps"),
        (Page::Performance, "Performance", "Polling rate"),
        (Page::Games, "Games", "Per-game profiles, tray"),
    ];
}

struct App {
    page: Page,
    msg: String,
    err: bool,
    status: Option<Status>,
    effect: Effect,
    color: [u8; 3],
    keys: [String; 8],
    macro_side: usize,
    macro_spec: String,
    games_text: String,
    startup: bool,
}

impl App {
    fn new() -> App {
        let mut app = App {
            page: Page::Lighting,
            msg: String::new(),
            err: false,
            status: None,
            effect: Effect::Wave,
            color: [0, 255, 0],
            keys: Default::default(),
            macro_side: 1,
            macro_spec: "a,50,b".into(),
            games_text: games::load_text(),
            startup: tray::startup_enabled(),
        };
        app.refresh();
        app.load_led();
        app
    }

    fn profile(&self) -> usize {
        self.status.as_ref().map_or(1, |s| s.profile.clamp(1, 5) as usize)
    }

    /// Copy the active profile's lighting into the editor.
    fn load_led(&mut self) {
        if let Some(s) = &self.status {
            let rec = s.leds[self.profile() - 1];
            if let Some(e) = Effect::from_record(&rec) {
                self.effect = e;
            }
            if self.effect.needs_color() {
                self.color = [rec[0], rec[1], rec[2]];
            }
        }
    }

    fn refresh(&mut self) {
        match Mouse::open().and_then(|m| m.status()) {
            Ok(s) => {
                self.status = Some(s);
                (self.msg, self.err) = ("Connected".into(), false);
            }
            Err(e) => {
                self.status = None;
                (self.msg, self.err) = (e, true);
            }
        }
    }

    /// Open the mouse, run one command, show the result, read the mouse again.
    fn run(&mut self, c: cli::Cmd) {
        let what = format!("{c:?}");
        let r = Mouse::open().and_then(|m| cli::run(&m, &c));
        self.refresh();
        (self.msg, self.err) = match r {
            Ok(()) => (format!("Saved: {what}"), false),
            Err(e) => (format!("{what}: {e}"), true),
        };
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(16, 12)))
            .show(ui, |ui| self.header(ui));
        egui::Panel::bottom("status")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(16, 6)))
            .show(ui, |ui| {
                let c = if self.err { ACCENT } else { MUTED };
                ui.label(RichText::new(&self.msg).color(c).small());
            });
        egui::Panel::left("nav")
            .resizable(false)
            .exact_size(200.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::same(12)))
            .show(ui, |ui| self.nav(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::same(20)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| match self.page {
                    Page::Lighting => self.lighting(ui),
                    Page::Dpi => self.dpi(ui),
                    Page::Buttons => self.buttons(ui),
                    Page::Macro => self.macro_page(ui),
                    Page::Performance => self.performance(ui),
                    Page::Games => self.games(ui),
                });
            });
    }
}

fn card(ui: &mut egui::Ui, title: &str, note: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(CARD).corner_radius(10).inner_margin(16).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).strong().size(17.0));
        if !note.is_empty() {
            ui.label(RichText::new(note).color(MUTED).small());
        }
        ui.add_space(6.0);
        body(ui);
    });
    ui.add_space(12.0);
}

/// Button that looks selected when `on`.
fn pill(ui: &mut egui::Ui, text: &str, on: bool) -> bool {
    let b = egui::Button::new(RichText::new(text).color(if on { Color32::WHITE } else { MUTED }))
        .fill(if on { ACCENT } else { CHIP })
        .corner_radius(16)
        .min_size(Vec2::new(44.0, 30.0));
    ui.add(b).clicked()
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

impl App {
    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("AATROX M811").strong().size(20.0).color(Color32::WHITE));
            let (dot, text) = if self.status.is_some() { (OK, "connected") } else { (ACCENT, "not found") };
            ui.label(RichText::new("●").color(dot));
            ui.label(RichText::new(text).color(MUTED));
            ui.add_space(24.0);
            ui.label(RichText::new("Profile").color(MUTED));
            let active = self.profile();
            for n in 1..=5 {
                if pill(ui, &n.to_string(), self.status.is_some() && n == active) {
                    self.run(cli::Cmd::Profile(n));
                    self.load_led();
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Unstick").on_hover_text("Leave config mode. Use if the mouse stops moving.").clicked() {
                    let r = Mouse::open().and_then(|m| m.unstick());
                    (self.msg, self.err) = match r {
                        Ok(()) => ("Sent config mode off".into(), false),
                        Err(e) => (e, true),
                    };
                }
                if ui.button("⟳ Refresh").clicked() {
                    self.refresh();
                    self.load_led();
                }
            });
        });
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        for (p, name, sub) in Page::ALL {
            let on = self.page == p;
            let r = egui::Frame::new()
                .fill(if on { CARD } else { PANEL })
                .stroke(if on { Stroke::new(1.0, ACCENT) } else { Stroke::NONE })
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(name).strong().color(if on { Color32::WHITE } else { MUTED }));
                    ui.label(RichText::new(sub).small().color(MUTED));
                })
                .response
                .interact(egui::Sense::click());
            if r.clicked() {
                self.page = p;
            }
        }
    }

    fn lighting(&mut self, ui: &mut egui::Ui) {
        let p = self.profile();
        card(ui, "Effect", &format!("Applies to profile {p}"), |ui| {
            ui.horizontal_wrapped(|ui| {
                for e in Effect::ALL {
                    if pill(ui, e.name(), self.effect == e) {
                        self.effect = e;
                    }
                }
            });
        });
        if self.effect.needs_color() {
            card(ui, "Color", "", |ui| {
                ui.horizontal(|ui| {
                    let (r, painter) = ui.allocate_painter(Vec2::new(64.0, 64.0), egui::Sense::hover());
                    painter.rect_filled(r.rect, 12.0, rgb(self.color));
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            for c in PRESETS {
                                let b = egui::Button::new("").fill(rgb(c)).corner_radius(14).min_size(Vec2::splat(28.0));
                                let b = if c == self.color { b.stroke(Stroke::new(2.0, Color32::WHITE)) } else { b };
                                if ui.add(b).clicked() {
                                    self.color = c;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Custom").color(MUTED));
                            ui.color_edit_button_srgb(&mut self.color);
                            ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}", self.color[0], self.color[1], self.color[2])).monospace());
                        });
                    });
                });
            });
        }
        let apply = egui::Button::new(RichText::new("Apply lighting").color(Color32::WHITE).strong())
            .fill(ACCENT)
            .min_size(Vec2::new(160.0, 36.0));
        if ui.add(apply).clicked() {
            self.run(cli::Cmd::Led(p, self.effect, self.color));
        }
    }

    fn dpi(&mut self, ui: &mut egui::Ui) {
        let codes = self.status.as_ref().map(|s| (s.dpi, s.dpi_flags));
        card(ui, "DPI stages", "Profile 1. Click to save. Any 200-6200 step 100 works from the command line.", |ui| {
            for stage in 1..=5 {
                let cur = codes.filter(|(_, f)| f[stage - 1] == 0).and_then(|(c, _)| proto::dpi_from_code(c[stage - 1]));
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Stage {stage}")).strong());
                    let now = cur.map_or("?".into(), |d| d.to_string());
                    ui.label(RichText::new(format!("{now:>4}")).monospace().color(MUTED));
                    ui.add_space(12.0);
                    for d in proto::DPI_PRESETS {
                        if pill(ui, &d.to_string(), cur == Some(d)) && cur != Some(d) {
                            self.run(cli::Cmd::Dpi(stage, d));
                        }
                    }
                });
            }
        });
    }

    fn buttons(&mut self, ui: &mut egui::Ui) {
        let keys = self.status.as_ref().map(|s| s.side_keys);
        card(
            ui,
            "Side keys",
            "Profile 1. Pick a function (same list as the vendor app), or type a key or combo like f5 or ctrl+shift+t. Keys holding your slot 0 macro are locked.",
            |ui| {
                egui::Grid::new("keys").num_columns(5).spacing([16.0, 10.0]).show(ui, |ui| {
                    for side in 1..=8 {
                        let cur = keys.map(|k| k[side - 1]);
                        ui.label(RichText::new(format!("Key {side}")).strong());
                        let text = cur.map_or("?".into(), |e| {
                            buttons::label(&e).map_or_else(|| proto::describe_entry(&e), String::from)
                        });
                        ui.label(RichText::new(text).color(MUTED));
                        if cur.is_some_and(|e| proto::is_slot0_macro(&e)) {
                            ui.label(RichText::new("🔒 locked").color(MUTED));
                            ui.label("");
                            ui.label("");
                        } else {
                            let mut pick = None;
                            ui.menu_button("Function ▾", |ui| {
                                for g in buttons::GROUPS {
                                    ui.menu_button(g, |ui| {
                                        for p in buttons::PRESETS.iter().filter(|p| p.group == g) {
                                            if ui.button(p.label).clicked() {
                                                pick = Some(p.word);
                                                ui.close();
                                            }
                                        }
                                    });
                                }
                            });
                            if let Some(w) = pick {
                                self.run(cli::Cmd::Key(side, w.into()));
                            }
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.keys[side - 1]).hint_text("key or combo").desired_width(110.0));
                                if ui.button("Set").clicked() {
                                    match cli::parse(&format!("key {side} {}", self.keys[side - 1])) {
                                        Ok(Some(c)) => self.run(c),
                                        Ok(None) => {}
                                        Err(e) => (self.msg, self.err) = (e, true),
                                    }
                                }
                            });
                        }
                        ui.end_row();
                    }
                });
            },
        );
    }

    fn macro_page(&mut self, ui: &mut egui::Ui) {
        card(ui, "Macro slot 1", "Letters are key taps, numbers are waits in ms. Example: a,50,b. Slot 0 is never written.", |ui| {
            ui.horizontal(|ui| {
                ui.label("Side key");
                egui::ComboBox::from_id_salt("macro_side").selected_text(self.macro_side.to_string()).show_ui(ui, |ui| {
                    for n in 1..=8 {
                        ui.selectable_value(&mut self.macro_side, n, n.to_string());
                    }
                });
                ui.add(egui::TextEdit::singleline(&mut self.macro_spec).desired_width(240.0).font(egui::TextStyle::Monospace));
                if ui.button("Write").clicked() {
                    match cli::parse(&format!("macro {} {}", self.macro_side, self.macro_spec)) {
                        Ok(Some(c)) => self.run(c),
                        Ok(None) => {}
                        Err(e) => (self.msg, self.err) = (e, true),
                    }
                }
            });
        });
    }

    fn performance(&mut self, ui: &mut egui::Ui) {
        let cur = self.status.as_ref().and_then(|s| s.poll_hz);
        card(ui, "Polling rate", "Profile 1. Click to save.", |ui| {
            ui.horizontal(|ui| {
                for hz in [500, 1000] {
                    if pill(ui, &format!("{hz} Hz"), cur == Some(hz)) && cur != Some(hz) {
                        self.run(cli::Cmd::Poll(hz));
                    }
                }
            });
        });
        card(ui, "Command line", "Same settings from a terminal or a script. See AGENTS.md.", |ui| {
            ui.label(RichText::new("m811 status > my.cfg\nm811 apply my.cfg").monospace());
        });
    }
}

impl App {
    fn games(&mut self, ui: &mut egui::Ui) {
        card(ui, "Per-game profiles", "The tray app watches which program is in front and runs its block.", |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.games_text)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(14)
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    (self.msg, self.err) = match games::save_text(&self.games_text) {
                        Ok(()) => (format!("Saved {}", games::path().display()), false),
                        Err(e) => (format!("games.cfg {e}"), true),
                    };
                }
                ui.label(RichText::new(games::path().display().to_string()).small().color(MUTED));
            });
        });
        card(ui, "Tray", "Runs in the background so game profiles switch on their own.", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Start tray now").clicked() {
                    match std::env::current_exe().and_then(|e| std::process::Command::new(e).arg("tray").spawn()) {
                        Ok(_) => (self.msg, self.err) = ("Tray started".into(), false),
                        Err(e) => (self.msg, self.err) = (e.to_string(), true),
                    }
                }
                let before = self.startup;
                ui.checkbox(&mut self.startup, "Start with Windows");
                if self.startup != before {
                    if let Err(e) = tray::set_startup(self.startup) {
                        self.startup = before;
                        (self.msg, self.err) = (e, true);
                    }
                }
            });
        });
    }
}
