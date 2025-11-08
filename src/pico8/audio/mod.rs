//! Sfx audio

use crate::pico8::cart::{to_byte, to_nybble};
use bevy::{
    audio::{AddAudioSource, Source},
    prelude::*,
};
use dasp::{
    signal::{self, noise, Noise, Phase, Step},
    Signal,
};
use std::time::Duration;
use std::{
    borrow::Cow,
    f32,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

mod command;
pub use command::*;

const SAMPLE_RATE: u32 = 22_050;

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

pub struct Triangle<S> {
    phase: Phase<S>,
}

impl<S> Signal for Triangle<S>
where
    S: Step,
{
    type Frame = f64;

    /// Make a triangle wave that starts and ends at zero.
    #[inline]
    fn next(&mut self) -> Self::Frame {
        let phase = self.phase.next_phase();
        let a = 4.0 * phase;
        if phase < 0.25 {
            a
        } else if phase < 0.75 {
            -a + 2.0
        } else {
            a - 4.0
        }
    }
}

const DEFAULT_KNEE: f64 = 0.9;

pub struct TiltedSaw<S> {
    /// Where the saw turns downward. If `knee` is 1, it degrades into [Saw].
    knee: f64,
    phase: Phase<S>,
}

impl<S> Signal for TiltedSaw<S>
where
    S: Step,
{
    type Frame = f64;

    /// Make a triangle wave that starts and ends at zero.
    #[inline]
    fn next(&mut self) -> Self::Frame {
        let phase = self.phase.next_phase();

        if phase < self.knee {
            2.0 * phase / self.knee - 1.0
        } else {
            (-2.0 * phase + self.knee + 1.0) / (1.0 - self.knee)
        }
    }
}

pub struct Saw<S> {
    phase: Phase<S>,
}

impl<S> Signal for Saw<S>
where
    S: Step,
{
    type Frame = f64;

    /// Make a triangle wave that starts and ends at zero.
    #[inline]
    fn next(&mut self) -> Self::Frame {
        let phase = self.phase.next_phase();
        phase * 2.0 - 1.0
    }
}

const MINOR_HEIGHT: f64 = 1.1;

pub struct Organ<S> {
    phase: Phase<S>,
    minor_height: f64,
}

impl<S> Signal for Organ<S>
where
    S: Step,
{
    type Frame = f64;

    /// Make a major and minor triangle wave.
    #[inline]
    fn next(&mut self) -> Self::Frame {
        let phase = self.phase.next_phase();
        if phase < 0.25 {
            8.0 * phase - 1.0
        } else if phase < 0.5 {
            3.0 - 8.0 * phase
        } else if phase < 0.75 {
            self.minor_height * (4.0 * phase - 2.0) - 1.0
        } else {
            self.minor_height * (4.0 - 4.0 * phase) - 1.0
        }
    }
}

const PULSE_WIDTH: f64 = 0.375;

pub struct Pulse<S> {
    phase: Phase<S>,
    width: f64,
}

impl<S> Signal for Pulse<S>
where
    S: Step,
{
    type Frame = f64;

    #[inline]
    fn next(&mut self) -> Self::Frame {
        let phase = self.phase.next_phase();
        if phase < self.width {
            1.0
        } else {
            -1.0
        }
    }
}

const DRUNK_PACE: f64 = 0.1;

pub struct DrunkNoise {
    noise: Noise,
    pace: f64,
    current: f64,
}

impl Signal for DrunkNoise {
    type Frame = f64;

    #[inline]
    fn next(&mut self) -> Self::Frame {
        let step = self.pace * self.noise.next();
        self.current += step;
        if self.current > 1.0 {
            self.current -= 2.0 * step.abs();
        } else if self.current < -1.0 {
            self.current += 2.0 * step.abs();
        }
        self.current
    }
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
        (self.0 & 0b0011_1111) as u8 + PITCH_OFFSET
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
        if let Some(sfx) = sfxs.get(&handle) {
            if let Some(ref loop_maybe) = sfx.loop_maybe {
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
        let result = self.sfx.notes.get(self.index).copied();
        if let Some(ref loop_maybe) = self.sfx.loop_maybe {
            match loop_maybe {
                Loop::Unstoppable { .. } => {
                    panic!("Cannot stop a unstoppable sfx.");
                }
                Loop::Stoppable {
                    start,
                    end,
                    release,
                } => 'block: {
                    if let Some(end) = end {
                        if *end as usize == self.index && !release.load(Ordering::Relaxed) {
                            self.index = start.unwrap_or(0) as usize;
                            break 'block;
                        }
                    }
                    self.index += 1;
                }
            }
        } else {
            self.index += 1;
        }
        result
    }
}

impl From<Sfx> for NoteIter {
    fn from(sfx: Sfx) -> Self {
        NoteIter {
            index: sfx
                .loop_maybe
                .as_ref()
                .and_then(|l| match *l {
                    Loop::Unstoppable { start, .. } | Loop::Stoppable { start, .. } => start,
                })
                .unwrap_or(0) as usize,
            sfx,
        }
    }
}

pub struct SfxDecoder {
    sfx_notes: NoteIter,
    samples: Option<Box<dyn Iterator<Item = f32> + Sync + Send + 'static>>,
}

impl Iterator for SfxDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result = None;
        if let Some(samples) = &mut self.samples {
            result = samples.next();
            if result.is_none() {
                self.samples = None; // Will create one for the next note.
            }
        }
        if self.samples.is_none() {
            self.samples = self.sfx_notes.next().map(|note| {
                // midi pitch to frequency equation.
                // https://www.music.mcgill.ca/~gary/307/week1/node28.html
                let freq = 440.0 * f32::exp2((note.pitch() as i8 - 69) as f32 / 12.0);
                // dbg!(note.pitch(), freq);
                let hz = signal::rate(SAMPLE_RATE as f64).const_hz(freq as f64);
                let duration = (self.sfx_notes.sfx.speed as f32 / 120.0) * SAMPLE_RATE as f32;
                let volume: f32 = note.volume();
                match note.wave() {
                    WaveForm::Triangle => {
                        let synth = Triangle { phase: hz.phase() }
                            .map(|x| x as f32)
                            .scale_amp(volume);
                        Box::new(synth.take(duration as usize))
                            as Box<dyn Iterator<Item = f32> + Sync + Send + 'static>
                    }
                    WaveForm::TiltedSaw => {
                        let synth = TiltedSaw {
                            phase: hz.phase(),
                            knee: DEFAULT_KNEE,
                        }
                        .map(|x| x as f32)
                        .scale_amp(volume);
                        Box::new(synth.take(duration as usize))
                            as Box<dyn Iterator<Item = f32> + Sync + Send + 'static>
                    }
                    WaveForm::Saw => {
                        let synth = Saw { phase: hz.phase() }
                            .map(|x| x as f32)
                            .scale_amp(volume);
                        Box::new(synth.take(duration as usize))
                            as Box<dyn Iterator<Item = f32> + Sync + Send + 'static>
                    }
                    WaveForm::Square => {
                        // let synth = TiltedSaw { phase: hz.phase(),
                        //                         knee: DEFAULT_KNEE }
                        let synth = hz.square().map(|x| x as f32).scale_amp(volume);
                        Box::new(synth.take(duration as usize))
                            as Box<dyn Iterator<Item = f32> + Sync + Send + 'static>
                    }
                    WaveForm::Pulse => {
                        let synth = Pulse {
                            phase: hz.phase(),
                            width: PULSE_WIDTH,
                        }
                        .map(|x| x as f32)
                        .scale_amp(volume);
                        Box::new(synth.take(duration as usize))
                            as Box<dyn Iterator<Item = f32> + Sync + Send + 'static>
                    }
                    WaveForm::Organ => {
                        let synth = Organ {
                            phase: hz.phase(),
                            minor_height: MINOR_HEIGHT,
                        }
                        .map(|x| x as f32)
                        .scale_amp(volume);
                        Box::new(synth.take(duration as usize))
                            as Box<dyn Iterator<Item = f32> + Sync + Send + 'static>
                    }
                    WaveForm::Noise => {
                        let synth = DrunkNoise {
                            noise: noise(0),
                            pace: DRUNK_PACE,
                            current: 0.0,
                        }
                        .map(|x| x as f32)
                        .scale_amp(volume);
                        Box::new(synth.take(duration as usize))
                            as Box<dyn Iterator<Item = f32> + Sync + Send + 'static>
                    }
                    WaveForm::Phaser => {
                        let synth = hz.sine().map(|x| x as f32).scale_amp(volume);
                        Box::new(synth.take(duration as usize))
                            as Box<dyn Iterator<Item = f32> + Sync + Send + 'static>
                    }
                    x => todo!("WaveForm {x:?} not supported yet"),
                }
            });
        }
        result.or_else(|| self.samples.as_mut().and_then(|samples| samples.next()))
    }
}

impl Source for SfxDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for Sfx {
    type DecoderItem = <SfxDecoder as Iterator>::Item;

    type Decoder = SfxDecoder;

    fn decoder(&self) -> Self::Decoder {
        SfxDecoder {
            sfx_notes: self.clone().into(),
            samples: None,
        }
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<Sfx>()
        .register_type::<Loop>()
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
}
