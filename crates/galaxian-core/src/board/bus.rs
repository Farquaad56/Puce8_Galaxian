use super::Board;
use crate::consts::*;
use puce_z80::{Bus, Pins};

impl Bus for Board {
    fn read(&mut self, addr: u16) -> u8 {
        let v = match addr {
            0x0000..=0x3fff => self.program[addr as usize],
            0x4000..=0x47ff => self.ram[(addr & 0x3ff) as usize],
            0x5000..=0x57ff => self.video.vram[(addr & 0x3ff) as usize],
            0x5800..=0x5fff => self.video.objram[(addr & 0xff) as usize],
            0x6000..=0x67ff => self.inputs.in0,
            0x6800..=0x6fff => self.inputs.in1,
            0x7000..=0x77ff => self.inputs.in2,
            0x7800..=0x7fff => {
                self.watchdog_count = 0;
                0xff
            }
            _ => 0xff,
        };
        self.record(addr, v, false);
        v
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.record(addr, data, true);
        let (vpos, hpos, frame) = self.bus_stamp();
        match addr {
            0x4000..=0x47ff => self.ram[(addr & 0x3ff) as usize] = data,
            0x5000..=0x57ff => self
                .video
                .write_vram((addr & 0x3ff) as usize, data, vpos, frame),
            0x5800..=0x5fff => self
                .video
                .write_objram((addr & 0xff) as usize, data, vpos, frame),
            0x6000..=0x67ff => match addr & 7 {
                0 => self.start_lamp[0] = data & 1 != 0,
                1 => self.start_lamp[1] = data & 1 != 0,
                2 => self.coin_lockout_raw = data & 1 != 0,
                3 => self.coin_counter = data & 1 != 0,
                n => {
                    self.lfo[(n - 4) as usize] = data & 1 != 0;
                    let mut v = 0u8;
                    for (i, &b) in self.lfo.iter().enumerate() {
                        v |= u8::from(b) << i;
                    }
                    self.audio.regs.lfo = v;
                }
            },
            0x6800..=0x6fff => {
                let n = (addr & 7) as usize;
                let b = data & 1 != 0;
                self.sound_latch[n] = b;
                match n {
                    0..=2 => self.audio.regs.fs[n] = b,
                    3 => self.audio.regs.hit = b,
                    5 => self.audio.regs.fire = b,
                    6 | 7 => self.audio.regs.vol[n - 6] = b,
                    _ => {}
                }
            }
            0x7000..=0x77ff => match addr & 7 {
                1 => self.irq_enable_w(data),
                4 => {
                    self.video.write_stars_enable(data, vpos, hpos, frame);
                }
                6 => self.video.write_flip_x(data, vpos, frame),
                7 => self.video.write_flip_y(data, vpos, frame),
                _ => {}
            },
            0x7800..=0x7fff => {
                self.pitch = data;
                self.audio.regs.pitch = data;
            }
            _ => {}
        }
    }

    fn nmi(&mut self) -> bool {
        self.nmi_line
    }

    fn on_tick(&mut self, _pins: Pins) {
        self.tstate += 1;
        if self.tstate & 63 == 0 {
            let s = self.audio.step_sample();
            self.samples.push(s);
        }
        let ft = self.frame_t();
        if ft == 0 {
            self.video.start_frame();
        }
        // Calibrage MAME 0.287 : la NMI est visible à la frontière d'instruction qui précède d'un
        // T-state le début du VBLANK (cas observé : instruction terminée à t, VBLANK à t+1,
        // NMI prise avant l'instruction suivante).
        if ft + NMI_LEAD_T == VBLANK_START_LINE * T_PER_LINE && self.irq_enabled {
            self.nmi_line = true;
        }
        if ft == VBLANK_START_LINE * T_PER_LINE {
            self.vblank_start();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::input_bits;
    use crate::rom::RomSet;

    #[test]
    fn memory_map_mirrors() {
        let mut b = Board::new(&RomSet::blank());
        b.write(0x4005, 0x12);
        assert_eq!(b.read(0x4405), 0x12); // miroir 0x400
        b.write(0x5001, 0x34);
        assert_eq!(b.read(0x5401), 0x34);
        b.write(0x5810, 0x56);
        assert_eq!(b.read(0x5910), 0x56); // objram miroir 0x100
        assert_eq!(b.read(0x4800), 0xff); // non mappé
        assert_eq!(b.read(0x8000), 0xff);
        b.write(0x0000, 0xAA); // ROM non inscriptible
        assert_eq!(b.read(0x0000), 0x00);
    }

    #[test]
    fn io_ports_and_default_dips() {
        let mut b = Board::new(&RomSet::blank());
        assert_eq!(b.read(0x6000), 0x00);
        assert_eq!(b.read(0x67ff), 0x00); // miroir complet
        assert_eq!(b.read(0x6800), 0x00);
        assert_eq!(b.read(0x7000), 0x04);
        b.inputs.in0 = input_bits::IN0_COIN1;
        assert_eq!(b.read(0x6123), 1);
    }

    #[test]
    fn latches_decode_low_three_address_bits() {
        let mut b = Board::new(&RomSet::blank());
        b.write(0x6003, 1);
        assert!(b.coin_counter);
        b.write(0x600B, 0); // miroir +8
        assert!(!b.coin_counter);
        b.write(0x6805, 1);
        assert!(b.sound_latch[5]);
        b.write(0x7800, 0x5A);
        assert_eq!(b.pitch, 0x5A);
        b.write(0x7006, 1);
        assert!(b.video.flip_x);
    }
}
