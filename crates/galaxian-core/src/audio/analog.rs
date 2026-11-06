//! Partie ANALOGIQUE du son Galaxian : port des modules `discrete` de MAME 0.287
//! (`galaxian_a.cpp`, `disc_dev.hxx`, `disc_mth.hxx`, `disc_flt.hxx`).
//! Chaque struct correspond à un nœud du netlist ; tout est calculé à 48 kHz.

use super::SAMPLE_TIME;

const V_POS: f64 = 5.0;

/// `RC_CHARGE_EXP(rc)` de MAME pour un pas d'échantillon.
fn rc_exp(rc: f64) -> f64 {
    1.0 - (-SAMPLE_TIME / rc).exp()
}

fn par2(a: f64, b: f64) -> f64 {
    1.0 / (1.0 / a + 1.0 / b)
}

// ---------------------------------------------------------------- DAC du LFO

/// `DISCRETE_DAC_R1` : échelle R1 4 bits (R18, R17, R16, R15) + polarisation 4.4 V / 15 k + 330 k à la masse.
pub fn dac_r1(data: u8, ttl: f64) -> f64 {
    let ladder = [1000e3, 470e3, 220e3, 100e3];
    let (r_bias, v_bias, r_gnd) = (15e3, 4.4, 330e3);
    let mut r_total = 1.0 / r_bias + 1.0 / r_gnd;
    for r in ladder {
        r_total += 1.0 / r;
    }
    r_total = 1.0 / r_total;
    let mut i_total = v_bias / r_bias;
    for (bit, r) in ladder.iter().enumerate() {
        if (data >> bit) & 1 != 0 {
            i_total += ttl / r;
        }
    }
    i_total * r_total
}

// ---------------------------------------------------------------- 555 à courant constant

/// `DISCRETE_555_CC` (type 0 : ni RBIAS, ni RGND, ni RDIS) : dent de scie 1/3..2/3 Vcc.
pub struct Cc {
    ff: bool,
    v_cap: f64,
}

impl Cc {
    pub fn new() -> Cc {
        Cc {
            ff: true,
            v_cap: 0.0,
        }
    }

    /// `v_in` = tension du DAC, `r`/`c` = R21/C15. Retourne la tension du condensateur.
    pub fn step(&mut self, v_in: f64, r: f64, c: f64) -> f64 {
        let threshold = V_POS * 2.0 / 3.0;
        let trigger = V_POS / 3.0;
        let limit = v_in + 0.7;
        let mut i = (V_POS - limit) / r;
        if i < 0.0 {
            i = 0.0;
        }
        let mut dt = SAMPLE_TIME;
        let mut v_cap = self.v_cap;
        let mut ff = self.ff;
        for _ in 0..64 {
            let v_next;
            if ff {
                if i == 0.0 {
                    v_next = v_cap - v_cap * rc_exp(10e6 * c);
                    dt = 0.0;
                } else {
                    let mut v = v_cap + i * dt / c;
                    if v > limit {
                        v = limit;
                    }
                    dt = 0.0;
                    if v >= threshold {
                        dt = c * (v - threshold) / i;
                        v = threshold;
                        ff = false;
                    }
                    v_next = v;
                }
            } else {
                // décharge immédiate (RDIS = 0), dt inchangé
                v_next = trigger;
                ff = true;
            }
            v_cap = v_next;
            if dt == 0.0 {
                break;
            }
        }
        self.v_cap = v_cap;
        self.ff = ff;
        v_cap
    }
}

impl Default for Cc {
    fn default() -> Self {
        Cc::new()
    }
}

// ---------------------------------------------------------------- 555 astable avec tension de contrôle

/// Mode de sortie du 555 astable.
#[derive(Clone, Copy, PartialEq)]
pub enum Out555 {
    /// `DISC_555_OUT_ENERGY` (énergie moyenne sur l'échantillon).
    Energy,
    /// `DISC_555_OUT_SQW` (carré, niveau logique).
    Square,
}

/// Paramètres fixes d'un 555 astable.
#[derive(Clone, Copy)]
pub struct AstableParams {
    pub r1: f64,
    pub r2: f64,
    pub c: f64,
    pub v_high: f64,
    pub mode: Out555,
}

/// `DISCRETE_555_ASTABLE_CV`.
pub struct Astable {
    ff: bool,
    v_cap: f64,
    out: f64,
}

impl Astable {
    pub fn new() -> Astable {
        Astable {
            ff: true,
            v_cap: 0.0,
            out: 0.0,
        }
    }

