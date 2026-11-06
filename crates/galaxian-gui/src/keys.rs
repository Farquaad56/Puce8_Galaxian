//! Touches du clavier vers l'état des ports d'entrée de la carte.

use galaxian_core::InputState;

/// Touches appuyées à un instant donné.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pressed {
    pub coin: bool,
    pub start1: bool,
    pub start2: bool,
    pub left: bool,
    pub right: bool,
    pub fire: bool,
}

/// Convertit les touches en valeurs de ports (IN2 = réglages par défaut MAME : 0x04).
pub fn to_inputs(p: &Pressed) -> InputState {
    let mut s = InputState::default();
    if p.coin {
        s.in0 |= 0x01;
    }
    if p.left {
        s.in0 |= 0x04;
    }
    if p.right {
        s.in0 |= 0x08;
    }
    if p.fire {
        s.in0 |= 0x10;
    }
    if p.start1 {
        s.in1 |= 0x01;
    }
    if p.start2 {
        s.in1 |= 0x02;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_key_gives_mame_defaults() {
        let s = to_inputs(&Pressed::default());
        assert_eq!((s.in0, s.in1, s.in2), (0x00, 0x00, 0x04));
    }

    #[test]
    fn keys_map_to_expected_bits() {
        let p = Pressed {
            coin: true,
            start1: true,
            left: true,
            fire: true,
            ..Pressed::default()
        };
        let s = to_inputs(&p);
        assert_eq!(s.in0, 0x01 | 0x04 | 0x10);
        assert_eq!(s.in1, 0x01);
        let p2 = Pressed {
            right: true,
            start2: true,
            ..Pressed::default()
        };
        let s2 = to_inputs(&p2);
        assert_eq!(s2.in0, 0x08);
        assert_eq!(s2.in1, 0x02);
    }
}
