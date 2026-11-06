//! WAV 16 bits mono (écriture, lecture) et comparaison simple.

use std::io::Write;

pub fn write_wav(path: &str, samples: &[f32], rate: u32) -> std::io::Result<()> {
    let mut out = std::fs::File::create(path)?;
    let data_len = (samples.len() * 2) as u32;
    out.write_all(b"RIFF")?;
    out.write_all(&(36 + data_len).to_le_bytes())?;
    out.write_all(b"WAVEfmt ")?;
    out.write_all(&16u32.to_le_bytes())?;
    out.write_all(&1u16.to_le_bytes())?; // PCM
    out.write_all(&1u16.to_le_bytes())?; // mono
    out.write_all(&rate.to_le_bytes())?;
    out.write_all(&(rate * 2).to_le_bytes())?;
    out.write_all(&2u16.to_le_bytes())?;
    out.write_all(&16u16.to_le_bytes())?;
    out.write_all(b"data")?;
    out.write_all(&data_len.to_le_bytes())?;
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.write_all(&v.to_le_bytes())?;
    }
    Ok(())
}

/// Lit un WAV PCM 16 bits ; retourne (échantillons du canal gauche normalisés, fréquence).
pub fn read_wav(path: &str) -> Result<(Vec<f32>, u32), String> {
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    if b.len() < 44 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("pas un WAV".into());
    }
    let mut pos = 12;
    let (mut channels, mut rate, mut bits) = (1u16, 0u32, 16u16);
    while pos + 8 <= b.len() {
        let id = &b[pos..pos + 4];
        let len = u32::from_le_bytes([b[pos + 4], b[pos + 5], b[pos + 6], b[pos + 7]]) as usize;
        let body = pos + 8;
        if id == b"fmt " {
            channels = u16::from_le_bytes([b[body + 2], b[body + 3]]);
            rate = u32::from_le_bytes([b[body + 4], b[body + 5], b[body + 6], b[body + 7]]);
            bits = u16::from_le_bytes([b[body + 14], b[body + 15]]);
        } else if id == b"data" {
            if bits != 16 {
                return Err("seul le PCM 16 bits est géré".into());
            }
            let end = (body + len).min(b.len());
            let step = 2 * usize::from(channels);
            let mut out = Vec::new();
            let mut i = body;
            while i + 1 < end {
                out.push(f32::from(i16::from_le_bytes([b[i], b[i + 1]])) / 32768.0);
                i += step;
            }
            return Ok((out, rate));
        }
        pos = body + len + (len & 1);
    }
    Err("chunk data introuvable".into())
}

/// (RMS de a, RMS de b, RMS de la différence, corrélation) sur la longueur commune.
pub fn compare(a: &[f32], b: &[f32]) -> (f64, f64, f64, f64) {
    let n = a.len().min(b.len());
    if n == 0 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let (mut sa, mut sb, mut sd, mut sab) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for i in 0..n {
        let (x, y) = (f64::from(a[i]), f64::from(b[i]));
        sa += x * x;
        sb += y * y;
        sd += (x - y) * (x - y);
        sab += x * y;
    }
    let nn = n as f64;
    let corr = if sa > 0.0 && sb > 0.0 {
        sab / (sa.sqrt() * sb.sqrt())
    } else {
        0.0
    };
    ((sa / nn).sqrt(), (sb / nn).sqrt(), (sd / nn).sqrt(), corr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_round_trip() {
        let dir = std::env::temp_dir().join("galaxian_wav_test.wav");
        let p = dir.to_str().unwrap();
        let s: Vec<f32> = (0..100).map(|i| (i as f32 / 100.0) - 0.5).collect();
        write_wav(p, &s, 48_000).unwrap();
        let (r, rate) = read_wav(p).unwrap();
        assert_eq!(rate, 48_000);
        assert_eq!(r.len(), 100);
        assert!((r[10] - s[10]).abs() < 0.001);
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn compare_identical_signals() {
        let a = vec![0.1f32, -0.2, 0.3, -0.4];
        let (_, _, d, c) = compare(&a, &a);
        assert_eq!(d, 0.0);
        assert!((c - 1.0).abs() < 1e-9);
    }
}
