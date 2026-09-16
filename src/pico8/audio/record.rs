//! Offline mixdown for `extcmd("audio_rec")` / `extcmd("audio_end")`.
//!
//! Play/stop/release events are logged against Bevy's frame counter, then
//! rendered through [`Sfx::decoder`] so a WAV can be written without tapping
//! the audio device.

use super::{Loop, SAMPLE_RATE, Sfx, SfxDecoder};
use bevy::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

const CHANNELS: usize = 4;
const FRAMES_PER_SECOND: f64 = 60.0;

#[derive(Resource, Default)]
pub struct AudioRecorder {
    active: Option<Recording>,
}

pub struct Recording {
    start_frame: u32,
    events: Vec<RecEvent>,
}

struct RecEvent {
    frame: u32,
    channel: u8,
    kind: RecKind,
}

enum RecKind {
    Play(Sfx),
    Stop,
    Release,
}

impl AudioRecorder {
    pub fn start(&mut self, frame: u32) {
        self.active = Some(Recording {
            start_frame: frame,
            events: Vec::new(),
        });
    }

    pub fn play(&mut self, frame: u32, channel: u8, sfx: Sfx) {
        self.push(frame, channel, RecKind::Play(sfx));
    }

    pub fn stop(&mut self, frame: u32, channel: u8) {
        self.push(frame, channel, RecKind::Stop);
    }

    pub fn release(&mut self, frame: u32, channel: u8) {
        self.push(frame, channel, RecKind::Release);
    }

    fn push(&mut self, frame: u32, channel: u8, kind: RecKind) {
        if let Some(active) = &mut self.active {
            active.events.push(RecEvent {
                frame,
                channel,
                kind,
            });
        }
    }

    pub fn take(&mut self) -> Option<Recording> {
        self.active.take()
    }
}

impl Recording {
    pub fn render(&self, end_frame: u32) -> Vec<i16> {
        let n_samples = frame_to_sample(self.start_frame, end_frame.max(self.start_frame));
        let mut mix = vec![0.0f32; n_samples];
        let mut by_channel: [Vec<&RecEvent>; CHANNELS] = Default::default();
        for event in &self.events {
            if let Some(slot) = by_channel.get_mut(event.channel as usize) {
                slot.push(event);
            }
        }
        for events in &by_channel {
            mix_channel(&mut mix, self.start_frame, events);
        }
        mix.into_iter()
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect()
    }
}

fn frame_to_sample(start_frame: u32, frame: u32) -> usize {
    let frames = frame.saturating_sub(start_frame) as f64;
    (frames * SAMPLE_RATE as f64 / FRAMES_PER_SECOND).round() as usize
}

fn mix_channel(mix: &mut [f32], start_frame: u32, events: &[&RecEvent]) {
    let n = mix.len();
    let mut decoder: Option<SfxDecoder> = None;
    let mut release: Option<Arc<AtomicBool>> = None;
    let mut cursor = 0usize;
    for event in events {
        let at = frame_to_sample(start_frame, event.frame).min(n);
        drain(mix, &mut decoder, cursor, at);
        cursor = at;
        match &event.kind {
            RecKind::Play(sfx) => {
                let (dec, rel) = decoder_for_record(sfx);
                decoder = Some(dec);
                release = Some(rel);
            }
            RecKind::Stop => {
                decoder = None;
                release = None;
            }
            RecKind::Release => {
                if let Some(rel) = &release {
                    rel.store(true, Ordering::Relaxed);
                }
            }
        }
    }
    drain(mix, &mut decoder, cursor, n);
}

fn drain(mix: &mut [f32], decoder: &mut Option<SfxDecoder>, from: usize, to: usize) {
    let Some(dec) = decoder.as_mut() else {
        return;
    };
    for sample in mix.iter_mut().take(to).skip(from) {
        match dec.next() {
            Some(s) => *sample += s,
            None => {
                *decoder = None;
                return;
            }
        }
    }
}

fn decoder_for_record(sfx: &Sfx) -> (SfxDecoder, Arc<AtomicBool>) {
    let release = Arc::new(AtomicBool::new(false));
    let mut sfx = sfx.clone();
    match sfx.loop_maybe {
        Some(Loop::Unstoppable { start, end }) => {
            sfx.loop_maybe = Some(Loop::Stoppable {
                start,
                end,
                release: release.clone(),
            });
        }
        Some(Loop::Stoppable { start, end, .. }) => {
            sfx.loop_maybe = Some(Loop::Stoppable {
                start,
                end,
                release: release.clone(),
            });
        }
        None => {}
    }
    (sfx.decoder(), release)
}

pub fn write_wav(path: &Path, pcm: &[i16]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = BufWriter::new(File::create(path).map_err(|e| e.to_string())?);
    let data_bytes = (pcm.len() * 2) as u32;
    let sample_rate = SAMPLE_RATE;
    file.write_all(b"RIFF").map_err(|e| e.to_string())?;
    file.write_all(&(36 + data_bytes).to_le_bytes())
        .map_err(|e| e.to_string())?;
    file.write_all(b"WAVEfmt ").map_err(|e| e.to_string())?;
    file.write_all(&16u32.to_le_bytes()).map_err(|e| e.to_string())?; // PCM header size
    file.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?; // audio format
    file.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?; // channels
    file.write_all(&sample_rate.to_le_bytes())
        .map_err(|e| e.to_string())?;
    file.write_all(&(sample_rate * 2).to_le_bytes())
        .map_err(|e| e.to_string())?; // byte rate
    file.write_all(&2u16.to_le_bytes()).map_err(|e| e.to_string())?; // block align
    file.write_all(&16u16.to_le_bytes()).map_err(|e| e.to_string())?; // bits
    file.write_all(b"data").map_err(|e| e.to_string())?;
    file.write_all(&data_bytes.to_le_bytes())
        .map_err(|e| e.to_string())?;
    for sample in pcm {
        file.write_all(&sample.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    file.flush().map_err(|e| e.to_string())?;
    Ok(())
}
