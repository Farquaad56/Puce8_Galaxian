//! Formats de trace texte (communs à l'émulateur et aux scripts MAME).
//!
//! CPU  : `C <frame> <vpos> <hpos> <tstate> <pc> <af> <bc> <de> <hl> <ix> <iy> <sp>`
//! Bus  : `B <frame> <vpos> <hpos> <R|W> <addr> <data> <pc>`
//! Vidéo: `V <index> <crc32>`
//! Les nombres hexadécimaux sont en majuscules sans préfixe (pc = 4 chiffres, data = 2 chiffres).

use crate::board::BusEvent;
use crate::machine::Machine;

/// Ligne `C` pour l'état du CPU avant l'exécution de l'instruction suivante.
pub fn cpu_line(m: &Machine) -> String {
    let r = &m.cpu.regs;
    let b = &m.board;
    format!(
        "C {} {} {} {} {:04X} {:04X} {:04X} {:04X} {:04X} {:04X} {:04X} {:04X}",
        b.tstate / u64::from(crate::consts::T_PER_FRAME),
        b.vpos(),
        b.hpos(),
        b.tstate,
        r.pc,
        r.af(),
        r.bc(),
        r.de(),
        r.hl(),
        r.ix,
        r.iy,
        r.sp
    )
}

/// Ligne `B` pour un accès bus.
pub fn bus_line(e: &BusEvent) -> String {
    format!(
        "B {} {} {} {} {:04X} {:02X} {:04X}",
        e.frame,
        e.vpos,
        e.hpos,
        if e.write { 'W' } else { 'R' },
        e.addr,
        e.data,
        e.pc
    )
}

/// Ligne `V` : CRC32 d'une image complète.
pub fn video_line(index: u64, crc: u32) -> String {
    format!("V {index} {crc:08X}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::RomSet;

    #[test]
    fn cpu_line_has_13_fields() {
        let m = Machine::new(&RomSet::blank());
        let l = cpu_line(&m);
        assert_eq!(l.split_whitespace().count(), 13);
        assert!(l.starts_with("C 0 0 0 0 0000"));
    }

    #[test]
    fn bus_line_format() {
        let e = BusEvent {
            frame: 1,
            tstate: 0,
            vpos: 2,
            hpos: 3,
            addr: 0x7001,
            data: 1,
            write: true,
            pc: 0x1234,
        };
        assert_eq!(bus_line(&e), "B 1 2 3 W 7001 01 1234");
        assert_eq!(video_line(7, 0xABCDEF), "V 7 00ABCDEF");
    }
}
