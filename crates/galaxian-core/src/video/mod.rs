//! Vidéo Galaxian : rendu ligne par ligne qui reproduit `update_partial` de MAME.
//!
//! Règle d'or : avant toute écriture qui change l'image (VRAM, OBJRAM, flip, étoiles), on
//! rend les lignes jusqu'à la ligne courante INCLUSE avec l'ANCIEN état. Le nouvel état ne
//! s'applique donc qu'aux lignes strictement après la ligne de l'écriture.
//! Le framebuffer est en résolution 768x224 (x3 horizontal, comme MAME avec GALAXIAN_XSCALE).

mod bullets;
mod sprites;
mod starfield;
mod tiles;

use crate::consts::*;
use crate::crc::crc32;
use crate::gfx::{self, Gfx};
use crate::palette;
use crate::rom::RomSet;
use crate::stars;

pub(crate) const SPRITES_BASE: usize = 0x40;
pub(crate) const BULLETS_BASE: usize = 0x60;

pub struct Video {
    pub vram: [u8; 0x400],
    pub objram: [u8; 0x100],
    pub flip_x: bool,
    pub flip_y: bool,
    pub stars_enabled: bool,
    pub star_origin: u32,
    pub star_origin_frame: u32,
    /// 768 x 224 pixels `0x00RRGGBB`. Ligne 0 = ligne écran 16.
    pub fb: Vec<u32>,
    next_row: u32,
    frame_finished: bool,
    pub(crate) gfx: Gfx,
    pub(crate) palette: [u32; 32],
    pub(crate) star_colors: [u32; 64],
    pub(crate) star_table: Vec<u8>,
}

impl Video {
    pub fn new(rom: &RomSet) -> Video {
        let mut prom = [0u8; 32];
        prom.copy_from_slice(&rom.prom);
        Video {
            vram: [0; 0x400],
            objram: [0; 0x100],
            flip_x: false,
            flip_y: false,
            stars_enabled: false,
            star_origin: 0,
            star_origin_frame: 0,
            fb: vec![0; FB_WIDTH * FB_HEIGHT],
            next_row: 0,
            frame_finished: true,
            gfx: gfx::decode(&rom.gfx),
            palette: palette::build_palette(&prom),
            star_colors: palette::build_star_colors(),
            star_table: stars::build_star_table(),
        }
    }

    /// Début d'une nouvelle image (ligne 0).
    pub fn start_frame(&mut self) {
        self.next_row = 0;
        self.frame_finished = false;
    }

    /// Début du VBLANK : termine le rendu de toutes les lignes visibles.
    pub fn finish_frame(&mut self, frame: u32) {
        self.render_until(LAST_VISIBLE_LINE, frame);
        self.frame_finished = true;
    }

    /// Rend les lignes `next_row..=vpos` (bornées à 16..=239) avec l'état courant.
    pub fn render_until(&mut self, vpos: u32, frame: u32) {
        if self.frame_finished {
            return;
        }
        let last = vpos.min(LAST_VISIBLE_LINE);
        let first = self.next_row.max(FIRST_VISIBLE_LINE);
        if last < first {
            return;
        }
        self.stars_update_origin(frame);
        for y in first..=last {
            self.render_row(y);
        }
        self.next_row = last + 1;
    }

    fn stars_update_origin(&mut self, frame: u32) {
        if frame != self.star_origin_frame {
            let per_frame_delta: i32 = if self.flip_x { 1 } else { -1 };
            let mut total = per_frame_delta * (frame as i32 - self.star_origin_frame as i32);
            while total < 0 {
                total += STAR_RNG_PERIOD as i32;
            }
            self.star_origin = self.star_origin.wrapping_add(total as u32) % STAR_RNG_PERIOD;
            self.star_origin_frame = frame;
        }
    }

    // ---------------------------------------------------------------- écritures

    pub fn write_vram(&mut self, offset: usize, data: u8, vpos: u32, frame: u32) {
        self.render_until(vpos, frame);
        self.vram[offset & 0x3ff] = data;
    }

    pub fn write_objram(&mut self, offset: usize, data: u8, vpos: u32, frame: u32) {
        self.render_until(vpos, frame);
        self.objram[offset & 0xff] = data;
    }

    pub fn write_flip_x(&mut self, data: u8, vpos: u32, frame: u32) {
        let new = data & 1 != 0;
        if self.flip_x != new {
            self.render_until(vpos, frame);
            self.stars_update_origin(frame);
            self.flip_x = new;
        }
    }

    pub fn write_flip_y(&mut self, data: u8, vpos: u32, frame: u32) {
        let new = data & 1 != 0;
        if self.flip_y != new {
            self.render_until(vpos, frame);
            self.flip_y = new;
        }
    }

