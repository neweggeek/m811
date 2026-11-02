// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mouse;
mod proto;

use eframe::egui;
use mouse::{Mouse, Status};
use proto::Effect;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([520.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native("M811", options, Box::new(|_| Ok(Box::new(App::new()))))
}

struct App {
    msg: String,
    status: Option<Status>,
    profile: usize,
    led_profile: usize,
    effect: Effect,
    color: [u8; 3],
    poll_hz: u32,
    dpi: [u32; 5],
    keys: [String; 8],
    macro_side: usize,
    macro_spec: String,
}

impl App {
    fn new() -> App {
        let mut app = App {
            msg: String::new(),
            status: None,
            profile: 1,
            led_profile: 1,
            effect: Effect::Wave,
            color: [0, 255, 0],
            poll_hz: 1000,
            dpi: [1000; 5],
            keys: Default::default(),
            macro_side: 1,
            macro_spec: "a,50,b".into(),
        };
        app.refresh();
        app
    }

    /// Read the mouse and copy its values into the controls.
    fn refresh(&mut self) {
        match Mouse::open().and_then(|m| m.status()) {
            Ok(s) => {
                self.profile = s.profile.clamp(1, 5) as usize;
                self.led_profile = self.profile;
                let rec = s.leds[self.led_profile - 1];
                if let Some(e) = Effect::from_record(&rec) {
                    self.effect = e;
                }
                self.color = [rec[0], rec[1], rec[2]];
                if let Some(hz) = s.poll_hz {
                    self.poll_hz = hz;
                }
                for (i, &c) in s.dpi.iter().enumerate() {
                    if let Some(d) = proto::dpi_from_code(c) {
                        self.dpi[i] = d;
                    }
                }
                self.status = Some(s);
                self.msg = "Read mouse OK".into();
            }
            Err(e) => {
                self.status = None;
                self.msg = e;
            }
        }
    }

    /// Open the mouse, run f, show the result, read the mouse again.
    fn run(&mut self, what: &str, f: impl FnOnce(&Mouse) -> Result<(), String>) {
        let r = Mouse::open().and_then(|m| f(&m));
        let msg = match r {
            Ok(()) => format!("{what}: done"),
            Err(e) => format!("{what}: {e}"),
        };
        self.refresh();
        self.msg = msg;
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::bottom("msg").show(ui, |ui| {
            ui.label(&self.msg);
        });
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| self.body(ui));
        });
    }
}

impl App {
    fn body(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Read mouse").clicked() {
                self.refresh();
            }
            if ui.button("Unstick").on_hover_text("Send config mode off. Use if the mouse stops moving.").clicked() {
                self.run("Unstick", |m| m.unstick());
            }
        });

        ui.separator();
        ui.heading("Profile");
        ui.horizontal(|ui| {
            for n in 1..=5 {
                ui.radio_value(&mut self.profile, n, n.to_string());
            }
            if ui.button("Switch").clicked() {
                let n = self.profile;
                self.run(&format!("Profile {n}"), |m| m.send_all(&proto::profile(n)));
            }
        });

        ui.separator();
        ui.heading("Lighting");
        ui.horizontal(|ui| {
            ui.label("Profile");
            egui::ComboBox::from_id_salt("led_profile")
                .selected_text(self.led_profile.to_string())
                .show_ui(ui, |ui| {
                    for n in 1..=5 {
                        ui.selectable_value(&mut self.led_profile, n, n.to_string());
                    }
                });
            egui::ComboBox::from_id_salt("effect").selected_text(self.effect.name()).show_ui(ui, |ui| {
                for e in Effect::ALL {
                    ui.selectable_value(&mut self.effect, e, e.name());
                }
            });
            if self.effect.needs_color() {
                ui.color_edit_button_srgb(&mut self.color);
            }
            if ui.button("Apply").clicked() {
                let (p, e, c) = (self.led_profile, self.effect, self.color);
                self.run(&format!("Lighting {}", e.name()), |m| m.send_all(&proto::led(p, e, c, &m.leds()?)));
            }
        });

        ui.separator();
        ui.heading("Poll rate (profile 1)");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.poll_hz, 500, "500 Hz");
            ui.radio_value(&mut self.poll_hz, 1000, "1000 Hz");
            if ui.button("Apply").clicked() {
                let hz = self.poll_hz;
                self.run(&format!("Poll {hz} Hz"), |m| m.send_all(&proto::poll(hz)));
            }
        });

        ui.separator();
        ui.heading("DPI stages (profile 1)");
        ui.label("Only values seen from the vendor app. Formula unknown.");
        for stage in 1..=5 {
            ui.horizontal(|ui| {
                ui.label(format!("Stage {stage}"));
                egui::ComboBox::from_id_salt(("dpi", stage)).selected_text(self.dpi[stage - 1].to_string()).show_ui(ui, |ui| {
                    for (d, _) in proto::DPI_VALUES {
                        ui.selectable_value(&mut self.dpi[stage - 1], d, d.to_string());
                    }
                });
                if ui.button("Set").clicked() {
                    let d = self.dpi[stage - 1];
                    self.run(&format!("DPI stage {stage} = {d}"), |m| {
                        m.send_all(&proto::dpi(stage, d).ok_or("unknown DPI value")?)
                    });
                }
            });
        }

        ui.separator();
        ui.heading("Side keys (profile 1)");
        ui.label("New key: a letter, a digit, or volup.");
        for side in 1..=8 {
            let current = self.status.as_ref().map(|s| s.side_keys[side - 1]);
            let locked = current.is_some_and(|e| proto::is_slot0_macro(&e));
            ui.horizontal(|ui| {
                ui.label(format!("Key {side}:"));
                ui.label(current.map(|e| proto::describe_entry(&e)).unwrap_or("?".into()));
                if locked {
                    ui.label("(locked)");
                    return;
                }
                ui.add(egui::TextEdit::singleline(&mut self.keys[side - 1]).desired_width(60.0));
                if ui.button("Set").clicked() {
                    let k = self.keys[side - 1].clone();
                    self.run(&format!("Side key {side} = {k}"), |m| {
                        let entry = proto::key_entry(&k).ok_or("use a letter, a digit, or volup")?;
                        m.guard_side_key(side)?;
                        m.send_all(&proto::key(side, entry))
                    });
                }
            });
        }

        ui.separator();
        ui.heading("Macro (slot 1)");
        ui.label("Letters are key taps, numbers are ms waits. Example: a,50,b");
        ui.horizontal(|ui| {
            ui.label("Side key");
            egui::ComboBox::from_id_salt("macro_side").selected_text(self.macro_side.to_string()).show_ui(ui, |ui| {
                for n in 1..=8 {
                    ui.selectable_value(&mut self.macro_side, n, n.to_string());
                }
            });
            ui.text_edit_singleline(&mut self.macro_spec);
            if ui.button("Write").clicked() {
                let (side, spec) = (self.macro_side, self.macro_spec.clone());
                self.run(&format!("Macro on side key {side}"), |m| {
                    let packets = proto::macro_(side, &spec)?;
                    m.guard_side_key(side)?;
                    m.send_all(&packets)
                });
            }
        });
    }
}
