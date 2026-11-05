//! Palette : port fidèle de `compute_resistor_weights` (MAME resnet.cpp) et de
//! `galaxian_state::galaxian_palette` (galaxian_v.cpp).

/// Poids d'un réseau de résistances (resnet.cpp, un réseau à la fois).
/// Retourne les tensions non mises à l'échelle `w[n]` (clamp minval..maxval).
fn raw_weights(res: &[i32], pulldown: i32, pullup: i32, minval: i32, maxval: i32) -> Vec<f64> {
    let mut w = Vec::with_capacity(res.len());
    for n in 0..res.len() {
        let mut r0 = if pulldown == 0 {
            1.0 / 1e12
        } else {
            1.0 / f64::from(pulldown)
        };
        let mut r1 = if pullup == 0 {
            1.0 / 1e12
        } else {
            1.0 / f64::from(pullup)
        };
        for (j, &r) in res.iter().enumerate() {
            let r = f64::from(r);
            if j == n {
                if r != 0.0 {
                    r1 += 1.0 / r;
                }
            } else if r != 0.0 {
                r0 += 1.0 / r;
            }
        }
        r0 = 1.0 / r0;
        r1 = 1.0 / r1;
        let vout = f64::from(maxval - minval) * r0 / (r1 + r0) + f64::from(minval);
        w.push(vout.clamp(f64::from(minval), f64::from(maxval)));
    }
    w
}

/// `compute_resistor_weights(0, maxval, -1.0, ...)` avec mise à l'échelle automatique
/// sur le réseau qui a la plus grande somme. Retourne un `Vec` de poids par réseau.
pub fn compute_resistor_weights(maxval: i32, nets: &[(&[i32], i32, i32)]) -> Vec<Vec<f64>> {
    let raw: Vec<Vec<f64>> = nets
        .iter()
        .map(|(res, pd, pu)| raw_weights(res, *pd, *pu, 0, maxval))
        .collect();
    let mut max = 0.0f64;
    let mut jmax = 0usize;
    for (i, w) in raw.iter().enumerate() {
        let sum: f64 = w.iter().sum();
        if max < sum {
            max = sum;
            jmax = i;
        }
    }
    let max_out: f64 = raw[jmax].iter().sum();
    let scale = f64::from(maxval) / max_out;
    raw.iter()
        .map(|w| w.iter().map(|x| x * scale).collect())
        .collect()
}

/// `combine_weights` de MAME : somme pondérée + 0.5, tronquée.
fn combine(weights: &[f64], bits: &[u8]) -> u32 {
    let mut s = 0.0f64;
    for (w, &b) in weights.iter().zip(bits) {
        s += w * f64::from(b);
    }
    (s + 0.5) as u32
}

const RGB_MAXIMUM: i32 = 224;
const RGB_RESISTANCES: [i32; 3] = [1000, 470, 220];

/// Palette de 32 couleurs `0x00RRGGBB` depuis la PROM 6l.bpr.
pub fn build_palette(prom: &[u8; 32]) -> [u32; 32] {
    let nets = compute_resistor_weights(
        RGB_MAXIMUM,
        &[
            (&RGB_RESISTANCES[0..3], 470, 0), // rouge
            (&RGB_RESISTANCES[0..3], 470, 0), // vert
            (&RGB_RESISTANCES[1..3], 470, 0), // bleu (2 résistances)
        ],
    );
    let (rw, gw, bw) = (&nets[0], &nets[1], &nets[2]);
    let mut pal = [0u32; 32];
    for (i, &p) in prom.iter().enumerate() {
        let bit = |n: u8| (p >> n) & 1;
        let r = combine(rw, &[bit(0), bit(1), bit(2)]);
        let g = combine(gw, &[bit(3), bit(4), bit(5)]);
        let b = combine(bw, &[bit(6), bit(7)]);
        pal[i] = (r << 16) | (g << 8) | b;
    }
    pal
}

/// Les 64 couleurs d'étoiles (index de 6 bits : RR GG BB, chaque paire = résistances 150/100 ohms).
pub fn build_star_colors() -> [u32; 64] {
    let minval = RGB_MAXIMUM * 130 / 150;
    let midval = RGB_MAXIMUM * 130 / 100;
    let maxval = RGB_MAXIMUM * 130 / 60;
    let starmap: [u32; 4] = [
        0,
        minval as u32,
        (minval + (255 - minval) * (midval - minval) / (maxval - minval)) as u32,
        255,
    ];
    let mut out = [0u32; 64];
    for (i, o) in out.iter_mut().enumerate() {
        let bit = |n: usize| (i >> n) & 1;
        let r = starmap[(bit(4) << 1) | bit(5)];
        let g = starmap[(bit(2) << 1) | bit(3)];
        let b = starmap[(bit(0) << 1) | bit(1)];
        *o = (r << 16) | (g << 8) | b;
    }
    out
}

/// Couleur des shells (blanc) et du missile (jaune) : index 0..=6 blanc, 7 jaune.
pub const fn bullet_color(index: usize) -> u32 {
    if index == 7 {
        0x00FF_FF00
    } else {
        0x00FF_FFFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_colors_endpoints() {
        let s = build_star_colors();
        assert_eq!(s[0], 0);
        assert_eq!(s[63], 0x00FF_FFFF);
        // 194 pour une seule résistance 150 ohms
        assert_eq!(s[0b100000], 194 << 16);
        assert_eq!(s[0b010000], 214 << 16);
    }

    #[test]
    fn palette_black_and_max_are_monotonic() {
        let mut prom = [0u8; 32];
        assert_eq!(build_palette(&prom)[0], 0);
        prom[1] = 0xFF;
        let c = build_palette(&prom)[1];
        // tous les canaux à fond : au plus RGB_MAXIMUM (224) après mise à l'échelle
        assert_eq!((c >> 16) & 0xFF, 224);
        assert_eq!((c >> 8) & 0xFF, 224);
        assert!(((c) & 0xFF) <= 224 && (c & 0xFF) > 150);
    }

    #[test]
    fn weights_are_ordered_by_resistance() {
        let nets = compute_resistor_weights(224, &[(&[1000, 470, 220], 470, 0)]);
        let w = &nets[0];
        assert!(w[0] < w[1] && w[1] < w[2]);
    }
}
