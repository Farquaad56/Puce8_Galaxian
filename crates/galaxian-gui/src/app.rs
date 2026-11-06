//! Application egui : affiche le framebuffer 768x224 (avec rotation) et fait tourner la machine en temps réel.

use crate::audio::AudioOut;
use crate::debugger::{Cmd, Debugger};
use crate::keys::{to_inputs, Pressed};
use crate::pacing::Pacer;
use crate::settings::{Action, Dip, Rotation, Settings};
use eframe::egui;
use galaxian_core::{Machine, RomSet};
use std::time::Instant;

const W: usize = 768;
const H: usize = 224;
/// Rapport largeur/hauteur de l'image brute avec des pixels carrés : 256 x 224.
const RAW_ASPECT: f32 = 256.0 / 224.0;

pub struct GalaxianApp {
    rom: RomSet,
    machine: Machine,
    pacer: Pacer,
    last: Instant,
    texture: Option<egui::TextureHandle>,
    frames: u64,
    audio: Option<AudioOut>,
    settings: Settings,
    saved: Settings,
    show_settings: bool,
    capturing: Option<Action>,
    dbg: Debugger,
}

impl GalaxianApp {
    pub fn new(rom: RomSet, machine: Machine) -> GalaxianApp {
        let settings = Settings::load();
        GalaxianApp {
            rom,
            machine,
            pacer: Pacer::new(),
            last: Instant::now(),
            texture: None,
            frames: 0,
            audio: AudioOut::new(),
            saved: settings.clone(),
            settings,
            show_settings: false,
            capturing: None,
            dbg: Debugger::new(),
        }
    }

    fn read_keys(&self, ctx: &egui::Context) -> Pressed {
        let k = &self.settings.keys;
        ctx.input(|i| Pressed {
            coin: i.key_down(k.get(Action::Coin)),
            start1: i.key_down(k.get(Action::Start1)),
            start2: i.key_down(k.get(Action::Start2)),
            left: i.key_down(k.get(Action::Left)),
            right: i.key_down(k.get(Action::Right)),
            fire: i.key_down(k.get(Action::Fire)),
        })
    }

    fn framebuffer_image(&self) -> egui::ColorImage {
        let mut rgb = Vec::with_capacity(W * H * 3);
        for &p in &self.machine.board.video.fb {
            rgb.push((p >> 16) as u8);
            rgb.push((p >> 8) as u8);
            rgb.push(p as u8);
        }
        egui::ColorImage::from_rgb([W, H], &rgb)
    }

