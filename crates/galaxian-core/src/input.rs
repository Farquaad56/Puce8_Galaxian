//! Entrées joueur : 3 ports actifs à l'état haut (IN0, IN1, IN2).

/// État des 3 ports d'entrée (actifs à l'état haut). Valeurs par défaut MAME : IN2 = 0x04 (3 vies).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputState {
    pub in0: u8,
    pub in1: u8,
    pub in2: u8,
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            in0: 0x00,
            in1: 0x00,
            in2: 0x04,
        }
    }
}

pub mod input_bits {
    pub const IN0_COIN1: u8 = 0x01;
    pub const IN0_COIN2: u8 = 0x02;
    pub const IN0_LEFT: u8 = 0x04;
    pub const IN0_RIGHT: u8 = 0x08;
    pub const IN0_FIRE: u8 = 0x10;
    pub const IN0_COCKTAIL: u8 = 0x20;
    pub const IN0_SERVICE_MODE: u8 = 0x40;
    pub const IN0_SERVICE1: u8 = 0x80;
    pub const IN1_START1: u8 = 0x01;
    pub const IN1_START2: u8 = 0x02;
}

/// Script d'entrées : lignes `frame in0 in1 in2` (hexa, `#` = commentaire).
/// L'état s'applique à partir de l'image `frame` (appliqué au VBLANK de l'image précédente)
/// et reste actif jusqu'à la ligne suivante.
#[derive(Clone, Debug, Default)]
pub struct InputScript {
    steps: Vec<(u64, InputState)>,
}

impl InputScript {
    pub fn parse(text: &str) -> Result<InputScript, String> {
        let mut steps = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.len() != 4 {
                return Err(format!("ligne {}: attendu `frame in0 in1 in2`", n + 1));
            }
            let frame: u64 = f[0]
                .parse()
                .map_err(|_| format!("ligne {}: frame invalide", n + 1))?;
            let hex = |s: &str| {
                u8::from_str_radix(s, 16).map_err(|_| format!("ligne {}: hexa invalide", n + 1))
            };
            steps.push((
                frame,
                InputState {
                    in0: hex(f[1])?,
                    in1: hex(f[2])?,
                    in2: hex(f[3])?,
                },
            ));
        }
        steps.sort_by_key(|s| s.0);
        Ok(InputScript { steps })
    }

    /// État des entrées pour l'image `frame` (par défaut : `InputState::default()`).
    pub fn at(&self, frame: u64) -> InputState {
        let mut cur = InputState::default();
        for (f, st) in &self.steps {
            if *f <= frame {
                cur = *st;
            } else {
                break;
            }
        }
        cur
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_holds_state_until_next_line() {
        let s = InputScript::parse("# coin\n10 01 00 04\n20 00 00 04\n").unwrap();
        assert_eq!(s.at(0), InputState::default());
        assert_eq!(s.at(10).in0, 1);
        assert_eq!(s.at(19).in0, 1);
        assert_eq!(s.at(20).in0, 0);
    }

    #[test]
    fn script_rejects_bad_lines() {
        assert!(InputScript::parse("1 2 3").is_err());
        assert!(InputScript::parse("x 00 00 00").is_err());
    }

    #[test]
    fn defaults_match_mame() {
        let i = InputState::default();
        assert_eq!((i.in0, i.in1, i.in2), (0x00, 0x00, 0x04));
    }
}
