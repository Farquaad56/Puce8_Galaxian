//! Son Galaxian. Un échantillon (48 kHz) est produit tous les 64 T-states (3 072 000 / 48 000).
//!
//! Partie NUMÉRIQUE exacte (port de MAME discrete) : `noise`, `pitch`.
//! Partie ANALOGIQUE : port des modules discrets de MAME 0.287 (voir `analog`) :
//! DAC LFO -> 555 CC -> tension de commande des 3 oscillateurs 555 de fond, mixeurs Millman,
//! HIT (RCDISC5 + passe-bande), FIRE (RC + 555 + RCDISC5), mixeur final avec passe-haut.

mod analog;
mod noise;
mod pitch;

pub use noise::Noise;
pub use pitch::Pitch;

use analog::{Astable, AstableParams, BandPass, Cc, FinalMix, Out555, RcDisc5, RcFilter};

pub const SAMPLE_RATE: u32 = 48_000;
pub(crate) const SAMPLE_TIME: f64 = 1.0 / SAMPLE_RATE as f64;
/// Signal « 2V » : 60 * 264 / 2.
const SQUARE_2V_HZ: f64 = 7920.0;
const TTL_OUT: f64 = 4.0;

/// État de tous les registres sonores écrits par le CPU.
#[derive(Clone, Copy, Default)]
pub struct SoundRegs {
    /// FS1..FS3 (6800..6802) : activent les 3 oscillateurs 555 de fond.
    pub fs: [bool; 3],
    /// HIT (6803).
    pub hit: bool,
    /// FIRE (6805).
    pub fire: bool,
    /// VOL1, VOL2 (6806, 6807).
    pub vol: [bool; 2],
    /// Latch de pitch (7800).
    pub pitch: u8,
    /// DAC de fond 4 bits (6004..6007).
    pub lfo: u8,
}

pub struct Audio {
    pub regs: SoundRegs,
    noise: Noise,
    pitch: Pitch,
    sq_phase: f64,
    sq_last: bool,
    noise_sampled: bool,
    // fond
    lfo_cc: Cc,
    bg_osc: [Astable; 3],
    bg_lp: f64,
    // HIT
    hit_rc: RcDisc5,
    hit_bp: BandPass,
    // FIRE
    fire_rc: RcFilter,
    fire_osc: Astable,
    fire_out: RcDisc5,
    // mixeur final
    mix: FinalMix,
}

const R21: f64 = 100e3;
const C15: f64 = 1e-6;
/// R1, R2, C des trois 555 de fond.
const BG_RC: [(f64, f64, f64); 3] = [
    (100e3, 470e3, 0.01e-6),
    (100e3, 330e3, 0.01e-6),
    (100e3, 220e3, 0.01e-6),
];
/// MULTADD du nœud 110 : R33 / (R31 || R32 || R33), puis -5 * R33 / R31.
const MULT: f64 = 10e3 * (1.0 / 47e3 + 1.0 / 47e3 + 1.0 / 10e3);
const ADD: f64 = -5.0 * 10e3 / 47e3;
/// Mélange des trois sorties 555 : 3 x 10 k, puis passe-bas C20 = 0,1 µF sur 10k/3.
const BG_MIX_TAU: f64 = (10e3 / 3.0) * 0.1e-6;

impl Audio {
    pub fn new() -> Audio {
        Audio {
            regs: SoundRegs::default(),
            noise: Noise::new(),
            pitch: Pitch::new(0),
            sq_phase: 0.0,
            sq_last: false,
            noise_sampled: false,
            lfo_cc: Cc::new(),
            bg_osc: [Astable::new(), Astable::new(), Astable::new()],
            bg_lp: 0.0,
            hit_rc: RcDisc5::new(150e3 + 22e3, 2.2e-6),
            hit_bp: BandPass::new(
                150e3,
                22e3,
                470e3,
                0.01e-6,
                0.01e-6,
                5.0 * 22e3 / (33e3 + 22e3),
            ),
            fire_rc: RcFilter::new(2.2e3, 47e-6),
            fire_osc: Astable::new(),
            fire_out: RcDisc5::new(100e3, 1e-6),
            mix: FinalMix::new(),
        }
    }

