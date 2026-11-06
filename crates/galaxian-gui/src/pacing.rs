//! Cadencement : une image Galaxian = 50 688 T-states à 3 072 000 Hz = exactement 16,5 ms.

/// Durée exacte d'une image matérielle, en nanosecondes.
pub const FRAME_NS: u64 = 16_500_000;
/// Nombre maximal d'images émulées par rafraîchissement d'écran (évite la spirale de rattrapage).
pub const MAX_CATCHUP: u32 = 4;

/// Accumulateur de temps réel.
#[derive(Default)]
pub struct Pacer {
    acc_ns: u64,
}

impl Pacer {
    pub fn new() -> Pacer {
        Pacer { acc_ns: 0 }
    }

    /// Ajoute `dt_ns` de temps réel écoulé ; retourne le nombre d'images à émuler maintenant.
    pub fn frames_due(&mut self, dt_ns: u64) -> u32 {
        self.acc_ns += dt_ns;
        let mut n = 0;
        while self.acc_ns >= FRAME_NS && n < MAX_CATCHUP {
            self.acc_ns -= FRAME_NS;
            n += 1;
        }
        if n == MAX_CATCHUP {
            self.acc_ns = 0;
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_frame_after_16_5_ms() {
        let mut p = Pacer::new();
        assert_eq!(p.frames_due(16_499_999), 0);
        assert_eq!(p.frames_due(1), 1);
        assert_eq!(p.frames_due(0), 0);
    }

    #[test]
    fn catch_up_is_capped_and_resets() {
        let mut p = Pacer::new();
        assert_eq!(p.frames_due(1_000_000_000), MAX_CATCHUP);
        assert_eq!(p.frames_due(0), 0);
    }

    #[test]
    fn frame_duration_matches_clock() {
        // 50688 T-states / 3 072 000 Hz
        assert_eq!(50_688u64 * 1_000_000_000 / 3_072_000, FRAME_NS);
    }
}
