//! Table du LFSR 17 bits des étoiles (MAME `galaxian_state::stars_init`).

use crate::consts::STAR_RNG_PERIOD;

/// `table[i]` : bits 0..=5 = couleur (index dans la palette d'étoiles), bit 7 = étoile active.
pub fn build_star_table() -> Vec<u8> {
    let mut table = vec![0u8; STAR_RNG_PERIOD as usize];
    let mut shiftreg: u32 = 0;
    for entry in table.iter_mut() {
        // étoile active si les 8 bits hauts valent 1 et le bit 0 vaut 0
        let enabled = (shiftreg & 0x1fe01) == 0x1fe00;
        // la couleur vient des 6 bits sous les 8 bits hauts
        let color = ((!shiftreg) & 0x1f8) >> 3;
        *entry = (color as u8) | (u8::from(enabled) << 7);
        // rétroaction : XOR du bit 12 et de l'inverse du bit 0
        shiftreg = (shiftreg >> 1) | ((((shiftreg >> 12) ^ !shiftreg) & 1) << 16);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_period_entries_and_some_stars() {
        let t = build_star_table();
        assert_eq!(t.len(), 131_071);
        let n = t.iter().filter(|&&e| e & 0x80 != 0).count();
        assert!(n > 100 && n < 2000, "nombre d'étoiles inattendu : {n}");
        assert!(t.iter().all(|&e| e & 0x40 == 0));
    }

    #[test]
    fn table_is_deterministic() {
        assert_eq!(build_star_table(), build_star_table());
    }
}