    /// Touche pressée pendant une capture de rebinding (Échap annule).
    fn captured_key(ctx: &egui::Context) -> Option<egui::Key> {
        ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    ..
                } => Some(*key),
                _ => None,
            })
        })
    }

    fn restart(&mut self) {
        self.machine = Machine::new(&self.rom);
        self.pacer = Pacer::new();
        self.frames = 0;
    }

    fn draw_screen(&self, ui: &mut egui::Ui) {
        let Some(t) = &self.texture else {
            return;
        };
        let rect = ui.available_rect_before_wrap();
        let rot = self.settings.rotation;
        let aspect = if rot.swaps_axes() {
            1.0 / RAW_ASPECT
        } else {
            RAW_ASPECT
        };
        let mut size = egui::vec2(rect.width(), rect.width() / aspect);
        if size.y > rect.height() {
            size = egui::vec2(rect.height() * aspect, rect.height());
        }
        let dest = egui::Rect::from_center_size(rect.center(), size);
        let corners = [
            dest.left_top(),
            dest.right_top(),
            dest.right_bottom(),
            dest.left_bottom(),
        ];
        let mut mesh = egui::Mesh::with_texture(t.id());
        for (pos, (u, v)) in corners.into_iter().zip(rot.uvs()) {
            mesh.vertices.push(egui::epaint::Vertex {
                pos,
                uv: egui::pos2(u, v),
                color: egui::Color32::WHITE,
            });
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        ui.painter().add(egui::Shape::mesh(mesh));
        self.draw_scanlines(ui, dest, rot.swaps_axes());
    }

    /// Scanlines : bande sombre sur le bas de chaque ligne de balayage (les lignes brutes deviennent
    /// des colonnes quand l'écran est tourné de 90°). Ignorées si les lignes sont trop serrées.
    fn draw_scanlines(&self, ui: &egui::Ui, dest: egui::Rect, vertical: bool) {
        if self.settings.scanlines == 0 {
            return;
        }
        let extent = if vertical {
            dest.width()
        } else {
            dest.height()
        };
        let step = extent / H as f32;
        if step < 2.0 {
            return;
        }
        let alpha = (f32::from(self.settings.scanlines) / 100.0 * 200.0) as u8;
        let color = egui::Color32::from_black_alpha(alpha);
        let painter = ui.painter();
        for k in 0..H {
            let a = (k as f32 + 0.6) * step;
            let b = (k as f32 + 1.0) * step;
            let r = if vertical {
                egui::Rect::from_min_max(
                    egui::pos2(dest.left() + a, dest.top()),
                    egui::pos2(dest.left() + b, dest.bottom()),
                )
            } else {
                egui::Rect::from_min_max(
                    egui::pos2(dest.left(), dest.top() + a),
                    egui::pos2(dest.right(), dest.top() + b),
                )
            };
            painter.rect_filled(r, 0.0, color);
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_settings;
        let settings = &mut self.settings;
        let capturing = &mut self.capturing;
        let mut restart = false;
        egui::Window::new("Réglages")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Affichage");
                egui::ComboBox::from_label("Rotation")
                    .selected_text(settings.rotation.label())
                    .show_ui(ui, |ui| {
                        for r in Rotation::ALL {
                            ui.selectable_value(&mut settings.rotation, r, r.label());
                        }
                    });
                ui.add(
                    egui::Slider::new(&mut settings.scanlines, 0..=100)
                        .text("Scanlines (effet cathodique)")
                        .suffix(" %"),
                );
                ui.separator();
                ui.heading("Son");
                ui.add(
                    egui::Slider::new(&mut settings.volume, 0..=100)
                        .text("Volume")
                        .suffix(" %"),
                );
                ui.separator();
                ui.heading("Touches");
                egui::Grid::new("keys").show(ui, |ui| {
                    for a in Action::ALL {
                        ui.label(a.label());
                        let text = if *capturing == Some(a) {
                            "Appuyez sur une touche… (Échap : annuler)".to_owned()
                        } else {
                            settings.keys.get(a).name().to_owned()
                        };
                        if ui.button(text).clicked() {
                            *capturing = Some(a);
                        }
                        ui.end_row();
                    }
                });
                if ui.button("Touches par défaut").clicked() {
                    settings.keys = Settings::default().keys;
                    *capturing = None;
                }
                ui.separator();
                ui.heading("Interrupteurs DIP");
                egui::ComboBox::from_label("Vie bonus à")
                    .selected_text(Dip::BONUS_LABELS[usize::from(settings.dip.bonus & 3)])
                    .show_ui(ui, |ui| {
                        for (i, l) in Dip::BONUS_LABELS.iter().enumerate() {
                            ui.selectable_value(&mut settings.dip.bonus, i as u8, *l);
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label("Vies");
                    ui.radio_value(&mut settings.dip.three_lives, false, "2");
                    ui.radio_value(&mut settings.dip.three_lives, true, "3");
                });
                egui::ComboBox::from_label("Pièces")
                    .selected_text(Dip::COINAGE_LABELS[usize::from(settings.dip.coinage & 3)])
                    .show_ui(ui, |ui| {
                        for (i, l) in Dip::COINAGE_LABELS.iter().enumerate() {
                            ui.selectable_value(&mut settings.dip.coinage, i as u8, *l);
                        }
                    });
                ui.checkbox(&mut settings.dip.cocktail, "Cabinet cocktail");
                ui.checkbox(&mut settings.dip.service_mode, "Mode service");
                ui.label("Les DIP sont lus en direct ; redémarrez pour un effet complet.");
                if ui.button("Redémarrer la machine").clicked() {
                    restart = true;
                }
            });
        self.show_settings = open;
        if restart {
            self.restart();
        }
    }
}

