//! Carte Galaxian : mémoire, latches, interruption NMI, watchdog, temps.
//! Implémente le trait `Bus` du crate `puce-z80`.

use crate::audio::Audio;
use crate::consts::*;
use crate::input::InputState;
use crate::rom::RomSet;
use crate::video::Video;

mod bus;

/// Un accès bus journalisé (pour comparer au golden log MAME).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BusEvent {
    pub frame: u64,
    pub tstate: u64,
    pub vpos: u32,
    pub hpos: u32,
    pub addr: u16,
    pub data: u8,
    pub write: bool,
    pub pc: u16,
}

pub struct Board {
    program: Vec<u8>,
    ram: [u8; 0x400],
    pub video: Video,
    pub inputs: InputState,
    /// T-states écoulés depuis le reset.
    pub tstate: u64,
    irq_enabled: bool,
    nmi_line: bool,
    watchdog_count: u32,
    pub watchdog_fired: bool,
    pub sound_latch: [bool; 8],
    pub lfo: [bool; 4],
    pub pitch: u8,
    pub start_lamp: [bool; 2],
    pub coin_lockout_raw: bool,
    pub coin_counter: bool,
    pub frames_completed: u64,
    pub vblank_flag: bool,
    /// PC de l'instruction en cours (renseigné par `Machine`), pour le journal.
    pub cur_pc: u16,
    pub log: Option<Vec<BusEvent>>,
    pub audio: Audio,
    /// Échantillons audio 48 kHz mono (un tous les 64 T-states), vidés par l'appelant.
    pub samples: Vec<f32>,
}

impl Board {
    pub fn new(rom: &RomSet) -> Board {
        Board {
            program: rom.program.clone(),
            ram: [0; 0x400],
            video: Video::new(rom),
            inputs: InputState::default(),
            tstate: 0,
            irq_enabled: false,
            nmi_line: false,
            watchdog_count: 0,
            watchdog_fired: false,
            sound_latch: [false; 8],
            lfo: [false; 4],
            pitch: 0,
            start_lamp: [false; 2],
            coin_lockout_raw: false,
            coin_counter: false,
            frames_completed: 0,
            vblank_flag: false,
            cur_pc: 0,
            log: None,
            audio: Audio::new(),
            samples: Vec::new(),
        }
    }

    /// T-state dans l'image courante (0..50688).
    pub fn frame_t(&self) -> u32 {
        (self.tstate % u64::from(T_PER_FRAME)) as u32
    }
    /// Ligne courante (0..264), équivalent de `screen->vpos()`.
    pub fn vpos(&self) -> u32 {
        self.frame_t() / T_PER_LINE
    }
    /// Position horizontale en cycles d'horloge maître (0..1152), équivalent de `screen->hpos()`.
    pub fn hpos(&self) -> u32 {
        (self.frame_t() % T_PER_LINE) * MASTER_PER_T
    }
    /// Numéro d'image au sens de `screen->frame_number()` : incrémenté à la fin du VBLANK (ligne 16).
    pub fn mame_frame(&self) -> u32 {
        ((self.tstate + u64::from(248 * T_PER_LINE)) / u64::from(T_PER_FRAME)) as u32
    }

    fn record(&mut self, addr: u16, data: u8, write: bool) {
        if self.log.is_some() {
            let ev = BusEvent {
                frame: self.tstate / u64::from(T_PER_FRAME),
                tstate: self.tstate,
                vpos: self.vpos(),
                hpos: self.hpos(),
                addr,
                data,
                write,
                pc: self.cur_pc,
            };
            if let Some(l) = self.log.as_mut() {
                l.push(ev);
            }
        }
    }

    fn vblank_start(&mut self) {
        let frame = self.mame_frame();
        self.video.finish_frame(frame);
        self.frames_completed += 1;
        self.vblank_flag = true;
        self.watchdog_count += 1;
        if self.watchdog_count >= 8 {
            self.watchdog_fired = true;
        }
        if self.irq_enabled {
            self.nmi_line = true;
        }
    }

    fn irq_enable_w(&mut self, data: u8) {
        self.irq_enabled = data & 1 != 0;
        if !self.irq_enabled {
            self.nmi_line = false;
        }
    }
}
