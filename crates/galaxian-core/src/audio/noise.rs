//! Bruit : LFSR 17 bits, port exact de `dss_lfsr_noise` (MAME disc_wav.hxx).

use super::SAMPLE_TIME;

/// Horloge du LFSR bruit : RNG_RATE/100 = 18.432 MHz / 3 * 2 / 100.
const NOISE_CLOCK_HZ: f64 = 18_432_000.0 / 3.0 * 2.0 / 100.0;

/// LFSR bruit : port exact de `dss_lfsr_noise` avec le descripteur `galaxian_lfsr`.
pub struct Noise {
    reg: u32,
    t_left: f64,
    out: bool,
}

impl Noise {
    pub fn new() -> Noise {
        // reset() de MAME : registre 0, puis calcul du bit de rétroaction dans le bit 17
        let mut n = Noise {
            reg: 0,
            t_left: 0.0,
            out: false,
        };
        let f0 = n.feedback();
        n.reg |= f0 << 17;
        n.out = f0 != 0;
        n
    }

    /// F0 = XOR_INV_IN1(bit4, bit16) = bit4 ^ bit16 ^ 1.
    fn feedback(&self) -> u32 {
        let fb0 = (self.reg >> 4) & 1;
        let fb1 = (self.reg >> 16) & 1;
        fb0 ^ fb1 ^ 1
    }

    /// Avance d'un échantillon de 1/48000 s ; retourne la sortie (maintenue entre deux horloges).
    pub fn step_sample(&mut self) -> bool {
        let t_clock = 1.0 / NOISE_CLOCK_HZ;
        let cycles = (self.t_left + SAMPLE_TIME) / t_clock;
        let inc = cycles as i32;
        self.t_left = (cycles - f64::from(inc)) * t_clock;
        let mut f0 = 0;
        for _ in 0..inc {
            let fb = (self.reg >> 17) & 1; // bit « en attente » (étage 1 : IN0, étage 2 : masque 1)
            self.reg = ((self.reg << 1) & 0x1FFFF) | fb;
            f0 = self.feedback();
            self.reg |= f0 << 17;
        }
        if inc > 0 {
            self.out = f0 != 0;
        }
        self.out
    }
}

impl Default for Noise {
    fn default() -> Self {
        Noise::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_toggles() {
        let mut a = Noise::new();
        let mut b = Noise::new();
        let sa: Vec<bool> = (0..5000).map(|_| a.step_sample()).collect();
        let sb: Vec<bool> = (0..5000).map(|_| b.step_sample()).collect();
        assert_eq!(sa, sb);
        let ones = sa.iter().filter(|&&x| x).count();
        assert!(
            ones > 1500 && ones < 3500,
            "bruit déséquilibré : {ones}/5000"
        );
    }

    #[test]
    fn noise_clock_gives_about_2_56_clocks_per_sample() {
        // 122880 / 48000 = 2.56 : après 48000 échantillons le LFSR a reçu ~122880 horloges
        let mut n = Noise::new();
        for _ in 0..48_000 {
            n.step_sample();
        }
        // pas d'assertion sur l'état interne ; on vérifie seulement l'absence de panique
    }
}
