//! Puce8 Galaxian : frontend egui/eframe (wgpu).
//! Usage : puce8-galaxian-gui <galaxian.zip>
//! Touches par défaut : 5 = pièce, 1 / 2 = start, flèches = déplacement, espace = tir, échap = quitter.
//! F1 : réglages (touches, interrupteurs DIP, rotation de l'écran).

mod app;
mod audio;
mod keys;
mod pacing;
mod settings;

use galaxian_core::{Machine, RomSet};
use std::path::Path;

fn main() -> eframe::Result {
    let zip = match std::env::args().nth(1) {
        Some(z) => z,
        None => {
            eprintln!("usage : puce8-galaxian-gui <galaxian.zip>");
            std::process::exit(2);
        }
    };
    let rom = match RomSet::load_zip(Path::new(&zip)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let machine = Machine::new(&rom);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Puce8 Galaxian")
            .with_inner_size([1024.0, 672.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Puce8 Galaxian",
        options,
        Box::new(|_cc| Ok(Box::new(app::GalaxianApp::new(rom, machine)))),
    )
}
