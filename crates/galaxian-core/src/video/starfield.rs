//! Fond d'étoiles : une ligne, comme `stars_draw_row` de MAME.

use super::Video;
use crate::consts::*;

impl Video {
    pub(super) fn draw_stars(&self, y: u32, line: &mut [u32; FB_WIDTH]) {
        let period = STAR_RNG_PERIOD;
        let mut offs = self.star_origin.wrapping_add(y.wrapping_mul(512)) % period;
        for x in 0..256usize {
            let enable = ((y as usize) ^ (x >> 3)) & 1 == 1;
            // premier tick RNG : 1 pixel
            let s = self.star_table[offs as usize];
            offs += 1;
            if offs >= period {
                offs = 0;
            }
            if enable && s & 0x80 != 0 {
                line[XSCALE * x] = self.star_colors[(s & 0x3f) as usize];
            }
            // second tick RNG : 2 pixels
            let s = self.star_table[offs as usize];
            offs += 1;
            if offs >= period {
                offs = 0;
            }
            if enable && s & 0x80 != 0 {
                let c = self.star_colors[(s & 0x3f) as usize];
                line[XSCALE * x + 1] = c;
                line[XSCALE * x + 2] = c;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::consts::*;
    use crate::rom::RomSet;
    use crate::video::Video;

    #[test]
    fn stars_appear_only_when_enabled() {
        let mut v = Video::new(&RomSet::blank());
        v.start_frame();
        v.finish_frame(1);
        assert!(v.fb.iter().all(|&p| p == 0));
        let mut v = Video::new(&RomSet::blank());
        v.stars_enabled = true;
        v.start_frame();
        v.finish_frame(0); // frame == star_origin_frame : pas de recalage
        let n = v.fb.iter().filter(|&&p| p != 0).count();
        assert!(n > 100, "il doit y avoir des étoiles, trouvé {n}");
        assert_eq!(v.fb.len(), FB_WIDTH * FB_HEIGHT);
    }
}
