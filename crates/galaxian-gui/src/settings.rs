//! Réglages du frontend : touches, interrupteurs DIP et rotation de l'écran (fichier texte).

use eframe::egui::Key;
use galaxian_core::InputState;
use std::path::PathBuf;

/// Actions du joueur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Coin,
    Start1,
    Start2,
    Left,
    Right,
    Fire,
}

impl Action {
    pub const ALL: [Action; 6] = [
        Action::Coin,
        Action::Start1,
        Action::Start2,
        Action::Left,
        Action::Right,
        Action::Fire,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Action::Coin => "Pièce",
            Action::Start1 => "Start 1 joueur",
            Action::Start2 => "Start 2 joueurs",
            Action::Left => "Gauche",
            Action::Right => "Droite",
            Action::Fire => "Tir",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Action::Coin => "coin",
            Action::Start1 => "start1",
            Action::Start2 => "start2",
            Action::Left => "left",
            Action::Right => "right",
            Action::Fire => "fire",
        }
    }
}

/// Une touche par action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bindings {
    keys: [Key; 6],
}

impl Default for Bindings {
    fn default() -> Bindings {
        Bindings {
            keys: [
                Key::Num5,
                Key::Num1,
                Key::Num2,
                Key::ArrowLeft,
                Key::ArrowRight,
                Key::Space,
            ],
        }
    }
}

impl Bindings {
    fn index(a: Action) -> usize {
        Action::ALL.iter().position(|&x| x == a).unwrap_or(0)
    }

    pub fn get(&self, a: Action) -> Key {
        self.keys[Self::index(a)]
    }

    pub fn set(&mut self, a: Action, k: Key) {
        self.keys[Self::index(a)] = k;
    }
}

/// Interrupteurs DIP (valeurs MAME 0.287 : coinage sur IN1, bonus et vies sur IN2, cabinet sur IN0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dip {
    /// 0 = 7000, 1 = 10000, 2 = 12000, 3 = 20000.
    pub bonus: u8,
    pub three_lives: bool,
    /// 0 = 1 pièce/1 crédit, 1 = 2 pièces/1 crédit, 2 = 1 pièce/2 crédits, 3 = jeu gratuit.
    pub coinage: u8,
    pub cocktail: bool,
    pub service_mode: bool,
}

impl Default for Dip {
    fn default() -> Dip {
        Dip {
            bonus: 0,
            three_lives: true,
            coinage: 0,
            cocktail: false,
            service_mode: false,
        }
    }
}

impl Dip {
    pub const BONUS_LABELS: [&'static str; 4] = ["7000", "10000", "12000", "20000"];
    pub const COINAGE_LABELS: [&'static str; 4] = [
        "1 pièce / 1 crédit",
        "2 pièces / 1 crédit",
        "1 pièce / 2 crédits",
        "Jeu gratuit",
    ];

    /// Ajoute les bits DIP aux ports d'entrée (IN2 est entièrement défini par les DIP).
    pub fn apply(&self, s: &mut InputState) {
        if self.cocktail {
            s.in0 |= 0x20;
        }
        if self.service_mode {
            s.in0 |= 0x40;
        }
        s.in1 |= [0x00, 0x40, 0x80, 0xC0][usize::from(self.coinage & 3)];
        s.in2 = (self.bonus & 3) | if self.three_lives { 0x04 } else { 0x00 };
    }
}

/// Rotation appliquée à l'image brute (le tube de Galaxian est monté à 90°).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    None,
    Cw90,
    Half,
    Ccw90,
}