    /// Produit UN échantillon (à appeler tous les 64 T-states). Sortie dans -1.0..=1.0.
    pub fn step_sample(&mut self) -> f32 {
        // --- partie numérique exacte
        let noise = self.noise.step_sample();
        self.sq_phase = (self.sq_phase + SQUARE_2V_HZ * SAMPLE_TIME).fract();
        let sq = self.sq_phase < 0.5;
        if !self.sq_last && sq {
            self.noise_sampled = noise; // bascule D : échantillonne le bruit sur front montant de 2V
        }
        self.sq_last = sq;
        let count2 = self.pitch.step_sample(self.regs.pitch);
        let bit = |n: u8| f64::from((count2 >> n) & 1) * TTL_OUT;
        let n152 = if self.noise_sampled { 1.0 } else { 0.0 };

        // --- fond : DAC -> 555 CC -> MULTADD/CLAMP -> 3 x 555 astable -> mixeur + passe-bas
        let v_dac = analog::dac_r1(self.regs.lfo, TTL_OUT);
        let v_cap = self.lfo_cc.step(v_dac, R21, C15);
        let cv = (v_cap * MULT + ADD).clamp(0.0, 5.0);
        let mut outs = [0.0; 3];
        for (i, osc) in self.bg_osc.iter_mut().enumerate() {
            let (r1, r2, c) = BG_RC[i];
            let p = AstableParams {
                r1,
                r2,
                c,
                v_high: 5.0 - 0.5,
                mode: Out555::Energy,
            };
            outs[i] = osc.step(!self.regs.fs[i], p, cv);
        }
        let bg_raw = (outs[0] + outs[1] + outs[2]) / 3.0;
        self.bg_lp += (bg_raw - self.bg_lp) * (1.0 - (-SAMPLE_TIME / BG_MIX_TAU).exp());

        // --- pré-mixeur (nœud 279)
        let vol1_r = if self.regs.vol[0] { 10e3 } else { 0.0 };
        let vol2_r = if self.regs.vol[1] { 15e3 } else { 0.0 };
        let pre = analog::millman(
            &[
                (bit(0), 33e3),
                (bit(2), vol1_r),
                (bit(2), 22e3),
                (bit(3), vol2_r),
                (self.bg_lp, 5.1e3),
            ],
            0.0,
        );

        // --- HIT
        // DISCRETE_RCDISC5(node, ENAB = bruit 152, INP = HIT * 4 V)
        let hit_in = if self.regs.hit { TTL_OUT } else { 0.0 };
        let n155 = self.hit_rc.step(n152 != 0.0, hit_in);
        let n157 = self.hit_bp.step(n155);

        // --- FIRE
        let fire = self.regs.fire;
        let n171 = if fire { TTL_OUT } else { 0.0 };
        let n172 = if fire { 0.0 } else { TTL_OUT };
        let n173 = self.fire_rc.step(n172);
        let r46 = 10e3;
        let r48 = 2.2e3;
        let n177 = n152 * TTL_OUT / r46 + n173 / r48;
        let n178 = n177 * (r46 * r48 / (r46 + r48));
        let n181 = self.fire_osc.step(
            false,
            AstableParams {
                r1: 10e3,
                r2: 22e3,
                c: 0.01e-6,
                v_high: 1.0,
                mode: Out555::Square,
            },
            n178,
        );
        let n182 = self.fire_out.step(n181 != 0.0, n171);

        // --- mixeur final + passe-haut
        let out = self.mix.step(pre, n157, n182);
        out.clamp(-1.0, 1.0) as f32
    }
}

impl Default for Audio {
    fn default() -> Self {
        Audio::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_when_everything_is_off_settles() {
        let mut a = Audio::new();
        for _ in 0..48_000 {
            a.step_sample();
        }
        let mut max_dev = 0.0f32;
        for _ in 0..4_000 {
            max_dev = max_dev.max(a.step_sample().abs());
        }
        assert!(max_dev < 0.2, "sortie instable : {max_dev}");
    }

    #[test]
    fn output_is_always_finite_and_bounded() {
        let mut a = Audio::new();
        a.regs.fs = [true; 3];
        a.regs.hit = true;
        a.regs.fire = true;
        a.regs.vol = [true; 2];
        a.regs.pitch = 0x40;
        a.regs.lfo = 5;
        for _ in 0..100_000 {
            let s = a.step_sample();
            assert!(s.is_finite() && s.abs() <= 1.0);
        }
    }
}
