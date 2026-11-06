//! Constantes de timing et de géométrie (source : MAME galaxian.h / galaxian_v.cpp).
//!
//! Unités : `t` = T-state du Z80 (3,072 MHz). 1 T-state = 6 cycles d'horloge maître (18,432 MHz).

/// Quartz principal (Hz).
pub const MASTER_CLOCK_HZ: u32 = 18_432_000;
/// Horloge du Z80 = quartz / 6 (Hz).
pub const CPU_CLOCK_HZ: u32 = 3_072_000;
/// Cycles d'horloge maître par T-state.
pub const MASTER_PER_T: u32 = 6;
/// T-states par ligne vidéo (384 pixels à 6,144 MHz = 192 T-states).
pub const T_PER_LINE: u32 = 192;
/// Lignes par image (V total).
pub const LINES_PER_FRAME: u32 = 264;
/// T-states par image : 192 * 264 = 50688.
pub const T_PER_FRAME: u32 = T_PER_LINE * LINES_PER_FRAME;
/// Cycles d'horloge maître par ligne (échelle `hpos` de MAME, GALAXIAN_HTOTAL = 384*3).
pub const MASTER_PER_LINE: u32 = 1152;
/// Première ligne visible (GALAXIAN_VBEND).
pub const FIRST_VISIBLE_LINE: u32 = 16;
/// Dernière ligne visible (GALAXIAN_VBSTART - 1).
pub const LAST_VISIBLE_LINE: u32 = 239;
/// Ligne où commence le VBLANK (GALAXIAN_VBSTART) : l'interruption est levée ici.
pub const VBLANK_START_LINE: u32 = 240;
/// Facteur d'échelle horizontal du framebuffer (GALAXIAN_XSCALE).
pub const XSCALE: usize = 3;
/// Largeur du framebuffer : 256 * 3.
pub const FB_WIDTH: usize = 768;
/// Hauteur du framebuffer : lignes 16..=239.
pub const FB_HEIGHT: usize = 224;
/// Période du LFSR des étoiles : 2^17 - 1.
pub const STAR_RNG_PERIOD: u32 = (1 << 17) - 1;

/// Phase de l'écran à t = 0 : MAME démarre avec le faisceau au début du VBLANK (ligne 240),
/// soit 240 lignes de 192 T-states. Vérifié contre MAME 0.287 (vpos/hpos du débogueur et CRC vidéo).
pub const SCREEN_PHASE_T: u32 = 240 * T_PER_LINE;

/// Avance (en T-states) de la ligne NMI sur le début du VBLANK, calibrée sur la trace MAME 0.287.
pub const NMI_LEAD_T: u32 = 1;

/// Décalage (T-states) entre la date de l'accès bus dans la trace MAME et la fin du cycle machine.
pub const BUS_STAMP_LAG_T: u32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_is_50688_tstates() {
        assert_eq!(T_PER_FRAME, 50_688);
        assert_eq!(MASTER_PER_LINE, T_PER_LINE * MASTER_PER_T);
        assert_eq!(CPU_CLOCK_HZ, MASTER_CLOCK_HZ / 6);
        assert_eq!(FB_HEIGHT as u32, LAST_VISIBLE_LINE - FIRST_VISIBLE_LINE + 1);
    }
}
