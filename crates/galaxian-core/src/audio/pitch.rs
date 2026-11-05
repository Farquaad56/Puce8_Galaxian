//! Compteur de pitch : port exact de `dss_note` (MAME disc_wav.hxx).

use super::SAMPLE_TIME;

/// Horloge du compteur de pitch : 18.432 MHz / 6 / 2.
const PITCH_CLOCK_HZ: f64 = 1_536_000.0;

/// Compteur de pitch : port exact de `dss_note` (max1 = 255, max2 = 15).
pub struct Pitch {
    count1: i32,
    count2: i32,
    t_left: f64,
}

impl Pitch {
    pub fn new(initial_pitch: u8) -> Pitch {
        Pitch {
            count1: i32::from(initial_pitch),
            count2: 0,
            t_left: 0.0,
        }
    }

    /// Avance d'un échantillon ; `pitch` = valeur du latch 7800. Retourne `count2` (0..=15).
    pub fn step_sample(&mut self, pitch: u8) -> u8 {
        let t_clock = 1.0 / PITCH_CLOCK_HZ;
        let cycles = (self.t_left + SAMPLE_TIME) / t_clock;
        let inc = cycles as i32;
        self.t_left = (cycles - f64::from(inc)) * t_clock;
        if i32::from(pitch) != 255 {
            for _ in 0..inc {
                self.count1 += 1;
                if self.count1 > 255 {
                    self.count1 = i32::from(pitch);
                    self.count2 += 1;
                    if self.count2 > 15 {
                        self.count2 = 0;
                    }
                }
            }
        }
        self.count2 as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitch_counter_divides_by_256_minus_pitch() {
        // pitch = 240 → count1 recharge à 240 : 16 horloges (1.536 MHz) par incrément de count2
        let mut p = Pitch::new(240);
        let mut transitions = 0;
        let mut last = 0u8;
        for _ in 0..48_000 {
            let c = p.step_sample(240);
            if c != last {
                transitions += 1;
                last = c;
            }
        }
        // 1.536e6 / 16 = 96000 incréments/s  → déborde le modulo 16, mais > 48000 changements/s
        assert!(transitions > 40_000, "transitions = {transitions}");
    }

    #[test]
    fn pitch_255_freezes_counter() {
        let mut p = Pitch::new(255);
        for _ in 0..1000 {
            assert_eq!(p.step_sample(255), 0);
        }
    }
}
