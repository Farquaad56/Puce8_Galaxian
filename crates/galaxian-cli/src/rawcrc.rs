//! `raw-crc <fichier.raw> [--width 768] [--height 224] [--format bgra|rgba] --out fichier`
//! Convertit un fichier de dumps bruts de MAME (images consécutives) en lignes `V`.

use crate::args::Args;
use galaxian_core::crc::crc32;
use galaxian_core::trace;
use std::io::Write;

/// CRC32 des octets R,G,B d'une image `pixels` (4 octets par pixel).
pub fn frame_crc(pixels: &[u8], bgra: bool) -> u32 {
    let mut rgb = Vec::with_capacity(pixels.len() / 4 * 3);
    let (chunks, _) = pixels.as_chunks::<4>();
    for px in chunks {
        if bgra {
            rgb.extend_from_slice(&[px[2], px[1], px[0]]);
        } else {
            rgb.extend_from_slice(&[px[0], px[1], px[2]]);
        }
    }
    crc32(&rgb)
}

pub fn raw_crc(args: &Args) -> Result<(), String> {
    let path = args
        .pos
        .first()
        .ok_or("usage : raw-crc <fichier.raw> --out f")?;
    let w: usize = args.num("width", 768);
    let h: usize = args.num("height", 224);
    let bgra = args.get("format").unwrap_or("bgra") == "bgra";
    let out = args.get("out").ok_or("--out requis")?;
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let size = w * h * 4;
    if size == 0 || data.len() % size != 0 {
        return Err(format!("taille {} non multiple de {size}", data.len()));
    }
    let mut f = std::fs::File::create(out).map_err(|e| e.to_string())?;
    for (i, frame) in data.chunks_exact(size).enumerate() {
        writeln!(f, "{}", trace::video_line(i as u64, frame_crc(frame, bgra)))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgra_and_rgba_give_same_crc_for_swapped_channels() {
        let bgra = [3u8, 2, 1, 255, 6, 5, 4, 255];
        let rgba = [1u8, 2, 3, 255, 4, 5, 6, 255];
        assert_eq!(frame_crc(&bgra, true), frame_crc(&rgba, false));
    }
}
