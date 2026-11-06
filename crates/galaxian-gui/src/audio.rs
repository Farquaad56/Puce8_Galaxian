//! Sortie audio (cpal) : les échantillons de la carte (48 kHz, un par 64 T-states) sont mis dans une
//! file puis rééchantillonnés vers la fréquence du périphérique. Un filtre passe-haut retire la
//! composante continue du signal analogique.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Fréquence des échantillons produits par la carte : 3,072 MHz / 64.
const SRC_RATE: f64 = 48_000.0;
/// Retard maximal accepté dans la file (200 ms) : au-delà, on jette les plus anciens.
const MAX_BUFFERED: usize = 9_600;
/// Gain de sortie.
const GAIN: f32 = 2.0;

struct Shared {
    queue: VecDeque<f32>,
    hp_x: f32,
    hp_y: f32,
}

pub struct AudioOut {
    _stream: cpal::Stream,
    shared: Arc<Mutex<Shared>>,
}

impl AudioOut {
    /// Ouvre le périphérique de sortie par défaut ; `None` (avec un message) si impossible.
    pub fn new() -> Option<AudioOut> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let cfg = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("audio : configuration impossible : {e}");
                return None;
            }
        };
        let shared = Arc::new(Mutex::new(Shared {
            queue: VecDeque::new(),
            hp_x: 0.0,
            hp_y: 0.0,
        }));
        let rate = f64::from(cfg.sample_rate().0);
        let channels = usize::from(cfg.channels());
        let stream_cfg: cpal::StreamConfig = cfg.clone().into();
        let stream = match cfg.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&device, &stream_cfg, &shared, rate, channels),
            cpal::SampleFormat::I16 => build::<i16>(&device, &stream_cfg, &shared, rate, channels),
            cpal::SampleFormat::U16 => build::<u16>(&device, &stream_cfg, &shared, rate, channels),
            other => {
                eprintln!("audio : format {other:?} non géré");
                return None;
            }
        };
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("audio : ouverture impossible : {e}");
                return None;
            }
        };
        if let Err(e) = stream.play() {
            eprintln!("audio : lecture impossible : {e}");
            return None;
        }
        Some(AudioOut {
            _stream: stream,
            shared,
        })
    }

    /// Ajoute des échantillons de la carte (après filtrage passe-haut).
    pub fn push(&self, samples: &[f32]) {
        let Ok(mut s) = self.shared.lock() else {
            return;
        };
        for &x in samples {
            let y = x - s.hp_x + 0.995 * s.hp_y;
            s.hp_x = x;
            s.hp_y = y;
            s.queue.push_back(y);
        }
        while s.queue.len() > MAX_BUFFERED {
            s.queue.pop_front();
        }
    }
}

fn build<T>(
    device: &cpal::Device,
    cfg: &cpal::StreamConfig,
    shared: &Arc<Mutex<Shared>>,
    rate: f64,
    channels: usize,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample + FromSample<f32>,
{
    let shared = Arc::clone(shared);
    let step = SRC_RATE / rate;
    let mut pos = 0.0_f64;
    let mut prev = 0.0_f32;
    let mut next = 0.0_f32;
    device.build_output_stream(
        cfg,
        move |data: &mut [T], _| {
            let Ok(mut s) = shared.lock() else {
                return;
            };
            for frame in data.chunks_mut(channels) {
                pos += step;
                while pos >= 1.0 {
                    prev = next;
                    next = s.queue.pop_front().unwrap_or(next * 0.999);
                    pos -= 1.0;
                }
                // interpolation linéaire entre deux échantillons source
                let v = prev + (next - prev) * pos as f32;
                let out = T::from_sample((v * GAIN).clamp(-1.0, 1.0));
                for ch in frame.iter_mut() {
                    *ch = out;
                }
            }
        },
        |e| eprintln!("audio : {e}"),
        None,
    )
}
