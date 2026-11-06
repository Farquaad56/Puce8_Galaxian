//! Sous-commandes qui exécutent l'émulateur : run, trace-cpu, trace-bus, crc-frames.

use crate::args::Args;
use crate::wav;
use galaxian_core::{trace, InputScript, Machine, RomSet};
use std::io::{BufWriter, Write};
use std::path::Path;

fn load(args: &Args) -> Result<(Machine, InputScript), String> {
    let zip = args
        .pos
        .first()
        .ok_or("usage : <commande> <galaxian.zip> [options]")?;
    let rom = RomSet::load_zip(Path::new(zip)).map_err(|e| e.to_string())?;
    let script = match args.get("inputs") {
        Some(p) => InputScript::parse(&std::fs::read_to_string(p).map_err(|e| e.to_string())?)?,
        None => InputScript::default(),
    };
    let mut m = Machine::new(&rom);
    m.board.inputs = script.at(0);
    Ok((m, script))
}

fn dump_ppm(m: &Machine, path: &str) -> Result<(), String> {
    let mut out = std::fs::File::create(path).map_err(|e| e.to_string())?;
    write!(out, "P6\n768 224\n255\n").map_err(|e| e.to_string())?;
    for &p in &m.board.video.fb {
        out.write_all(&[(p >> 16) as u8, (p >> 8) as u8, p as u8])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// `run <zip> [--frames N] [--inputs f] [--dump-ppm f] [--wav f]` : affiche une ligne `V` par image.
pub fn run(args: &Args) -> Result<(), String> {
    let (mut m, script) = load(args)?;
    let frames: u64 = args.num("frames", 60);
    let mut samples: Vec<f32> = Vec::new();
    for f in 0..frames {
        m.run_frame();
        println!("{}", trace::video_line(f, m.board.video.frame_crc32()));
        samples.append(&mut m.board.samples);
        m.board.inputs = script.at(f + 1);
    }
    println!(
        "watchdog_fired={} pc={:04X}",
        m.board.watchdog_fired, m.cpu.regs.pc
    );
    if let Some(p) = args.get("dump-ppm") {
        dump_ppm(&m, p)?;
    }
    if let Some(p) = args.get("wav") {
        wav::write_wav(p, &samples, 48_000).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// `crc-frames <zip> --frames N [--inputs f] --out fichier` : lignes `V`.
pub fn crc_frames(args: &Args) -> Result<(), String> {
    let (mut m, script) = load(args)?;
    let frames: u64 = args.num("frames", 60);
    let out = args.get("out").ok_or("--out requis")?;
    let mut w = BufWriter::new(std::fs::File::create(out).map_err(|e| e.to_string())?);
    // MAME appelle son rappel « image terminée » une première fois à t = 0 (image noire initiale) :
    // l'index 0 est cette image, l'index n est l'image n-ème fin de VBLANK.
    if frames > 0 {
        writeln!(w, "{}", trace::video_line(0, m.board.video.frame_crc32()))
            .map_err(|e| e.to_string())?;
    }
    for f in 1..frames {
        m.run_frame();
        writeln!(w, "{}", trace::video_line(f, m.board.video.frame_crc32()))
            .map_err(|e| e.to_string())?;
        m.board.inputs = script.at(f);
    }
    Ok(())
}

/// `trace-cpu <zip> --frames N [--inputs f] --out fichier` : une ligne `C` par instruction.
pub fn trace_cpu(args: &Args) -> Result<(), String> {
    let (mut m, script) = load(args)?;
    let frames: u64 = args.num("frames", 60);
    let out = args.get("out").ok_or("--out requis")?;
    let mut w = BufWriter::new(std::fs::File::create(out).map_err(|e| e.to_string())?);
    for f in 0..frames {
        m.run_frame_hook(&mut |mm| {
            let _ = writeln!(w, "{}", trace::cpu_line(mm));
        });
        m.board.inputs = script.at(f + 1);
    }
    Ok(())
}

/// `trace-bus <zip> --frames N [--inputs f] --out fichier` : une ligne `B` par accès 4000..7FFF.
pub fn trace_bus(args: &Args) -> Result<(), String> {
    let (mut m, script) = load(args)?;
    let frames: u64 = args.num("frames", 60);
    let out = args.get("out").ok_or("--out requis")?;
    let mut w = BufWriter::new(std::fs::File::create(out).map_err(|e| e.to_string())?);
    m.board.log = Some(Vec::new());
    for f in 0..frames {
        m.run_frame();
        if let Some(log) = m.board.log.as_mut() {
            for e in log.drain(..) {
                if (0x4000..0x8000).contains(&e.addr) && !(0x4000..0x4800).contains(&e.addr) {
                    writeln!(w, "{}", trace::bus_line(&e)).map_err(|e| e.to_string())?;
                }
            }
        }
        m.board.inputs = script.at(f + 1);
    }
    Ok(())
}
