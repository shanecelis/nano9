//! Pattern playback for Pico-8 `music()`.
//!
//! A channel byte below 64 is an SFX index. `0x40`–`0x43` are silent channels.
//! The pattern clock waits out each SFX slot, including trailing rests the
//! decoder does not play, then steps to the next pattern. A slot is
//! `span * speed` ticks of 183 samples, so 32 notes at speed 8 last 2.124s.

use bevy::audio::PlaybackSettings;
use bevy::diagnostic::FrameCount;
use bevy::prelude::*;

use super::Sfx;
use super::command::{Audio, AudioBank, AudioCommand, SfxDest};
use crate::pico8::MusicPattern;

#[derive(Resource)]
pub(crate) struct MusicPlayback {
    patterns: Vec<MusicPattern>,
    bank: Handle<AudioBank>,
    index: usize,
    /// Bits set protect that channel from automatic `sfx()`. Pattern playback
    /// still uses every occupied channel; `sfx()` does not consult this yet.
    #[allow(dead_code)]
    mask: u8,
    /// Bits set are channels this playback started.
    active: u8,
    /// Frame when this `music()` call started.
    origin_frame: u32,
    /// Ideal sample position of the current pattern, from `origin_frame`.
    elapsed_samples: u64,
    /// Sample length of the pattern now playing. Zero when nothing is cued.
    pattern_samples: u64,
    next_frame: u32,
    playing: bool,
}

struct Cue {
    channel: u8,
    audio: Audio,
    /// `None` when the SFX loops and holds the pattern.
    samples: Option<u64>,
}

pub(crate) fn start_music(
    world: &mut World,
    patterns: Vec<MusicPattern>,
    bank: Handle<AudioBank>,
    index: usize,
    mask: u8,
) {
    stop_music(world);
    if index >= patterns.len() {
        return;
    }
    let frame = world
        .get_resource::<FrameCount>()
        .map(|frame| frame.0)
        .unwrap_or(0);
    world.insert_resource(MusicPlayback {
        patterns,
        bank,
        index,
        mask: mask & 0x0f,
        active: 0,
        origin_frame: frame,
        elapsed_samples: 0,
        pattern_samples: 0,
        next_frame: frame,
        playing: true,
    });
    play_current(world, frame);
}

pub(crate) fn stop_music(world: &mut World) {
    let active = {
        let Some(mut playback) = world.get_resource_mut::<MusicPlayback>() else {
            return;
        };
        let active = playback.active;
        playback.playing = false;
        playback.active = 0;
        playback.next_frame = u32::MAX;
        active
    };
    stop_channels(world, active);
}

pub(crate) fn tick(
    frame: Res<FrameCount>,
    playback: Option<Res<MusicPlayback>>,
    mut commands: Commands,
) {
    let Some(playback) = playback else {
        return;
    };
    if !playback.playing || frame.0 < playback.next_frame {
        return;
    }
    let frame = frame.0;
    commands.queue(move |world: &mut World| advance(world, frame));
}

fn advance(world: &mut World, frame: u32) {
    let (stop, next, len) = {
        let Some(playback) = world.get_resource::<MusicPlayback>() else {
            return;
        };
        if !playback.playing || frame < playback.next_frame {
            return;
        }
        let index = playback.index;
        let len = playback.patterns.len();
        match playback.patterns.get(index) {
            None => (true, len, len),
            Some(pattern) => {
                let next = if pattern.end {
                    loop_target(&playback.patterns, index)
                } else {
                    index + 1
                };
                (pattern.stop, next, len)
            }
        }
    };
    if stop || next >= len {
        stop_music(world);
        return;
    }
    if let Some(mut playback) = world.get_resource_mut::<MusicPlayback>() {
        playback.elapsed_samples = playback
            .elapsed_samples
            .saturating_add(playback.pattern_samples);
        playback.index = next;
    }
    play_current(world, frame);
}

fn play_current(world: &mut World, frame: u32) {
    let Some(cues) = cues(world) else {
        return;
    };
    let mut active = 0u8;
    let mut duration = 0u64;
    let mut any = false;
    let mut holds = false;
    for cue in cues {
        match cue.samples {
            None => holds = true,
            Some(samples) => {
                any = true;
                duration = duration.max(samples);
            }
        }
        AudioCommand::Play(
            cue.audio,
            SfxDest::Channel(cue.channel),
            PlaybackSettings::REMOVE,
        )
        .apply(world);
        active |= 1 << cue.channel;
    }
    let (origin, elapsed) = world
        .get_resource::<MusicPlayback>()
        .map(|playback| (playback.origin_frame, playback.elapsed_samples))
        .unwrap_or((frame, 0));
    let next_frame = if holds {
        u32::MAX
    } else if any {
        frame_at_sample(origin, elapsed.saturating_add(duration.max(1)))
    } else {
        frame.saturating_add(1)
    };
    if let Some(mut playback) = world.get_resource_mut::<MusicPlayback>() {
        playback.active = active;
        playback.pattern_samples = if any { duration } else { 0 };
        playback.next_frame = next_frame;
    }
}

fn cues(world: &World) -> Option<Vec<Cue>> {
    let playback = world.get_resource::<MusicPlayback>()?;
    let pattern = playback.patterns.get(playback.index)?.clone();
    let bank = playback.bank.clone();
    let banks = world.resource::<Assets<AudioBank>>();
    let sfxs = world.resource::<Assets<Sfx>>();
    let bank = banks.get(&bank)?;
    let mut cues = Vec::new();
    for (channel, sfx_id) in pattern.channels.into_iter().enumerate() {
        let channel = channel as u8;
        let Some(sfx_id) = sfx_id else {
            continue;
        };
        let Some(audio) = bank.get(sfx_id as usize).cloned() else {
            warn!("music: no sfx {sfx_id} on channel {channel}");
            continue;
        };
        let frames = match &audio {
            Audio::Sfx(handle) => match sfxs.get(handle) {
                Some(sfx) => pattern_frames(sfx),
                None => {
                    warn!("music: sfx {sfx_id} is not loaded");
                    continue;
                }
            },
            Audio::AudioSource(_) => Some(1),
        };
        cues.push(Cue {
            channel,
            audio,
            samples: frames,
        });
    }
    Some(cues)
}

/// `None` holds the pattern because the SFX loops.
fn pattern_frames(sfx: &Sfx) -> Option<u64> {
    if sfx.loop_maybe.is_some() {
        return None;
    }
    let speed = u64::from(sfx.speed.max(1));
    let span = u64::from(sfx.span.max(1));
    Some(span * speed * u64::from(super::SAMPLES_PER_TICK))
}

/// First frame whose recorded sample position reaches `sample`.
fn frame_at_sample(origin: u32, sample: u64) -> u32 {
    if sample == 0 {
        return origin;
    }
    let mut delta = sample * 60 / u64::from(super::SAMPLE_RATE);
    loop {
        let at = ((delta as f64) * f64::from(super::SAMPLE_RATE) / 60.0).round() as u64;
        if at >= sample {
            return origin.saturating_add(delta as u32);
        }
        delta += 1;
    }
}

fn loop_target(patterns: &[MusicPattern], index: usize) -> usize {
    (0..=index).rev().find(|i| patterns[*i].begin).unwrap_or(0)
}

fn stop_channels(world: &mut World, active: u8) {
    for channel in 0..4u8 {
        if active & (1 << channel) == 0 {
            continue;
        }
        AudioCommand::Stop(
            SfxDest::Channel(channel),
            Some(bevy::audio::PlaybackMode::Remove),
        )
        .apply(world);
    }
}