impl eframe::App for GalaxianApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Capture d'une touche pour le rebinding : elle ne doit pas être vue par le jeu.
        if let Some(action) = self.capturing {
            if let Some(key) = Self::captured_key(ctx) {
                if key != egui::Key::Escape {
                    self.settings.keys.set(action, key);
                }
                self.capturing = None;
            }
        } else if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.show_settings = !self.show_settings;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F2)) {
            self.dbg.show_cpu = !self.dbg.show_cpu;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F3)) {
            self.dbg.show_mem = !self.dbg.show_mem;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F10)) {
            self.dbg.request(Cmd::StepInsn);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            self.dbg.request(Cmd::StepFrame);
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last).as_nanos() as u64;
        self.last = now;

        let mut inputs = if self.capturing.is_some() {
            to_inputs(&Pressed::default())
        } else {
            to_inputs(&self.read_keys(ctx))
        };
        self.settings.dip.apply(&mut inputs);
        self.machine.board.inputs = inputs;
        let due = self.pacer.frames_due(dt);
        let cmd = self.dbg.take_cmd();
        if self.dbg.paused {
            match cmd {
                Cmd::StepInsn => {
                    self.machine.step();
                }
                Cmd::StepFrame => {
                    self.machine.run_frame();
                    self.frames += 1;
                }
                Cmd::None => {}
            }
            self.machine.board.samples.clear();
        } else {
            for _ in 0..due {
                let hit = run_frame_until_break(&mut self.machine, self.dbg.breakpoints());
                if let Some(a) = &self.audio {
                    let g = f32::from(self.settings.volume) / 100.0;
                    let g = g * g;
                    let scaled: Vec<f32> =
                        self.machine.board.samples.iter().map(|s| s * g).collect();
                    a.push(&scaled);
                }
                self.machine.board.samples.clear();
                self.frames += 1;
                if let Some(pc) = hit {
                    self.dbg.hit_breakpoint(pc);
                    break;
                }
            }
        }

        let image = self.framebuffer_image();
        match &mut self.texture {
            Some(t) => t.set(image, egui::TextureOptions::NEAREST),
            None => {
                self.texture =
                    Some(ctx.load_texture("framebuffer", image, egui::TextureOptions::NEAREST));
            }
        }

        egui::TopBottomPanel::top("barre").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Réglages (F1)").clicked() {
                    self.show_settings = !self.show_settings;
                }
                if ui.button("Redémarrer").clicked() {
                    self.restart();
                }
                if ui.button("CPU (F2)").clicked() {
                    self.dbg.show_cpu = !self.dbg.show_cpu;
                }
                if ui.button("Mémoire (F3)").clicked() {
                    self.dbg.show_mem = !self.dbg.show_mem;
                }
                if self.dbg.paused {
                    ui.colored_label(egui::Color32::YELLOW, "PAUSE");
                }
                ui.label("Échap : quitter");
            });
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |ui| self.draw_screen(ui));
        self.settings_window(ctx);
        self.dbg.ui(ctx, &self.machine, &self.rom, self.frames);

        if self.settings != self.saved {
            self.settings.save();
            self.saved = self.settings.clone();
        }
        ctx.request_repaint();
    }
}

/// Exécute une image ; avec des points d'arrêt, s'arrête dès que PC en atteint un (retourne son adresse).
fn run_frame_until_break(machine: &mut Machine, breakpoints: &[u16]) -> Option<u16> {
    if breakpoints.is_empty() {
        machine.run_frame();
        return None;
    }
    machine.board.vblank_flag = false;
    while !machine.board.vblank_flag {
        machine.step();
        let pc = machine.cpu.regs.pc;
        if breakpoints.contains(&pc) {
            return Some(pc);
        }
    }
    None
}
