use bevy::{audio::PlaybackMode, prelude::*};

use bitvec::prelude::*;

use crate::pico8::audio::{Sfx, SfxChannels};

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) fn plugin(app: &mut App) {
    app//.register_type::<Audio>()
        // .register_type::<AudioBank>()
        .init_asset::<AudioBank>();
}

#[derive(Clone, Debug, Deref, DerefMut, Reflect, Asset)]
pub struct AudioBank(pub Vec<Audio>);

#[derive(Debug, Clone, Reflect)]
pub enum Audio {
    Sfx(Handle<Sfx>),
    AudioSource(Handle<AudioSource>),
}

pub enum SfxDest {
    Any,
    All,
    // Channel(Entity),
    Channel(u8),
    ChannelMask(u8),
}

pub enum AudioCommand {
    Stop(SfxDest, Option<PlaybackMode>),
    Play(Audio, SfxDest, PlaybackSettings),
    Release(SfxDest),
}

#[derive(Component)]
struct SfxRelease(Arc<AtomicBool>);

fn mode_eq(a: PlaybackMode, b: PlaybackMode) -> bool {
    std::mem::discriminant(&a) == std::mem::discriminant(&b)
}

#[cfg(feature = "mute")]
impl Command for AudioCommand {
    fn apply(self, world: &mut World) {}
}

