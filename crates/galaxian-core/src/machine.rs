//! Le CPU (puce-z80) et la carte ensemble.

use crate::board::Board;
use crate::rom::RomSet;
use puce_z80::Cpu;

/// Le CPU et la carte ensemble.
pub struct Machine {
    pub cpu: Cpu,
    pub board: Board,
}

impl Machine {
    pub fn new(rom: &RomSet) -> Machine {
        let mut cpu = Cpu::new();
        // État initial non défini par le matériel : valeurs observées sous MAME 0.287
        // (trace débogueur, première instruction de la ROM) pour que les golden logs concordent.
        cpu.regs.ix = 0xFFFF;
        cpu.regs.iy = 0xFFFF;
        cpu.regs.sp = 0x0000;
        Machine {
            cpu,
            board: Board::new(rom),
        }
    }

    /// Exécute une instruction ; retourne les T-states consommés.
    pub fn step(&mut self) -> u32 {
        self.board.cur_pc = self.cpu.regs.pc;
        self.cpu.step(&mut self.board)
    }

    /// Comme `run_frame`, mais appelle `hook` AVANT chaque instruction (traces).
    pub fn run_frame_hook(&mut self, hook: &mut dyn FnMut(&Machine)) {
        self.board.vblank_flag = false;
        while !self.board.vblank_flag {
            hook(self);
            self.step();
        }
    }

    /// Exécute jusqu'au prochain début de VBLANK : le framebuffer est alors complet.
    pub fn run_frame(&mut self) {
        self.board.vblank_flag = false;
        while !self.board.vblank_flag {
            self.step();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use puce_z80::Bus;

    fn rom_with_program(code: &[u8]) -> RomSet {
        let mut r = RomSet::blank();
        r.program[..code.len()].copy_from_slice(code);
        r
    }

    #[test]
    fn frame_has_exactly_50688_tstates_between_vblanks() {
        // JR $ (boucle infinie) : 12 T-states par itération
        let mut m = Machine::new(&rom_with_program(&[0x18, 0xFE]));
        m.run_frame();
        let t0 = m.board.tstate;
        m.run_frame();
        let t1 = m.board.tstate;
        // le vblank est détecté à la fin de l'instruction : écart < 12 T-states par rapport à 50688
        let d = t1 - t0;
        assert!((50_688 - 12..=50_688 + 12).contains(&d), "écart {d}");
        assert_eq!(m.board.frames_completed, 2);
    }

    #[test]
    fn first_vblank_is_at_line_240() {
        let mut m = Machine::new(&rom_with_program(&[0x18, 0xFE]));
        m.run_frame();
        assert_eq!(m.board.tstate / 192, 240); // 46080 T-states
    }

    #[test]
    fn nmi_is_raised_only_when_enabled_and_cleared_on_disable() {
        // LD A,1 ; LD (7001h),A ; JR $
        let code = [0x3E, 0x01, 0x32, 0x01, 0x70, 0x18, 0xFE];
        let mut m = Machine::new(&rom_with_program(&code));
        assert!(!m.board.nmi());
        m.run_frame();
        assert!(m.board.nmi());
        m.board.write(0x7001, 0);
        assert!(!m.board.nmi());
    }

    #[test]
    fn one_audio_sample_every_64_tstates() {
        let mut m = Machine::new(&rom_with_program(&[0x18, 0xFE]));
        m.run_frame();
        assert_eq!(m.board.samples.len() as u64, m.board.tstate / 64);
    }

    #[test]
    fn watchdog_fires_after_8_vblanks_without_kick() {
        let mut m = Machine::new(&rom_with_program(&[0x18, 0xFE]));
        for _ in 0..8 {
            m.run_frame();
        }
        assert!(m.board.watchdog_fired);
        let mut m = Machine::new(&rom_with_program(&[0x3A, 0x00, 0x78, 0x18, 0xFB])); // LD A,(7800h); JR -5
        for _ in 0..20 {
            m.run_frame();
        }
        assert!(!m.board.watchdog_fired);
    }
}
