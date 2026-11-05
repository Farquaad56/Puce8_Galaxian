//! Shells (7, blancs) et missile (1, jaune) : 4 pixels de large, OBJRAM 0x60..0x7F.

use super::{Video, BULLETS_BASE};
use crate::consts::*;
use crate::palette;

impl Video {
    pub(super) fn draw_bullets(&self, y: u32, line: &mut [u32; FB_WIDTH]) {
        let mut shell: Option<usize> = None;
        let mut missile: Option<usize> = None;
        let yi = y as i32;
        // les 3 premières entrées comparent avec y-1
        let effy = if self.flip_y { (yi - 1) ^ 255 } else { yi - 1 };
        for which in 0..3usize {
            if (i32::from(self.objram[BULLETS_BASE + which * 4 + 1]) + effy) as u8 == 0xff {
                shell = Some(which);
            }
        }
        let effy = if self.flip_y { yi ^ 255 } else { yi };
        for which in 3..8usize {
            if (i32::from(self.objram[BULLETS_BASE + which * 4 + 1]) + effy) as u8 == 0xff {
                if which != 7 {
                    shell = Some(which);
                } else {
                    missile = Some(which);
                }
            }
        }
        for which in [shell, missile].into_iter().flatten() {
            let color = palette::bullet_color(which);
            let x = 255 - i32::from(self.objram[BULLETS_BASE + which * 4 + 3]) - 4;
            for i in 0..4 {
                let px = x + i;
                for k in 0..3 {
                    let dx = px * 3 + k;
                    if (0..FB_WIDTH as i32).contains(&dx) {
                        line[dx as usize] = color;
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

    fn set(v: &mut Video, n: usize, y: u8, x: u8) {
        v.objram[BULLETS_BASE + n * 4 + 1] = y;
        v.objram[BULLETS_BASE + n * 4 + 3] = x;
    }

    #[test]
    fn shell_is_4_pixels_wide_and_white() {
        let mut v = Video::new(&RomSet::blank());
        // entrée 3 compare avec y : y + base == 0xFF → base = 0xFF - 100 = 155 pour la ligne 100
        set(&mut v, 3, 155, 100);
        v.start_frame();
        v.finish_frame(1);
        let row = (100 - 16) * FB_WIDTH;
        // x_centre = 255 - 100 = 155 ; pixels 151..=154 → sous-pixels 453..=464
        assert_eq!(v.fb[row + 452], 0);
        assert_eq!(v.fb[row + 453], 0x00FF_FFFF);
        assert_eq!(v.fb[row + 464], 0x00FF_FFFF);
        assert_eq!(v.fb[row + 465], 0);
    }

    #[test]
    fn missile_is_yellow_and_first_three_use_y_minus_1() {
        let mut v = Video::new(&RomSet::blank());
        set(&mut v, 7, 155, 100);
        set(&mut v, 0, 155, 50);
        v.start_frame();
        v.finish_frame(1);
        let row100 = (100 - 16) * FB_WIDTH;
        assert_eq!(v.fb[row100 + 453], 0x00FF_FF00);
        // entrée 0 : match sur y-1 → affichée sur la ligne 101
        let row101 = (101 - 16) * FB_WIDTH;
        assert_eq!(v.fb[row101 + (255 - 50 - 4) * 3], 0x00FF_FFFF);
        assert_eq!(v.fb[row100 + (255 - 50 - 4) * 3], 0);
    }

    #[test]
    fn only_one_shell_per_line_last_wins() {
        let mut v = Video::new(&RomSet::blank());
        set(&mut v, 3, 155, 100);
        set(&mut v, 4, 155, 20);
        v.start_frame();
        v.finish_frame(1);
        let row = (100 - 16) * FB_WIDTH;
        assert_eq!(v.fb[row + 453], 0); // entrée 3 écrasée par la 4
        assert_eq!(v.fb[row + (255 - 20 - 4) * 3], 0x00FF_FFFF);
        let _ = palette::bullet_color(0);
    }
}