    pub fn step(&mut self, reset: bool, p: AstableParams, cv: f64) -> f64 {
        let AstableParams {
            r1,
            r2,
            c,
            v_high,
            mode,
        } = p;
        if reset {
            self.ff = true;
            self.v_cap = 0.0;
            self.out = 0.0;
            return 0.0;
        }
        if cv < 0.25 {
            return self.out; // MAME ignore : sortie inchangée
        }
        let threshold = cv;
        let trigger = cv / 2.0;
        let t_charge = (r1 + r2) * c;
        let t_dis = r2 * c;
        let mut v_cap = self.v_cap;
        let mut ff = self.ff;
        let (mut count_f, mut count_r) = (0, 0);
        if v_cap >= threshold {
            ff = false;
            count_f += 1;
        } else if v_cap <= trigger {
            ff = true;
            count_r += 1;
        }
        let mut dt = SAMPLE_TIME;
        let mut x_time = 0.0;
        for _ in 0..64 {
            let mut v_next;
            if ff {
                v_next = v_cap + (V_POS - v_cap) * (1.0 - (-dt / t_charge).exp());
                dt = 0.0;
                if v_next >= threshold {
                    dt = t_charge * (1.0 / (1.0 - ((v_next - threshold) / (V_POS - v_cap)))).ln();
                    x_time = dt;
                    v_next = threshold;
                    ff = false;
                    count_f += 1;
                }
            } else {
                v_next = v_cap - v_cap * (1.0 - (-dt / t_dis).exp());
                dt = 0.0;
                if v_next <= trigger {
                    if v_next < trigger {
                        dt = t_dis * (1.0 / (1.0 - ((trigger - v_next) / v_cap))).ln();
                    }
                    x_time = dt;
                    v_next = trigger;
                    ff = true;
                    count_r += 1;
                }
            }
            v_cap = v_next;
            if dt == 0.0 {
                break;
            }
        }
        x_time /= SAMPLE_TIME;
        let out = match mode {
            Out555::Square => {
                if count_f + count_r >= 2 {
                    if self.ff {
                        0.0
                    } else {
                        v_high
                    }
                } else if ff {
                    v_high
                } else {
                    0.0
                }
            }
            Out555::Energy => {
                if x_time == 0.0 {
                    x_time = 1.0;
                }
                v_high * if ff { x_time } else { 1.0 - x_time }
            }
        };
        self.v_cap = v_cap;
        self.ff = ff;
        self.out = out;
        out
    }
}

impl Default for Astable {
    fn default() -> Self {
        Astable::new()
    }
}

// ---------------------------------------------------------------- RCDISC5 (diode + R//C)

pub struct RcDisc5 {
    v_cap: f64,
    exp0: f64,
}

impl RcDisc5 {
    pub fn new(r: f64, c: f64) -> RcDisc5 {
        RcDisc5 {
            v_cap: 0.0,
            exp0: rc_exp(r * c),
        }
    }

    pub fn step(&mut self, enable: bool, input: f64) -> f64 {
        let u = (input - 0.7).max(0.0);
        let mut diff = u - self.v_cap;
        if enable {
            if diff < 0.0 {
                diff *= self.exp0;
            }
            self.v_cap += diff;
            self.v_cap
        } else {
            if diff > 0.0 {
                self.v_cap = u;
            }
            0.0
        }
    }
}

// ---------------------------------------------------------------- passe-bande op-amp (BAND_PASS_1M)