#[cfg(not(feature = "mute"))]
impl Command for AudioCommand {
    fn apply(self, world: &mut World) {
        match self {
            AudioCommand::Stop(sfx_channel, mode) => {
                match sfx_channel {
                    SfxDest::All => {
                        // TODO: Consider using smallvec for channels.
                        let channels: Vec<Entity> = (*world.resource::<SfxChannels>()).clone();
                        for chan in channels {
                            if let Some(mode) = mode
                                && !world
                                    .get::<PlaybackSettings>(chan)
                                    .map(|s| mode_eq(s.mode, mode))
                                    .unwrap_or(true)
                            {
                                continue;
                            }
                            if let Some(sink) = world.get_mut::<AudioSink>(chan) {
                                sink.stop();
                            }
                            let mut commands = world.commands();

                            commands.entity(chan).remove::<(
                                // AudioPlayer<T>,
                                AudioSink,
                                PlaybackSettings,
                                // PlaybackRemoveMarker,
                            )>();
                        }
                    }
                    SfxDest::Channel(chan) => {
                        let id = world
                            .get_resource::<SfxChannels>()
                            .and_then(|sfx_channels| sfx_channels.get(chan as usize));
                        if let Some(id) = id {
                            if let Some(sink) = world.get_mut::<AudioSink>(*id) {
                                sink.stop();
                            }
                        } else {
                            warn!("Could not find audio channel {chan}");
                        }
                    }
                    SfxDest::ChannelMask(channel_mask) => {
                        for i in channel_mask.view_bits::<Lsb0>().iter_ones() {
                            let id = world
                                .get_resource::<SfxChannels>()
                                .and_then(|sfx_channels| sfx_channels.get(i))
                                .copied();
                            if let Some(id) = id {
                                if let Some(sink) = world.get_mut::<AudioSink>(id) {
                                    sink.stop();
                                }
                                let mut commands = world.commands();
                                commands.entity(id).remove::<(
                                    // AudioPlayer<T>,
                                    AudioSink,
                                    PlaybackSettings,
                                    // PlaybackRemoveMarker,
                                )>();
                            } else {
                                warn!("Could not find audio channel {i}");
                            }
                        }
                    }
                    SfxDest::Any => {
                        warn!("Cannot stop 'any' channels.");
                    }
                }
            }
            AudioCommand::Release(sfx_channel) => match sfx_channel {
                SfxDest::Channel(channel) => {
                    let id = world
                        .get_resource::<SfxChannels>()
                        .and_then(|sfx_channels| sfx_channels.get(channel as usize));
                    if let Some(id) = id {
                        if let Some(sfx_release) = world.get::<SfxRelease>(*id) {
                            sfx_release.0.store(true, Ordering::Relaxed);
                        } else {
                            warn!("Released a channel that did not have a sfx loop.");
                        }
                    } else {
                        warn!("Could not find audio channel {channel}");
                    }
                }
                SfxDest::Any => {}
                SfxDest::All => {}
                SfxDest::ChannelMask(_) => {}
            },
            AudioCommand::Play(audio, sfx_channel, playback_settings) => {
                match sfx_channel {
                    SfxDest::Any => {
                        if let Some(available_channel) = world
                            .resource::<SfxChannels>()
                            .iter()
                            .find(|id| {
                                world
                                    .get::<AudioSink>(**id)
                                    .map(|s| s.is_paused() || s.empty())
                                    .unwrap_or(true)
                            })
                            .copied()
                        {
                            match audio {
                                Audio::Sfx(sfx) => {
                                    let (sfx, release) = Sfx::get_stoppable_handle(sfx, world);
                                    let mut commands = world.commands();
                                    if let Some(release) = release {
                                        commands
                                            .entity(available_channel)
                                            .insert(SfxRelease(release));
                                    }
                                    commands
                                        .entity(available_channel)
                                        .insert((AudioPlayer(sfx), playback_settings));
                                }
                                Audio::AudioSource(source) => {
                                    let mut commands = world.commands();
                                    commands
                                        .entity(available_channel)
                                        .insert((AudioPlayer(source), playback_settings));
                                }
                            }
                        } else {
                            // The channels may be busy. If we log it, it can be
                            // noisy in the log despite it not having much of an
                            // effect to the game, so we're not going to log it.

                            warn_once!("Channels busy.");
                        }
                    }

                    SfxDest::ChannelMask(mask) => {
                        let mask_bits = mask.view_bits::<Lsb0>();
                        if let Some(available_channel) = world
                            .resource::<SfxChannels>()
                            .iter()
                            .enumerate()
                            .find_map(|(i, id)| {
                                (*mask_bits.get(i).as_deref().unwrap_or(&false)
                                    && world
                                        .get::<AudioSink>(*id)
                                        .map(|s| s.is_paused() || s.empty())
                                        .unwrap_or(true))
                                .then_some(id)
                            })
                            .copied()
                        {
                            match audio {
                                Audio::Sfx(sfx) => {
                                    let (sfx, release) = Sfx::get_stoppable_handle(sfx, world);
                                    let mut commands = world.commands();
                                    if let Some(release) = release {
                                        commands
                                            .entity(available_channel)
                                            .insert(SfxRelease(release));
                                    }
                                    commands
                                        .entity(available_channel)
                                        .insert((AudioPlayer(sfx), playback_settings));
                                }
                                Audio::AudioSource(source) => {
                                    let mut commands = world.commands();
                                    commands
                                        .entity(available_channel)
                                        .insert((AudioPlayer(source), playback_settings));
                                }
                            }
                        } else {
                            // The channels may be busy. If we log it, it can be
                            // noisy in the log despite it not having much of an
                            // effect to the game, so we're not going to log it.

                            warn_once!("Channels busy for mask {mask}.");
                        }
                    }
                    SfxDest::Channel(chan) => {
                        let id = world
                            .get_resource::<SfxChannels>()
                            .and_then(|sfx_channels| sfx_channels.get(chan as usize))
                            .copied();
                        let mut commands = world.commands();
                        if let Some(id) = id {
                            match audio {
                                Audio::Sfx(sfx) => {
                                    commands
                                        .entity(id)
                                        .insert((AudioPlayer(sfx.clone()), playback_settings));
                                }
                                Audio::AudioSource(source) => {
                                    commands
                                        .entity(id)
                                        .insert((AudioPlayer(source), playback_settings));
                                }
                            }
                        } else {
                            warn!("Could not find audio channel {chan}");
                        }
                    }
                    SfxDest::All => {
                        warn!("Cannot play on all channels.");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_matches_eq() {
        let x = PlaybackMode::Once;
        let y = PlaybackMode::Loop;
        // The y in the match expression below is not the y above this line.
        assert!(matches!(x, _y));
        assert!(!mode_eq(x, y));
    }
}
