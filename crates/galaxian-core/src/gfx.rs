//! Décodage des ROM graphiques (galaxian_charlayout / galaxian_spritelayout, MAME).
//!
//! `gfx` = 0x1000 octets : plan 0 aux octets 0x000..0x7FF, plan 1 aux octets 0x800..0xFFF.
//! Dans une ligne, le pixel x=0 est le bit de poids fort (0x80). pen = (bit_plan0 << 1) | bit_plan1.

/// Tuiles 8x8 (256) et sprites 16x16 (64) décodés en pens 0..=3.
pub struct Gfx {
    /// `tiles[code][y * 8 + x]`
    pub tiles: Vec<[u8; 64]>,
    /// `sprites[code][y * 16 + x]`
    pub sprites: Vec<[u8; 256]>,
}

fn bit(gfx: &[u8], plane: usize, byte: usize, x_in_byte: usize) -> u8 {
    (gfx[plane * 0x800 + byte] >> (7 - x_in_byte)) & 1
}

pub fn decode(gfx: &[u8]) -> Gfx {
    assert!(gfx.len() >= 0x1000, "gfx doit faire 0x1000 octets");
    let mut tiles = vec![[0u8; 64]; 256];
    for (code, t) in tiles.iter_mut().enumerate() {
        for y in 0..8 {
            for x in 0..8 {
                let b0 = bit(gfx, 0, code * 8 + y, x);
                let b1 = bit(gfx, 1, code * 8 + y, x);
                t[y * 8 + x] = (b0 << 1) | b1;
            }
        }
    }
    let mut sprites = vec![[0u8; 256]; 64];
    for (code, s) in sprites.iter_mut().enumerate() {
        for y in 0..16 {
            for x in 0..16 {
                // quadrants : haut-gauche +0, haut-droit +8, bas-gauche +16, bas-droit +24 octets
                let byte = code * 32 + (y / 8) * 16 + (x / 8) * 8 + (y % 8);
                let b0 = bit(gfx, 0, byte, x % 8);
                let b1 = bit(gfx, 1, byte, x % 8);
                s[y * 16 + x] = (b0 << 1) | b1;
            }
        }
    }
    Gfx { tiles, sprites }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_bit_order_and_planes() {
        let mut g = vec![0u8; 0x1000];
        g[3] = 0b1000_0001; // tuile 0, ligne 3, plan 0 : pixels x=0 et x=7
        g[0x800 + 3] = 0b1000_0000; // plan 1 : pixel x=0
        let d = decode(&g);
        assert_eq!(d.tiles[0][3 * 8], 0b11);
        assert_eq!(d.tiles[0][3 * 8 + 7], 0b10);
        assert_eq!(d.tiles[0][3 * 8 + 1], 0);
    }

    #[test]
    fn sprite_quadrants() {
        let mut g = vec![0u8; 0x1000];
        // sprite 1 : base octet 32. Quadrant bas-droit (x>=8, y>=8) : octet 32+16+8+row
        g[32 + 16 + 8] = 0b1000_0000; // y=8, x=8, plan 0
        let d = decode(&g);
        assert_eq!(d.sprites[1][8 * 16 + 8], 0b10);
        // quadrant haut-droit : octet 32+8+row
        g[32 + 8 + 2] = 0b0000_0001; // y=2, x=15
        let d = decode(&g);
        assert_eq!(d.sprites[1][2 * 16 + 15], 0b10);
    }
}
