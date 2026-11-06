use galaxian_cli::args::Args;
use galaxian_cli::{diff, rawcrc, run, wav};
use std::process::exit;

const USAGE: &str = "puce8-galaxian <commande> ...
  run <zip> [--frames N] [--inputs f] [--dump-ppm f] [--wav f]
  crc-frames <zip> --frames N [--inputs f] --out f
  trace-cpu <zip> --frames N [--inputs f] --out f
  trace-bus <zip> --frames N [--inputs f] --out f
  raw-crc <fichier.raw> [--width 768] [--height 224] [--format bgra|rgba] --out f
  diff <a> <b> [--tol-tstate N] [--tol-hpos N] [--ignore-vpos] [--ignore-pc]
  wav-compare <a.wav> <b.wav>";

fn diff_cmd(a: &Args) -> Result<(), String> {
    if a.pos.len() < 2 {
        return Err("usage : diff <a> <b>".into());
    }
    let ta = std::fs::read_to_string(&a.pos[0]).map_err(|e| e.to_string())?;
    let tb = std::fs::read_to_string(&a.pos[1]).map_err(|e| e.to_string())?;
    let o = diff::DiffOptions {
        tol_tstate: a.get("tol-tstate").and_then(|v| v.parse().ok()),
        tol_hpos: a.get("tol-hpos").and_then(|v| v.parse().ok()),
        check_vpos: !a.has("ignore-vpos"),
        check_pc: !a.has("ignore-pc"),
    };
    let n = diff::compare_text(&ta, &tb, &o)?;
    println!("IDENTIQUE : {n} lignes comparées");
    Ok(())
}

fn wav_cmd(a: &Args) -> Result<(), String> {
    if a.pos.len() < 2 {
        return Err("usage : wav-compare <a.wav> <b.wav>".into());
    }
    let (x, rx) = wav::read_wav(&a.pos[0])?;
    let (y, ry) = wav::read_wav(&a.pos[1])?;
    let (ra, rb, rd, c) = wav::compare(&x, &y);
    println!(
        "rate {rx}/{ry} n {}/{} rms_a {ra:.5} rms_b {rb:.5} rms_diff {rd:.5} corr {c:.4}",
        x.len(),
        y.len()
    );
    Ok(())
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = raw.first().cloned() else {
        eprintln!("{USAGE}");
        exit(2);
    };
    let args = Args::parse(&raw[1..], &["ignore-vpos", "ignore-pc"]);
    let result = match cmd.as_str() {
        "run" => run::run(&args),
        "crc-frames" => run::crc_frames(&args),
        "trace-cpu" => run::trace_cpu(&args),
        "trace-bus" => run::trace_bus(&args),
        "raw-crc" => rawcrc::raw_crc(&args),
        "diff" => diff_cmd(&args),
        "wav-compare" => wav_cmd(&args),
        _ => Err(USAGE.to_string()),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        exit(1);
    }
}
