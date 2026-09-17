//! Sfx audio

use crate::pico8::cart::{to_byte, to_nybble};
use bevy::{
    audio::{AddAudioSource, Decodable, Source},
    prelude::*,
};
use std::num::NonZero;
use std::time::Duration;
use std::{
    borrow::Cow,
    f32,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

mod command;
mod record;
pub use command::*;
pub use record::{AudioRecorder, write_wav};

pub(crate) const SAMPLE_RATE: u32 = 22_050;
/// Pico-8 tracker tick: 183 samples at 22050 Hz (~120.49 Hz), not 22050/120.
const SAMPLES_PER_TICK: u32 = 183;
const DT: f32 = 1.0 / SAMPLE_RATE as f32;
const ANTICLICK_RAMP: f32 = 0.0025;
const NOISE_CUTOFF_SCALE: f32 = 8.858923;
/// Pico-8 WAV exports peak ~1.6% below a full 0.5-amplitude triangle.
const OUTPUT_GAIN: f32 = 16125.0 / 16383.5;

/// Pitch 33 is A-4 = 440 Hz (Pico-8 key 0..=63).
fn key_to_freq(key: f32) -> f32 {
    440.0 * f32::exp2((key - 33.0) / 12.0)
}

/// One sample of a built-in waveform. `t` / `t_phaser` are phases in `[0, 1)`.
/// Amplitudes from zepto-8, measured against Pico-8 WAV exports.
fn tonal_wave(wave: WaveForm, t: f32, t_phaser: f32) -> f32 {
    match wave {
        WaveForm::Triangle => (1.0 - (4.0 * t - 2.0).abs()) * 0.5,
        WaveForm::TiltedSaw => {
            let a = 0.875;
            let ret = if t < a {
                2.0 * t / a - 1.0
            } else {
                2.0 * (1.0 - t) / (1.0 - a) - 1.0
            };
            ret * 0.5
        }
        WaveForm::Saw => {
            let ret = if t < 0.5 { t } else { t - 1.0 };
            0.653 * ret
        }
        WaveForm::Square => {
            if t < 0.5 { 0.25 } else { -0.25 }
        }
        WaveForm::Pulse => {
            if t < 0.316 { 0.25 } else { -0.25 }
        }
        WaveForm::Organ => {
            let ret = if t < 0.5 {
                3.0 - (24.0 * t - 6.0).abs()
            } else {
                1.0 - (16.0 * t - 12.0).abs()
            };
            ret / 9.0
        }
        WaveForm::Phaser => {
            let mut ret = 2.0 - (8.0 * t - 4.0).abs();
            ret += 1.0 - (4.0 * t_phaser - 2.0).abs();
            ret / 6.0
        }
        WaveForm::Noise | WaveForm::Custom(_) => 0.0,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaveForm {
    Triangle,
    TiltedSaw,
    Saw,
    Square,
    Pulse,
    Organ,
    Noise,
    Phaser,
    Custom(u8),
}

#[derive(Resource, Debug, Reflect, Deref)]
pub struct SfxChannels(pub Vec<Entity>);

#[derive(Component, Debug, Reflect)]
pub struct SfxLoop {
    release: AtomicBool,
}

impl From<WaveForm> for u8 {
    fn from(wave: WaveForm) -> u8 {
        use WaveForm::*;
        match wave {
            Triangle => 0,
            TiltedSaw => 1,
            Saw => 2,
            Square => 3,
            Pulse => 4,
            Organ => 5,
            Noise => 6,
            Phaser => 7,
            Custom(x) => x + 7,
        }
    }
}

impl TryFrom<u8> for WaveForm {
    type Error = SfxError;
    fn try_from(value: u8) -> Result<WaveForm, SfxError> {
        use WaveForm::*;
        match value {
            // 0 => Sine,
            0 => Ok(Triangle),
            1 => Ok(TiltedSaw),
            2 => Ok(Saw),
            3 => Ok(Square),
            4 => Ok(Pulse),
            5 => Ok(Organ),
            6 => Ok(Noise),
            7 => Ok(Phaser),
            x if x <= 0xf => Ok(Custom(x - 7)),
            y => Err(SfxError::InvalidWaveForm(y)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SfxError {
    #[error("Invalid effect: {0}")]
    InvalidEffect(u8),
    #[error("Invalid wave form: {0}")]
    InvalidWaveForm(u8),
    #[error("Invalid hex: {0}")]
    InvalidHex(String),
    #[error("Missing {0}")]
    Missing(Cow<'static, str>),
}

impl TryFrom<u8> for Effect {
    type Error = SfxError;
    fn try_from(value: u8) -> Result<Effect, SfxError> {
        use Effect::*;
        match value {
            0 => Ok(None),
            1 => Ok(Slide),
            2 => Ok(Vibrato),
            3 => Ok(Drop),
            4 => Ok(FadeIn),
            5 => Ok(FadeOut),
            6 => Ok(ArpFast),
            7 => Ok(ArpSlow),
            x => Err(SfxError::InvalidEffect(x)),
        }
    }
}

impl From<Effect> for u8 {
    fn from(value: Effect) -> u8 {
        use Effect::*;
        match value {
            None => 0,
            Slide => 1,
            Vibrato => 2,
            Drop => 3,
            FadeIn => 4,
            FadeOut => 5,
            ArpFast => 6,
            ArpSlow => 7,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    // 0 none, 1 slide, 2 vibrato, 3 drop, 4 fade_in, 5 fade_out, 6 arp fast, 7
    // arp slow; arpeggio commands loop over groups of four notes at speed 2 (fast)
    // and 4 (slow)
    None,
    Slide,
    Vibrato,
    Drop,
    FadeIn,
    FadeOut,
    ArpFast,
    ArpSlow,
}

pub trait Note {
    /// This is the pitch in midi format [0, 127].
    fn pitch(&self) -> u8;
    fn wave(&self) -> WaveForm;
    /// The volume [0, 1]
    fn volume(&self) -> f32;
    fn effect(&self) -> Effect;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct Pico8Note(pub u16);

impl Pico8Note {
    pub fn new(pitch: u8, wave: WaveForm, volume: u8, effect: Effect) -> Self {
        let pitch = pitch.saturating_sub(PITCH_OFFSET);
        assert!(
            volume <= 7,
            "expected volume was greater than 7 but was {volume}"
        );
        assert!(pitch <= 63, "expected pitch <= 63 but was {pitch}");
        Pico8Note(
            (pitch & 0b0011_1111) as u16
                | ((u8::from(wave) as u16) << 6)
                | (((volume & 0b111) as u16) << 9)
                | ((u8::from(effect) as u16 & 0b111) << 12),
        )
    }

    /// Tracker key 0..=63 (C-0 .. D#-5). Pitch 33 is A-4.
    pub fn key(&self) -> u8 {
        (self.0 & 0b0011_1111) as u8
    }
}

// impl From<u8> for Pico8Note {
//     fn from(value: u8) -> Self {
//         Pico8Note::new(value, 5.0 / 7.0, WaveForm::Sine, Effect::None)
//     }
// }

impl TryFrom<&str> for Sfx {
    type Error = SfxError;
    fn try_from(line: &str) -> Result<Self, Self::Error> {
        const HEADER_NYBBLES: usize = 8;
        const NOTE_NYBBLES: usize = 5;
        let note_nybbles = line.len() - HEADER_NYBBLES;
        let empty_notes = {
            let line_bytes = line.as_bytes();
            line_bytes
                .iter()
                .rev()
                .position(|a| *a != b'0')
                .map(|index| index / NOTE_NYBBLES)
                .unwrap_or(0)
        };
        let mut notes = Vec::with_capacity(note_nybbles / NOTE_NYBBLES - empty_notes);
        let line_bytes = &line.as_bytes()[..line.len() - empty_notes * NOTE_NYBBLES];

        let mut iter = line_bytes.chunks(2).map(|v| {
            to_byte(v[0], v[1])
                .ok_or_else(|| SfxError::InvalidHex(String::from_utf8(v.to_vec()).unwrap()))
        });

        // Process the header first.
        let _editor_mode = iter.next().ok_or(SfxError::Missing("editor_mode".into()))?;
        let note_duration = iter
            .next()
            .ok_or(SfxError::Missing("note_duration".into()))?;

        let loop_start = iter
            .next()
            .ok_or(SfxError::Missing("loop_start".into()))??;
        let loop_end = iter.next().ok_or(SfxError::Missing("loop_end".into()))??;

        let mut nybbles = line_bytes
            .iter()
            .map(|a| to_nybble(*a).ok_or(SfxError::InvalidHex((*a as char).to_string())))
            .skip(HEADER_NYBBLES);

        while let Some(pitch_high) = nybbles.next() {
            let pitch_low: u8 = nybbles
                .next()
                .ok_or(SfxError::Missing("pitch low nybble".into()))??;
            let wave_form: u8 = nybbles
                .next()
                .ok_or(SfxError::Missing("wave form".into()))??;
            let volume: u8 = nybbles.next().ok_or(SfxError::Missing("volume".into()))??;
            let effect: u8 = nybbles.next().ok_or(SfxError::Missing("effect".into()))??;
            // notes.push(Pico8Note::new(pitch_high << 4 | pitch_low?, WaveForm::try_from(wave_form)?,
            notes.push(Pico8Note::new(
                ((pitch_high? << 4) | pitch_low) + PITCH_OFFSET,
                WaveForm::try_from(wave_form)?,
                volume,
                Effect::try_from(effect)?,
            ));
        }
        Ok(Sfx::new(notes).with_speed(note_duration?).with_loop(
            (loop_start != 0).then_some(loop_start),
            (loop_end != 0).then_some(loop_end),
        ))
    }
}

impl From<u16> for Pico8Note {
    fn from(value: u16) -> Self {
        Pico8Note(value)
    }
}

impl From<Pico8Note> for u16 {
    fn from(value: Pico8Note) -> Self {
        value.0
    }
}

impl Default for Pico8Note {
    fn default() -> Self {
        Pico8Note::new(32, WaveForm::Triangle, 5, Effect::None)
    }
}

const PITCH_OFFSET: u8 = 35;

impl Note for Pico8Note {
    fn pitch(&self) -> u8 {
        self.key() + PITCH_OFFSET
    }

    fn wave(&self) -> WaveForm {
        WaveForm::try_from(((self.0 >> 6) & 0b111) as u8).unwrap()
    }

    fn volume(&self) -> f32 {
        ((self.0 >> 9) & 0b111) as f32 / 7.0
    }

    fn effect(&self) -> Effect {
        Effect::try_from(((self.0 >> 12) & 0b111) as u8).unwrap()
    }
}

// This struct usually contains the data for the audio being played.
// This is where data read from an audio file would be stored, for example.
// This allows the type to be registered as an asset.
#[derive(Asset, Clone, Default, Debug, Reflect)]
pub struct Sfx {
    pub notes: Vec<Pico8Note>,
    pub speed: u8,
    pub loop_maybe: Option<Loop>,
}

#[derive(Debug, Clone, Reflect)]
pub enum Loop {
    Unstoppable {
        start: Option<u8>,
        end: Option<u8>,
    },
    Stoppable {
        start: Option<u8>,
        end: Option<u8>,
        release: Arc<AtomicBool>,
    },
}

impl Sfx {
    pub fn new(notes: impl IntoIterator<Item = Pico8Note>) -> Self {
        Sfx {
            notes: notes.into_iter().collect(),
            speed: 16,
            loop_maybe: None,
        }
    }

    pub fn from_u8(data: &[u8]) -> Self {
        let n = data.len();
        let note_end = n - 4;
        let mut notes: Vec<_> = data[0..note_end]
            .chunks(2)
            .map(|pair| Pico8Note(((pair[1] as u16) << 8) | pair[0] as u16))
            .collect();
        let _editor = data[note_end];
        let speed = data[note_end + 1];
        let start = data[note_end + 2];
        let end = data[note_end + 3];
        // eprintln!("start {_start} end {_end}");
        let loop_maybe = if end == 0 {
            if start > 0 {
                // Treat start as a length limiter.
                notes.truncate(start as usize);
            }
            None
        } else if end < start {
            // Start from a certain note.
            notes.drain(0..start as usize);
            None
        } else if end > start {
            // Now we have a loop.
            Some(Loop::Unstoppable {
                start: Some(start),
                end: Some(end),
            })
        } else {
            // start == end, no loop
            None
        };
        Self {
            notes,
            speed,
            loop_maybe,
        }
    }

    /// Sample-accurate 22050 Hz mono decoder (Pico-8 tracker).
    ///
    /// Playback starts at oscillator phase 0. Pico-8 `EXPORT %d.wav` does not;
    /// use [`decode_with_phase`] / [`phase_after_export`] for those goldens.
    pub fn decode(&self) -> SfxDecoder {
        SfxDecoder::new(self.clone())
    }

    /// Same as [`decode`], with a starting oscillator phase in `[0, 1)`.
    pub fn decode_with_phase(&self, phase: f32) -> SfxDecoder {
        SfxDecoder::with_phase(self.clone(), phase)
    }

    /// Oscillator phase after a 32-note WAV export starting at `phase`.
    ///
    /// Pico-8 writes 32 notes even when the SFX is shorter. Empty tail notes
    /// do not re-key the oscillator: it keeps the last sounding frequency
    /// (volume 0). Measured on 64 identical triangle-scale exports: Δφ ≈ 0.125
    /// per slot, repeating every 8.
    pub fn phase_after_export(&self, phase: f32) -> f32 {
        const EXPORT_NOTES: usize = 32;
        let mut sfx = self.clone();
        if let Some(last) = sfx.notes.iter().rev().copied().find(|n| n.volume() > 0.0) {
            let held = Pico8Note(last.key() as u16);
            sfx.notes.truncate(EXPORT_NOTES);
            sfx.notes.resize(EXPORT_NOTES, held);
        }
        let n = EXPORT_NOTES * sfx.speed.max(1) as usize * SAMPLES_PER_TICK as usize;
        let mut decoder = SfxDecoder::with_phase(sfx, phase);
        for _ in 0..n {
            if decoder.next().is_none() {
                break;
            }
        }
        decoder.phase()
    }

    pub fn with_speed(mut self, speed: u8) -> Self {
        self.speed = speed;
        self
    }

    pub fn with_loop(mut self, loop_start: Option<u8>, loop_end: Option<u8>) -> Self {
        if loop_start.is_some() || loop_end.is_some() {
            self.loop_maybe = Some(Loop::Unstoppable {
                start: loop_start,
                end: loop_end,
            });
        }
        self
    }

    pub fn get_stoppable_handle(
        handle: Handle<Sfx>,
        world: &mut World,
    ) -> (Handle<Sfx>, Option<Arc<AtomicBool>>) {
        let mut sfxs = world.resource_mut::<Assets<Sfx>>();
        let mut new_sfx = None;
        let mut new_release = None;
        if let Some(sfx) = sfxs.get(&handle)
            && let Some(ref loop_maybe) = sfx.loop_maybe
        {
            match loop_maybe {
                &Loop::Unstoppable { start, end } => {
                    let mut sfx_stoppable = sfx.clone();
                    let release = Arc::new(AtomicBool::new(false));
                    new_release = Some(release.clone());
                    sfx_stoppable.loop_maybe = Some(Loop::Stoppable {
                        start,
                        end,
                        release,
                    });
                    new_sfx = Some(sfx_stoppable);
                }
                Loop::Stoppable { release, .. } => {
                    release.store(false, Ordering::Relaxed);
                    new_release = Some(release.clone());
                }
            }
        }
        if let Some(new_sfx) = new_sfx {
            (sfxs.add(new_sfx), new_release)
        } else {
            (handle, new_release)
        }
    }
}

pub struct NoteIter {
    sfx: Sfx,
    index: usize,
}

impl Iterator for NoteIter {
    type Item = Pico8Note;
    fn next(&mut self) -> Option<Pico8Note> {
        if let Some(ref loop_maybe) = self.sfx.loop_maybe {
            let (start, end, released) = match loop_maybe {
                Loop::Unstoppable { .. } => {
                    panic!("Cannot stop a unstoppable sfx.");
                }
                Loop::Stoppable {
                    start,
                    end,
                    release,
                } => (
                    start.unwrap_or(0) as usize,
                    *end,
                    release.load(Ordering::Relaxed),
                ),
            };
            // Pico-8 wraps when the playhead reaches loop end. The p8 text
            // parser trims trailing rest notes, so `end` can sit at `notes.len()`
            // and must wrap rather than yield None.
            let past_end = end.is_some_and(|e| self.index >= e as usize);
            let past_notes = self.index >= self.sfx.notes.len();
            if past_end || past_notes {
                if released {
                    return None;
                }
                self.index = start;
            }
        }
        let result = self.sfx.notes.get(self.index).copied()?;
        self.index += 1;
        Some(result)
    }
}

impl From<Sfx> for NoteIter {
    fn from(sfx: Sfx) -> Self {
        // Pico-8 always starts at note 0; `loop_start` is only the wrap target.
        NoteIter { sfx, index: 0 }
    }
}

pub struct SfxDecoder {
    sfx: Sfx,
    notes: NoteIter,
    current: Option<Pico8Note>,
    step: usize,
    pos: u32,
    note_len: u32,
    phase: f32,
    phase_b: f32,
    prev_key: f32,
    prev_vol: f32,
    amp: f32,
    t: f32,
    noise: u32,
    noise_level: f32,
}

impl SfxDecoder {
    fn new(sfx: Sfx) -> Self {
        Self::with_phase(sfx, 0.0)
    }

    fn with_phase(sfx: Sfx, phase: f32) -> Self {
        let speed = sfx.speed.max(1);
        let note_len = speed as u32 * SAMPLES_PER_TICK;
        let mut notes = NoteIter::from(sfx.clone());
        let current = notes.next();
        let step = notes.index.saturating_sub(1);
        let prev_key = current.map(|n| n.key() as f32).unwrap_or(0.0);
        let prev_vol = current.map(|n| n.volume()).unwrap_or(0.0);
        let phase = phase.fract().rem_euclid(1.0);
        Self {
            sfx,
            notes,
            current,
            step,
            pos: 0,
            note_len,
            phase,
            phase_b: phase * 109.0 / 110.0,
            prev_key,
            prev_vol,
            amp: 0.0,
            t: 0.0,
            noise: 0x1234_5678,
            noise_level: 0.0,
        }
    }

    /// Current oscillator phase in `[0, 1)`.
    pub fn phase(&self) -> f32 {
        self.phase
    }
}

impl Iterator for SfxDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let note = self.current?;
        let note_len = self.note_len.max(1);
        let frac = self.pos as f32 / note_len as f32;
        let base_key = note.key() as f32;
        let mut key = base_key;
        let mut vol = note.volume();
        match note.effect() {
            Effect::None => {}
            Effect::Slide => {
                key = self.prev_key + (base_key - self.prev_key) * frac;
                vol = self.prev_vol + (vol - self.prev_vol) * frac;
            }
            Effect::Vibrato => {
                key += 0.25 * (self.t * 2.0 * std::f32::consts::PI * 8.0).sin();
            }
            Effect::Drop => {
                key = base_key * (1.0 - frac);
            }
            Effect::FadeIn => vol *= frac,
            Effect::FadeOut => vol *= 1.0 - frac,
            Effect::ArpFast | Effect::ArpSlow => {
                // Pico-8: fast ~32 Hz, slow ~16 Hz over groups of four notes.
                let rate = if matches!(note.effect(), Effect::ArpFast) {
                    32.0
                } else {
                    16.0
                };
                let idx = (self.t * rate) as usize % 4;
                let group = (self.step / 4) * 4;
                if let Some(n) = self.sfx.notes.get(group + idx) {
                    key = n.key() as f32;
                }
            }
        }

        let freq = key_to_freq(key.max(0.0));
        self.phase = (self.phase + freq * DT).fract();
        self.phase_b = (self.phase_b + freq * (109.0 / 110.0) * DT).fract();

        let raw = if matches!(note.wave(), WaveForm::Noise) {
            self.noise = self.noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let white = (self.noise >> 16) as f32 / 32768.0 - 1.0;
            let scale = freq * DT * NOISE_CUTOFF_SCALE;
            self.noise_level = (self.noise_level + scale * white) / (1.0 + scale);
            let factor = 1.0 - key / 63.0;
            self.noise_level * 1.5 * (1.0 + factor * factor)
        } else {
            tonal_wave(note.wave(), self.phase, self.phase_b)
        };

        let max_step = DT / ANTICLICK_RAMP;
        self.amp += (vol - self.amp).clamp(-max_step, max_step);
        let out = (raw * self.amp * OUTPUT_GAIN).clamp(-1.0, 1.0);

        self.t += DT;
        self.pos += 1;
        if self.pos >= note_len {
            self.prev_key = base_key;
            self.prev_vol = note.volume();
            self.current = self.notes.next();
            self.step = self.notes.index.saturating_sub(1);
            self.pos = 0;
        }
        Some(out)
    }
}

impl Source for SfxDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(1).unwrap()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(SAMPLE_RATE).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for Sfx {
    type Decoder = SfxDecoder;

    fn decoder(&self) -> Self::Decoder {
        SfxDecoder::new(self.clone())
    }
}

pub(crate) fn plugin(app: &mut App) {
    app //.register_type::<Sfx>()
        //.register_type::<Loop>()
        .init_resource::<AudioRecorder>()
        .add_plugins(command::plugin)
        .add_systems(PreStartup, add_channels)
        .add_audio_source::<Sfx>();
}

fn add_channels(mut commands: Commands) {
    let channels: Vec<Entity> = (0..4)
        .map(|i| {
            commands
                .spawn((Name::new(format!("channel {i}")), PlaybackSettings::REMOVE))
                .id()
        })
        .collect();
    commands.insert_resource(SfxChannels(channels));
}

// fn setup(mut assets: ResMut<Assets<Sfx>>, mut commands: Commands) {
//         // .take(duration)
//         // .chain(hz.clone().saw().take(duration))
//         // .chain(hz.clone().square().take(duration))
//         // .chain(hz.clone().noise_simplex().take(duration))
//         // .chain(signal::noise(0).take(duration))
//         // .map(|s| s.to_sample::<f32>() * 0.2)
//         ;
//     // add a `Sfx` to the asset server so that it can be played
//     let audio_handle = assets.add(Sfx::new([Pico8Note::default()])
//     // .with_speed(128)
//     );//Sfx::new(synth));//  {
//     //     // frequency: 440., // this is the frequency of A4
//     //     signal: Box::new(synth), // this is the frequency of A4
//     // });
//     commands.spawn(AudioPlayer(audio_handle));
// }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_flat_map() {
        let a = 0..3;
        let b = 3..6;
        let c = 6..9;
        let v = [a, b, c];
        let _ = v.iter().flat_map(|it| it.clone());
    }

    #[test]
    fn test_flat_map2() {
        let a = 0..3;
        let b = 3..6;
        let c = 6..9;
        let v = vec![a, b, c];
        let w = v.into_iter().flatten();
        assert_eq!((0..9).collect::<Vec<_>>(), w.collect::<Vec<_>>());
    }

    #[test]
    fn check_note_conversion() {
        let a = Pico8Note::default();
        let x = u16::from(a);
        let b = Pico8Note::from(x);
        assert_eq!(a, b);
    }
    #[test]
    fn sfx_parse0() {
        let s = "000800000f0000f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let sfx = Sfx::try_from(s).unwrap();
        let note = &sfx.notes[0];
        assert_eq!(note.pitch(), 50); // C1
        assert_eq!(note.wave(), WaveForm::Triangle);
        assert_eq!(note.effect(), Effect::None);
        assert_eq!(note.volume(), 0.0);

        let note = Pico8Note(0x000f);
        assert_eq!(note.pitch(), 50); // C1
        assert_eq!(note.wave(), WaveForm::Triangle);
        assert_eq!(note.effect(), Effect::None);
        assert_eq!(note.volume(), 0.0);
    }

    #[test]
    fn sfx_volume() {
        //       0 1 2 3 a    b    c    d    e    f    g    h
        let s = "001000000c0000c0100c0200c0300c0400c0500c0600c070000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let sfx = Sfx::try_from(s).unwrap();
        let note = &sfx.notes[1];
        assert_eq!(note.pitch(), 47); // C1
        assert_eq!(note.volume(), 1.0 / 7.0);
        assert_eq!(note.wave(), WaveForm::Triangle);
        assert_eq!(note.effect(), Effect::None);
        let volumes: Vec<u8> = sfx
            .notes
            .iter()
            .take(8)
            .map(|n| (n.volume() * 7.0) as u8)
            .collect();
        assert_eq!(volumes, vec![0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(sfx.notes.len(), 8);
    }

    #[test]
    fn sfx_wave() {
        use WaveForm::*;
        //       0 1 2 3 a    b    c    d    e    f    g    h
        let s = "001000000c050000000c150000000c250000000c350000000c450000000c550000000c650000000c7500000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let sfx = Sfx::try_from(s).unwrap();
        let volumes: Vec<WaveForm> = sfx.notes.iter().map(|n| n.wave()).collect();
        assert_eq!(
            volumes,
            vec![
                Triangle, Triangle, TiltedSaw, Triangle, Saw, Triangle, Square, Triangle, Pulse,
                Triangle, Organ, Triangle, Noise, Triangle,
                Phaser,
                // Triangle,
                //         Custom(0),
            ]
        );
        // Custom(u8)
        assert_eq!(sfx.notes.len(), 15);
    }

    #[test]
    fn sfx_pitch() {
        //       0 1 2 3 a    b    c    d    e    f    g    h
        let s = "001000000c050000000c150000000c250000000c350000000c450000000c550000000c650000000c7500000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let sfx = Sfx::try_from(s).unwrap();
        let volumes: Vec<u8> = sfx.notes.iter().take(15).map(|n| n.pitch()).collect();
        assert_eq!(
            volumes,
            vec![47, 35, 47, 35, 47, 35, 47, 35, 47, 35, 47, 35, 47, 35, 47,]
        );
        // Custom(u8)
        assert_eq!(sfx.notes.len(), 15);
    }

    #[test]
    fn note_wave() {
        let note = Pico8Note::new(37, WaveForm::Noise, 7, Effect::None);
        assert_eq!(note.wave(), WaveForm::Noise);
    }

    #[test]
    fn sfx_loop_wraps_from_zero_until_release() {
        let s = "000801080d0700f070110701207014070160701807019070000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let mut sfx = Sfx::try_from(s).unwrap();
        let release = Arc::new(AtomicBool::new(false));
        sfx.loop_maybe = Some(Loop::Stoppable {
            start: Some(1),
            end: Some(8),
            release: release.clone(),
        });
        let pitches: Vec<u8> = NoteIter::from(sfx.clone())
            .take(16)
            .map(|n| n.pitch())
            .collect();
        // Play 0..7, wrap to 1, then 1..7, wrap to 1.
        assert_eq!(
            pitches,
            vec![48, 50, 52, 53, 55, 57, 59, 60, 50, 52, 53, 55, 57, 59, 60, 50]
        );

        release.store(true, Ordering::Relaxed);
        let leftover: Vec<u8> = NoteIter::from(sfx).map(|n| n.pitch()).collect();
        assert_eq!(leftover, vec![48, 50, 52, 53, 55, 57, 59, 60]);
    }

    #[test]
    fn key_33_is_a4() {
        let note = Pico8Note::new(33 + PITCH_OFFSET, WaveForm::Triangle, 7, Effect::None);
        assert_eq!(note.key(), 33);
        let freq = key_to_freq(note.key() as f32);
        assert!((freq - 440.0).abs() < 0.01);
    }

    #[test]
    fn export_holds_last_key_through_silent_tail() {
        // Same triangle scale as tests/golden/synth/oscillator.p8.
        let sfx = Sfx::try_from(
            "000800000d0700f070110701207014070160701807019070000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        let delta = (sfx.phase_after_export(0.39) - 0.39).rem_euclid(1.0);
        // 64 identical Pico-8 exports advanced ~0.125 per slot.
        assert!(
            (delta - 0.125).abs() < 0.002,
            "Δφ={delta} (expected ~0.125)"
        );
    }
}
