//! Test d'intégration avec la vraie ROM. Ignoré silencieusement si galaxian.zip est absent.
//! Chemin : variable d'environnement GALAXIAN_ZIP, sinon X:\Mame_0287\roms\galaxian.zip.

use galaxian_core::{Machine, RomSet};
use std::path::PathBuf;

fn zip_path() -> Option<PathBuf> {
    let p = std::env::var("GALAXIAN_ZIP")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"X:\Mame_0287\roms\galaxian.zip"));
    p.exists().then_some(p)
}

#[test]
fn real_rom_loads_and_boots_without_watchdog_reset() {
    let Some(p) = zip_path() else {
        eprintln!("galaxian.zip absent : test ignoré");
        return;
    };
    let rom = RomSet::load_zip(&p).expect("ROM valides");
    assert_eq!(rom.program.len(), 0x4000);
    let mut m = Machine::new(&rom);
    for _ in 0..120 {
        m.run_frame();
    }
    assert!(
        !m.board.watchdog_fired,
        "le jeu doit rafraîchir le watchdog"
    );
    // l'écran d'attract affiche du texte : le framebuffer n'est pas vide
    assert!(m.board.video.fb.iter().any(|&p| p != 0));
}

#[test]
fn real_rom_run_is_deterministic() {
    let Some(p) = zip_path() else {
        return;
    };
    let rom = RomSet::load_zip(&p).expect("ROM valides");
    let crcs = |_: ()| {
        let mut m = Machine::new(&rom);
        (0..30)
            .map(|_| {
                m.run_frame();
                m.board.video.frame_crc32()
            })
            .collect::<Vec<u32>>()
    };
    assert_eq!(crcs(()), crcs(()));
}
