//! Application egui : affiche le framebuffer 768x224 et fait tourner la machine en temps réel.

use crate::keys::{to_inputs, Pressed};
use crate::pacing::Pacer;
use eframe::egui;
use galaxian_core::Machine;
use std::time::Instant;

const W: usize = 768;
const H: usize = 224;

pub struct GalaxianApp {
    machine: Machine,
    pacer: Pacer,
    last: Instant,
    texture: Option<egui::TextureHandle>,
    frames: u64,
}

impl GalaxianApp {
    pub fn new(machine: Machine) -> GalaxianApp {
        GalaxianApp {
            machine,
            pacer: Pacer::new(),
            last: Instant::now(),
            texture: None,
            frames: 0,
        }
    }

    fn read_keys(ctx: &egui::Context) -> Pressed {
        ctx.input(|i| Pressed {
            coin: i.key_down(egui::Key::Num5),
            start1: i.key_down(egui::Key::Num1),
            start2: i.key_down(egui::Key::Num2),
            left: i.key_down(egui::Key::ArrowLeft),
            right: i.key_down(egui::Key::ArrowRight),
            fire: i.key_down(egui::Key::Space),
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
}

impl eframe::App for GalaxianApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_nanos() as u64;
        self.last = now;

        self.machine.board.inputs = to_inputs(&Self::read_keys(ctx));
        let due = self.pacer.frames_due(dt);
        for _ in 0..due {
            self.machine.run_frame();
            self.machine.board.samples.clear(); // pas de sortie audio dans cette version
            self.frames += 1;
        }

        let image = self.framebuffer_image();
        match &mut self.texture {
            Some(t) => t.set(image, egui::TextureOptions::NEAREST),
            None => {
                self.texture =
                    Some(ctx.load_texture("framebuffer", image, egui::TextureOptions::NEAREST));
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                if let Some(t) = &self.texture {
                    let avail = ui.available_size();
                    // Pixels matériels 4:3 : l'image 768x224 est affichée avec un rapport 768:672 (hauteur x3).
                    let aspect = W as f32 / (H as f32 * 3.0);
                    let mut size = egui::vec2(avail.x, avail.x / aspect);
                    if size.y > avail.y {
                        size = egui::vec2(avail.y * aspect, avail.y);
                    }
                    ui.centered_and_justified(|ui| {
                        ui.add(egui::Image::new((t.id(), size)));
                    });
                }
            });
        ctx.request_repaint();
    }
}