    /// `hpos` en cycles d'horloge maître (0..1151), comme `screen->hpos()` de MAME.
    pub fn write_stars_enable(&mut self, data: u8, vpos: u32, hpos: u32, frame: u32) {
        let new = data & 1 != 0;
        if self.stars_enabled != new {
            self.render_until(vpos, frame);
        }
        if !self.stars_enabled && new {
            let v = STAR_RNG_PERIOD as i32 - (vpos as i32 * 512 + hpos as i32);
            self.star_origin = v as u32; // débordement volontaire (u32), comme MAME
            self.star_origin_frame = frame;
        }
        self.stars_enabled = new;
    }

    // ---------------------------------------------------------------- rendu

    fn render_row(&mut self, y: u32) {
        let mut line = [0u32; FB_WIDTH];
        if self.stars_enabled {
            self.draw_stars(y, &mut line);
        }
        self.draw_tiles(y, &mut line);
        self.draw_sprites(y, &mut line);
        self.draw_bullets(y, &mut line);
        let off = (y - FIRST_VISIBLE_LINE) as usize * FB_WIDTH;
        self.fb[off..off + FB_WIDTH].copy_from_slice(&line);
    }

    /// CRC32 du framebuffer, octets R,G,B de chaque pixel dans l'ordre ligne par ligne.
    pub fn frame_crc32(&self) -> u32 {
        let mut bytes = Vec::with_capacity(self.fb.len() * 3);
        for &p in &self.fb {
            bytes.push((p >> 16) as u8);
            bytes.push((p >> 8) as u8);
            bytes.push(p as u8);
        }
        crc32(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video() -> Video {
        let mut rom = RomSet::blank();
        for i in 0..8 {
            rom.gfx[8 + i] = 0xF0;
            rom.gfx[0x800 + 8 + i] = 0xF0;
        }
        rom.prom[3] = 0b0000_0111;
        Video::new(&rom)
    }

    #[test]
    fn write_affects_only_following_rows() {
        let mut v = video();
        v.vram[2 * 32] = 1; // tuile visible sur les lignes écran 16..=23
        v.start_frame();
        // écriture pendant la ligne 20 : lignes 16..=20 rendues avec l'ANCIEN état (tuile)
        v.write_vram(2 * 32, 0, 20, 1);
        // écriture pendant la ligne 21 : la ligne 21 est rendue avec l'état intermédiaire (vide)
        v.write_vram(2 * 32, 1, 21, 1);
        v.finish_frame(1);
        let row = |r: usize| v.fb[(r - 16) * FB_WIDTH];
        assert_ne!(row(16), 0);
        assert_ne!(row(20), 0);
        assert_eq!(row(21), 0);
        assert_ne!(row(22), 0);
        assert_ne!(row(23), 0);
        assert_eq!(row(24), 0);
    }

    #[test]
    fn writes_before_first_visible_line_render_nothing() {
        let mut v = video();
        v.start_frame();
        v.write_vram(0, 1, 5, 1);
        assert_eq!(v.next_row, 0);
    }

    #[test]
    fn writes_during_vblank_do_not_rerender() {
        let mut v = video();
        v.start_frame();
        v.finish_frame(1);
        v.vram[2 * 32] = 1;
        v.write_vram(5, 7, 250, 1); // pendant le VBLANK : aucun rendu
        assert!(v.fb.iter().all(|&p| p == 0));
    }

    #[test]
    fn stars_origin_wraps_like_mame() {
        let mut v = video();
        v.write_stars_enable(1, 263, 1000, 5);
        let expect = (131_071i32 - (263 * 512 + 1000)) as u32;
        assert_eq!(v.star_origin, expect);
        assert_eq!(v.star_origin_frame, 5);
        assert!(v.stars_enabled);
    }

    #[test]
    fn stars_origin_advances_minus_one_per_frame_when_not_flipped() {
        let mut v = video();
        v.star_origin = 100;
        v.star_origin_frame = 10;
        v.stars_update_origin(13);
        assert_eq!(v.star_origin, (100 + 3 * 131_071 - 3) % 131_071);
        assert_eq!(v.star_origin_frame, 13);
    }

    #[test]
    fn flip_changes_are_idempotent() {
        let mut v = video();
        v.start_frame();
        v.write_flip_x(1, 100, 1);
        assert!(v.flip_x);
        v.write_flip_x(1, 120, 1);
        assert_eq!(v.next_row, 101);
    }

    #[test]
    fn frame_crc_changes_with_content() {
        let mut v = video();
        let a = v.frame_crc32();
        v.fb[0] = 0x00FF_FFFF;
        assert_ne!(a, v.frame_crc32());
    }
}
