//! Sprites : 8 sprites de 16x16 (octets Y, code/flip, couleur, X dans l'OBJRAM 0x40..0x5F).

use super::{Video, SPRITES_BASE};
use crate::consts::*;

impl Video {
    pub(super) fn draw_sprites(&self, y: u32, line: &mut [u32; FB_WIDTH]) {
        for sprnum in (0..8usize).rev() {
            let base = SPRITES_BASE + sprnum * 4;
            let b0 = self.objram[base];
            let b1 = self.objram[base + 1];
            let b2 = self.objram[base + 2];
            let b3 = self.objram[base + 3];
            // les 3 premiers sprites comparent avec y-1
            let mut sy = (240i32 - (i32::from(b0) - i32::from(sprnum < 3))) as u8;
            let code = (b1 & 0x3f) as usize;
            let mut flipx = b1 & 0x40 != 0;
            let mut flipy = b1 & 0x80 != 0;
            let color = (b2 & 7) as usize;
            let mut sx = b3.wrapping_add(1);
            if self.flip_x {
                sx = 240u8.wrapping_sub(sx);
                flipx = !flipx;
            }
            if self.flip_y {
                sy = 240u8.wrapping_sub(sy);
                flipy = !flipy;
            }
            let r = y as i32 - i32::from(sy);
            if !(0..16).contains(&r) {
                continue;
            }
            let src_y = (if flipy { 15 - r } else { r }) as usize;
            let (clip_l, clip_r) = if self.flip_x {
                (0i32, 716i32)
            } else {
                (51, 767)
            };
            for px in 0..16usize {
                let src_x = if flipx { 15 - px } else { px };
                let pen = self.gfx.sprites[code][src_y * 16 + src_x] as usize;
                if pen == 0 {
                    continue;
                }
                let c = self.palette[color * 4 + pen];
                for k in 0..3usize {
                    let dx = (XSCALE * usize::from(sx) + XSCALE * px + k) as i32;
                    if dx >= clip_l && dx <= clip_r {
                        line[dx as usize] = c;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::RomSet;

    fn video() -> Video {
        let mut rom = RomSet::blank();
        for i in 0..32 {
            rom.gfx[32 + i] = 0xFF; // sprite 1 : plan 0 plein → pen 2
        }
        rom.prom[2] = 0b0000_0111;
        Video::new(&rom)
    }

    fn set_sprite(v: &mut Video, n: usize, y: u8, code: u8, color: u8, x: u8) {
        let b = SPRITES_BASE + n * 4;
        v.objram[b] = y;
        v.objram[b + 1] = code;
        v.objram[b + 2] = color;
        v.objram[b + 3] = x;
    }

    #[test]
    fn sprite_is_drawn_at_expected_position() {
        let mut v = video();
        // sprite 7 : sy = 240 - Y ; Y = 200 → sy = 40 ; sx = X + 1 = 101
        set_sprite(&mut v, 7, 200, 1, 0, 100);
        v.start_frame();
        v.finish_frame(1);
        let c = v.palette[2];
        assert_ne!(c, 0);
        let px = |x: usize, y: usize| v.fb[(y - 16) * FB_WIDTH + x];
        assert_eq!(px(101 * 3, 40), c);
        assert_eq!(px(101 * 3 + 47, 40), c);
        assert_eq!(px(101 * 3 + 48, 40), 0);
        assert_eq!(px(101 * 3, 55), c);
        assert_eq!(px(101 * 3, 56), 0);
        assert_eq!(px(101 * 3, 39), 0);
    }

    #[test]
    fn first_three_sprites_are_one_line_lower() {
        let mut v = video();
        set_sprite(&mut v, 0, 200, 1, 0, 100); // sy = 240 - (200 - 1) = 41
        v.start_frame();
        v.finish_frame(1);
        let px = |x: usize, y: usize| v.fb[(y - 16) * FB_WIDTH + x];
        assert_eq!(px(303, 40), 0);
        assert_ne!(px(303, 41), 0);
    }

    #[test]
    fn left_16_pixels_are_clipped() {
        let mut v = video();
        set_sprite(&mut v, 7, 200, 1, 0, 4); // sx = 5 → x = 15..62 : les pixels < 51 sont coupés
        v.start_frame();
        v.finish_frame(1);
        let px = |x: usize, y: usize| v.fb[(y - 16) * FB_WIDTH + x];
        assert_eq!(px(3, 40), 0);
        assert_eq!(px(50, 40), 0);
        assert_ne!(px(51, 40), 0);
    }

    #[test]
    fn lower_sprite_numbers_have_priority() {
        let mut rom = RomSet::blank();
        for i in 0..32 {
            rom.gfx[32 + i] = 0xFF; // sprite 1 : pen 2
            rom.gfx[0x800 + 32 + i] = 0xFF; // plan 1 aussi → pen 3
        }
        for i in 0..32 {
            rom.gfx[64 + i] = 0xFF; // sprite 2 : pen 2 seulement
        }
        rom.prom[2] = 0b0000_0001;
        rom.prom[3] = 0b0000_0111;
        let mut v = Video::new(&rom);
        set_sprite(&mut v, 3, 200, 1, 0, 100); // pen 3
        set_sprite(&mut v, 4, 200, 2, 0, 100); // pen 2, dessiné avant (numéro plus haut)
        v.start_frame();
        v.finish_frame(1);
        assert_eq!(v.fb[(40 - 16) * FB_WIDTH + 303], v.palette[3]);
    }

    #[test]
    fn y_wraps_in_u8() {
        let mut v = video();
        // Y = 255 → sy = 240 - 255 = -15 → 241 (u8) : sprite hors de l'écran visible (>239)
        set_sprite(&mut v, 7, 255, 1, 0, 100);
        v.start_frame();
        v.finish_frame(1);
        assert!(v.fb.iter().all(|&p| p == 0));
    }
}