impl Rotation {
    pub const ALL: [Rotation; 4] = [
        Rotation::None,
        Rotation::Cw90,
        Rotation::Half,
        Rotation::Ccw90,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Rotation::None => "Aucune (image brute)",
            Rotation::Cw90 => "90° horaire (écran vertical, sens de jeu)",
            Rotation::Half => "180°",
            Rotation::Ccw90 => "90° antihoraire",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Rotation::None => "none",
            Rotation::Cw90 => "cw90",
            Rotation::Half => "half",
            Rotation::Ccw90 => "ccw90",
        }
    }

    /// Vrai si largeur et hauteur affichées sont échangées.
    pub fn swaps_axes(self) -> bool {
        matches!(self, Rotation::Cw90 | Rotation::Ccw90)
    }

    /// Coordonnées de texture des coins haut-gauche, haut-droit, bas-droit, bas-gauche affichés.
    pub fn uvs(self) -> [(f32, f32); 4] {
        match self {
            Rotation::None => [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
            Rotation::Cw90 => [(0.0, 1.0), (0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],
            Rotation::Half => [(1.0, 1.0), (0.0, 1.0), (0.0, 0.0), (1.0, 0.0)],
            Rotation::Ccw90 => [(1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.0, 0.0)],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub keys: Bindings,
    pub dip: Dip,
    pub rotation: Rotation,
    /// Volume sonore 0..=100.
    pub volume: u8,
    /// Intensité des scanlines 0..=100 (0 = désactivées).
    pub scanlines: u8,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            keys: Bindings::default(),
            dip: Dip::default(),
            rotation: Rotation::Cw90,
            volume: 70,
            scanlines: 35,
        }
    }
}

impl Settings {
    /// Fichier de réglages : `%APPDATA%\puce8-galaxian\gui.cfg` (ou `~/.config/puce8-galaxian/gui.cfg`).
    pub fn path() -> Option<PathBuf> {
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("puce8-galaxian").join("gui.cfg"))
    }

    pub fn load() -> Settings {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, self.to_text());
        }
    }

    pub fn to_text(&self) -> String {
        let mut t = String::new();
        for a in Action::ALL {
            t.push_str(&format!("key.{}={}\n", a.id(), self.keys.get(a).name()));
        }
        t.push_str(&format!("dip.bonus={}\n", self.dip.bonus));
        t.push_str(&format!(
            "dip.lives={}\n",
            if self.dip.three_lives { 3 } else { 2 }
        ));
        t.push_str(&format!("dip.coinage={}\n", self.dip.coinage));
        t.push_str(&format!("dip.cocktail={}\n", u8::from(self.dip.cocktail)));
        t.push_str(&format!(
            "dip.service={}\n",
            u8::from(self.dip.service_mode)
        ));
        t.push_str(&format!("rotation={}\n", self.rotation.id()));
        t.push_str(&format!("volume={}\n", self.volume));
        t.push_str(&format!("scanlines={}\n", self.scanlines));
        t
    }

    /// Lit un fichier de réglages ; toute ligne invalide est ignorée (valeur par défaut conservée).
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            if let Some(id) = k.strip_prefix("key.") {
                if let (Some(a), Some(key)) = (
                    Action::ALL.iter().copied().find(|a| a.id() == id),
                    Key::from_name(v),
                ) {
                    s.keys.set(a, key);
                }
                continue;
            }
            match k {
                "dip.bonus" => {
                    if let Ok(n) = v.parse::<u8>() {
                        s.dip.bonus = n.min(3);
                    }
                }
                "dip.lives" => s.dip.three_lives = v != "2",
                "dip.coinage" => {
                    if let Ok(n) = v.parse::<u8>() {
                        s.dip.coinage = n.min(3);
                    }
                }
                "dip.cocktail" => s.dip.cocktail = v == "1",
                "dip.service" => s.dip.service_mode = v == "1",
                "rotation" => {
                    if let Some(r) = Rotation::ALL.iter().copied().find(|r| r.id() == v) {
                        s.rotation = r;
                    }
                }
                "volume" => {
                    if let Ok(n) = v.parse::<u32>() {
                        s.volume = n.min(100) as u8;
                    }
                }
                "scanlines" => {
                    if let Ok(n) = v.parse::<u32>() {
                        s.scanlines = n.min(100) as u8;
                    }
                }
                _ => {}
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dip_matches_mame_defaults() {
        let mut s = InputState::default();
        Dip::default().apply(&mut s);
        assert_eq!((s.in0, s.in1, s.in2), (0x00, 0x00, 0x04));
    }

    #[test]
    fn dip_bits() {
        let d = Dip {
            bonus: 2,
            three_lives: false,
            coinage: 3,
            cocktail: true,
            service_mode: true,
        };
        let mut s = InputState::default();
        d.apply(&mut s);
        assert_eq!((s.in0, s.in1, s.in2), (0x60, 0xC0, 0x02));
    }

    #[test]
    fn text_roundtrip() {
        let mut s = Settings::default();
        s.keys.set(Action::Fire, Key::Z);
        s.keys.set(Action::Coin, Key::C);
        s.dip.bonus = 3;
        s.dip.three_lives = false;
        s.dip.coinage = 2;
        s.rotation = Rotation::Ccw90;
        assert_eq!(Settings::parse(&s.to_text()), s);
    }

    #[test]
    fn invalid_lines_keep_defaults() {
        let s = Settings::parse("n'importe quoi\nkey.fire=Inconnue\ndip.bonus=99\nrotation=xx\n");
        assert_eq!(s.keys.get(Action::Fire), Key::Space);
        assert_eq!(s.dip.bonus, 3);
        assert_eq!(s.rotation, Rotation::Cw90);
    }

    #[test]
    fn rotation_uvs_are_permutations_of_the_corners() {
        for r in Rotation::ALL {
            let mut v = r.uvs().to_vec();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            assert_eq!(v, vec![(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)]);
        }
    }

    #[test]
    fn volume_and_scanlines_roundtrip_and_are_clamped() {
        let s = Settings {
            volume: 25,
            scanlines: 80,
            ..Settings::default()
        };
        let back = Settings::parse(&s.to_text());
        assert_eq!((back.volume, back.scanlines), (25, 80));
        let c = Settings::parse("volume=250\nscanlines=300\n");
        assert_eq!((c.volume, c.scanlines), (100, 100));
    }
}