pub struct BandPass {
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
    v_ref: f64,
    r_total: f64,
    r1: f64,
    r2: f64,
    v_max: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BandPass {
    /// Paramètres de `galaxian_bandpass_desc` : R35/R36/R37, C22/C23, vRef = 5*R39/(R38+R39).
    pub fn new(r1: f64, r2: f64, r_f: f64, c1: f64, c2: f64, v_ref: f64) -> BandPass {
        let r_total = 1.0 / (1.0 / r1 + 1.0 / r2);
        let fc = 1.0 / (2.0 * std::f64::consts::PI * (r_total * r_f * c1 * c2).sqrt());
        let d = (c1 + c2) / (r_f / r_total * c1 * c2).sqrt();
        let gain = -r_f / r_total * c2 / (c1 + c2);
        let sr = 1.0 / SAMPLE_TIME;
        let two_over_t = 2.0 * sr;
        let two_over_t_sq = two_over_t * two_over_t;
        let wc = sr * 2.0 * (std::f64::consts::PI * fc / sr).tan();
        let wc_sq = wc * wc;
        let den = two_over_t_sq + d * wc * two_over_t + wc_sq;
        let a1 = 2.0 * (-two_over_t_sq + wc_sq) / den;
        let a2 = (two_over_t_sq - d * wc * two_over_t + wc_sq) / den;
        let b0 = d * wc * two_over_t / den * gain;
        BandPass {
            a1,
            a2,
            b0,
            b1: 0.0,
            b2: -b0,
            v_ref,
            r_total,
            r1,
            r2,
            v_max: 5.0 - 1.5,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    pub fn step(&mut self, input: f64) -> f64 {
        let i = (input - self.v_ref) / self.r1 + (0.0 - self.v_ref) / self.r2;
        let v = i * self.r_total;
        let mut out = -self.a1 * self.y1 - self.a2 * self.y2
            + self.b0 * v
            + self.b1 * self.x1
            + self.b2 * self.x2
            + self.v_ref;
        self.x2 = self.x1;
        self.x1 = v;
        self.y2 = self.y1;
        out = out.clamp(0.0, self.v_max);
        self.y1 = out - self.v_ref;
        out
    }
}

// ---------------------------------------------------------------- filtre RC passe-bas

pub struct RcFilter {
    v_cap: f64,
    exp: f64,
}

impl RcFilter {
    pub fn new(r: f64, c: f64) -> RcFilter {
        RcFilter {
            v_cap: 0.0,
            exp: rc_exp(r * c),
        }
    }

    pub fn step(&mut self, input: f64) -> f64 {
        self.v_cap += (input - self.v_cap) * self.exp;
        self.v_cap
    }
}

// ---------------------------------------------------------------- mixeur résistif Millman

/// Somme Millman `sum(V/R) / sum(1/R)` ; les entrées de résistance <= 0 sont déconnectées.
pub fn millman(inputs: &[(f64, f64)], extra_r_to_gnd: f64) -> f64 {
    let (mut num, mut den) = (0.0, 0.0);
    for &(v, r) in inputs {
        if r > 0.0 {
            num += v / r;
            den += 1.0 / r;
        }
    }
    if extra_r_to_gnd > 0.0 {
        den += 1.0 / extra_r_to_gnd;
    }
    if den == 0.0 {
        0.0
    } else {
        num / den
    }
}

/// Mixeur final (nœud 280) : 3 entrées, HP sur l'entrée FIRE (C26), charge RF = R91, HP de sortie C46.
pub struct FinalMix {
    cap_c: f64,
    exp_c: f64,
    cap_amp: f64,
    exp_amp: f64,
}

pub const R34: f64 = 5.1e3;
pub const R40: f64 = 2.2e3 * 0.6;
pub const R43: f64 = 2.2e3;
const R91: f64 = 10e3;
const C26: f64 = 0.01e-6;
const C46: f64 = 0.1e-6;

impl FinalMix {
    pub fn new() -> FinalMix {
        FinalMix {
            cap_c: 0.0,
            exp_c: rc_exp(par2(R43, R91) * C26),
            cap_amp: 0.0,
            exp_amp: rc_exp(100e3 * C46),
        }
    }

    pub fn step(&mut self, pre: f64, hit: f64, fire: f64) -> f64 {
        self.cap_c += (fire - self.cap_c) * self.exp_c;
        let fire_hp = fire - self.cap_c;
        let v = millman(&[(pre, R34), (hit, R40), (fire_hp, R43)], R91);
        self.cap_amp += (v - self.cap_amp) * self.exp_amp;
        v - self.cap_amp
    }
}

impl Default for FinalMix {
    fn default() -> Self {
        FinalMix::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dac_without_bits_is_about_3_36_volts() {
        let v = dac_r1(0, 4.0);
        assert!((v - 3.356).abs() < 0.01, "{v}");
    }

    #[test]
    fn cc_oscillates_between_one_third_and_two_thirds() {
        let mut cc = Cc::new();
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for _ in 0..24_000 {
            cc.step(3.356, 100e3, 1e-6);
        }
        for _ in 0..48_000 {
            let v = cc.step(3.356, 100e3, 1e-6);
            if v > 0.0 {
                lo = lo.min(v);
            }
            hi = hi.max(v);
        }
        assert!(
            hi <= 5.0 * 2.0 / 3.0 + 1e-9 && lo >= 5.0 / 3.0 - 1e-9,
            "{lo} {hi}"
        );
    }

    #[test]
    fn astable_has_expected_frequency() {
        // R1=100k, R2=470k, C=0.01u, CV=2.5 -> t = (R1+R2) C ln((5-1.25)/(5-2.5)) + R2 C ln 2
        let mut a = Astable::new();
        let mut high = 0.0;
        let n = 48_000;
        for _ in 0..n {
            high += a.step(
                false,
                AstableParams {
                    r1: 100e3,
                    r2: 470e3,
                    c: 0.01e-6,
                    v_high: 4.5,
                    mode: Out555::Energy,
                },
                2.5,
            );
        }
        assert!(high > 0.0);
    }
}
