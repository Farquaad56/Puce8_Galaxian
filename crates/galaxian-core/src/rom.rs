//! Chargement du jeu de ROM `galaxian` (MAME 0.287) depuis galaxian.zip.
//! Aucune ROM n'est jamais écrite sur disque ni committée.

use crate::crc::crc32;
use std::fmt;
use std::io::Read;
use std::path::Path;

/// Une ROM attendue : nom de fichier dans le zip, destination et CRC32.
pub struct RomSpec {
    pub name: &'static str,
    pub offset: usize,
    pub size: usize,
    pub crc: u32,
}

/// ROM programme (région `maincpu`, 0x4000 octets).
pub const PROGRAM_ROMS: [RomSpec; 5] = [
    RomSpec {
        name: "galmidw.u",
        offset: 0x0000,
        size: 0x800,
        crc: 0x745e2d61,
    },
    RomSpec {
        name: "galmidw.v",
        offset: 0x0800,
        size: 0x800,
        crc: 0x9c999a40,
    },
    RomSpec {
        name: "galmidw.w",
        offset: 0x1000,
        size: 0x800,
        crc: 0xb5894925,
    },
    RomSpec {
        name: "galmidw.y",
        offset: 0x1800,
        size: 0x800,
        crc: 0x6b3ca10b,
    },
    RomSpec {
        name: "7l",
        offset: 0x2000,
        size: 0x800,
        crc: 0x1b933207,
    },
];
/// ROM graphiques (région `gfx1`, 0x1000 octets : plan 0 puis plan 1).
pub const GFX_ROMS: [RomSpec; 2] = [
    RomSpec {
        name: "1h.bin",
        offset: 0x0000,
        size: 0x800,
        crc: 0x39fb43a4,
    },
    RomSpec {
        name: "1k.bin",
        offset: 0x0800,
        size: 0x800,
        crc: 0x7e3f56a2,
    },
];
/// PROM de couleurs (région `proms`, 0x20 octets).
pub const PROM_ROM: RomSpec = RomSpec {
    name: "6l.bpr",
    offset: 0,
    size: 0x20,
    crc: 0xc3ac9467,
};

/// Jeu de ROM complet, en mémoire.
#[derive(Clone)]
pub struct RomSet {
    /// 0x4000 octets, 0 si non chargé.
    pub program: Vec<u8>,
    /// 0x1000 octets.
    pub gfx: Vec<u8>,
    /// 32 octets.
    pub prom: [u8; 32],
}

#[derive(Debug)]
pub enum RomError {
    Missing(String),
    BadSize {
        name: String,
        expected: usize,
        actual: usize,
    },
    BadCrc {
        name: String,
        expected: u32,
        actual: u32,
    },
    Io(String),
}

impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RomError::Missing(n) => write!(f, "ROM manquante : {n}"),
            RomError::BadSize {
                name,
                expected,
                actual,
            } => {
                write!(f, "ROM {name} : taille {actual} au lieu de {expected}")
            }
            RomError::BadCrc {
                name,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "ROM {name} : CRC32 {actual:08x} au lieu de {expected:08x}"
                )
            }
            RomError::Io(m) => write!(f, "erreur de lecture : {m}"),
        }
    }
}

impl std::error::Error for RomError {}

fn fetch(
    get: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    spec: &RomSpec,
) -> Result<Vec<u8>, RomError> {
    let data = get(spec.name).ok_or_else(|| RomError::Missing(spec.name.to_string()))?;
    if data.len() != spec.size {
        return Err(RomError::BadSize {
            name: spec.name.to_string(),
            expected: spec.size,
            actual: data.len(),
        });
    }
    let actual = crc32(&data);
    if actual != spec.crc {
        return Err(RomError::BadCrc {
            name: spec.name.to_string(),
            expected: spec.crc,
            actual,
        });
    }
    Ok(data)
}

impl RomSet {
    /// Construit le jeu de ROM à partir d'une fonction `nom -> contenu`.
    pub fn from_files(get: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> Result<RomSet, RomError> {
        let mut program = vec![0u8; 0x4000];
        for spec in &PROGRAM_ROMS {
            let d = fetch(get, spec)?;
            program[spec.offset..spec.offset + spec.size].copy_from_slice(&d);
        }
        let mut gfx = vec![0u8; 0x1000];
        for spec in &GFX_ROMS {
            let d = fetch(get, spec)?;
            gfx[spec.offset..spec.offset + spec.size].copy_from_slice(&d);
        }
        let d = fetch(get, &PROM_ROM)?;
        let mut prom = [0u8; 32];
        prom.copy_from_slice(&d);
        Ok(RomSet { program, gfx, prom })
    }

    /// Charge `galaxian.zip` (MAME 0.287). Lecture en mémoire uniquement.
    pub fn load_zip(path: &Path) -> Result<RomSet, RomError> {
        let file = std::fs::File::open(path).map_err(|e| RomError::Io(e.to_string()))?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| RomError::Io(e.to_string()))?;
        let mut get = |name: &str| -> Option<Vec<u8>> {
            let mut entry = zip.by_name(name).ok()?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).ok()?;
            Some(buf)
        };
        RomSet::from_files(&mut get)
    }

    /// Jeu de ROM synthétique (tout à zéro) pour les tests sans ROM réelle.
    pub fn blank() -> RomSet {
        RomSet {
            program: vec![0; 0x4000],
            gfx: vec![0; 0x1000],
            prom: [0; 32],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_rom_is_reported() {
        let mut get = |_: &str| -> Option<Vec<u8>> { None };
        match RomSet::from_files(&mut get) {
            Err(RomError::Missing(n)) => assert_eq!(n, "galmidw.u"),
            _ => panic!("attendu Missing"),
        }
    }

    #[test]
    fn bad_size_is_reported() {
        let mut get = |_: &str| -> Option<Vec<u8>> { Some(vec![0; 10]) };
        assert!(matches!(
            RomSet::from_files(&mut get),
            Err(RomError::BadSize { .. })
        ));
    }

    #[test]
    fn bad_crc_is_reported() {
        let mut get = |_: &str| -> Option<Vec<u8>> { Some(vec![0; 0x800]) };
        assert!(matches!(
            RomSet::from_files(&mut get),
            Err(RomError::BadCrc { .. })
        ));
    }
}
