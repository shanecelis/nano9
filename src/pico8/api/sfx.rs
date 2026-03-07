use super::*;
use bevy::audio::PlaybackMode;

use crate::pico8::audio::{AudioCommand, SfxDest};

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

bobtail::define! {
    #[doc(hidden)]
    pub __sfx => fn sfx(
        &mut self,
        n: impl Into<SfxCommand>,
        #[tail]
        channel: Option<u8>,
        offset: Option<u8>,
        length: Option<u8>,
        bank: Option<u8>,
    ) -> Result<(), Error>;
    #[doc(hidden)]
    pub __music => fn music(
        &mut self,
        n: impl Into<SfxCommand>,
        #[tail]
        _fade_ms: Option<u32>,
        channel_mask: Option<u8>,
        bank: Option<u8>,
    ) -> Result<(), Error>;
}
pub use __music as music;
pub use __sfx as sfx;

#[derive(Debug, Clone, Copy)]
pub enum SfxCommand {
    Play(u8),
    Release,
    Stop,
}

impl From<u8> for SfxCommand {
    fn from(x: u8) -> Self {
        SfxCommand::Play(x)
    }
}

impl super::Pico8<'_, '_> {
    pub fn bank(&self, bank: usize) -> Result<&AudioBank, Error> {
        let bank_handle = self
            .pico8_asset()?
            .audio_banks
            .get(bank)
            .ok_or(Error::NoAsset(format!("bank {bank}").into()))?;
        self.audio_banks
            .get(bank_handle)
            .ok_or(Error::NoAsset("audio bank handle".into()))
    }
    // sfx( n, [channel,] [offset,] [length] )
    pub fn sfx(
        &mut self,
        n: impl Into<SfxCommand>,
        channel: Option<u8>,
        offset: Option<u8>,
        length: Option<u8>,
        bank: Option<u8>,
    ) -> Result<(), Error> {
        assert!(offset.is_none(), "offset not implemented");
        assert!(length.is_none(), "length not implemented");
        let n = n.into();
        let bank = bank.unwrap_or(0);
        match n {
            SfxCommand::Release => {
                if let Some(chan) = channel {
                    // let chan = self.sfx_channels[chan as usize];
                    self.commands
                        .queue(AudioCommand::Release(SfxDest::Channel(chan)));
                } else {
                    self.commands.queue(AudioCommand::Release(SfxDest::Any));
                }
            }
            SfxCommand::Stop => {
                if let Some(chan) = channel {
                    // let chan = self.sfx_channels[chan as usize];
                    self.commands.queue(AudioCommand::Stop(
                        SfxDest::Channel(chan),
                        Some(PlaybackMode::Remove),
                    ));
                } else {
                    self.commands
                        .queue(AudioCommand::Stop(SfxDest::All, Some(PlaybackMode::Remove)));
                }
            }
            SfxCommand::Play(n) => {
                let sfx = self
                    .bank(bank as usize)?
                    .get(n as usize)
                    .ok_or(Error::NoAsset(format!("sfx {n}").into()))?
                    .clone();

                if let Some(chan) = channel {
                    // let chan = self.sfx_channels[chan as usize];
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::Channel(chan),
                        PlaybackSettings::REMOVE,
                    ));
                } else {
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::Any,
                        PlaybackSettings::REMOVE,
                    ));
                }
            }
        }
        Ok(())
    }

    // music( n, [facems,] [channelmask,] )
    pub fn music(
        &mut self,
        n: impl Into<SfxCommand>,
        _fade_ms: Option<u32>,
        channel_mask: Option<u8>,
        bank: Option<u8>,
    ) -> Result<(), Error> {
        let n = n.into();
        let bank = bank.unwrap_or(0);
        match n {
            SfxCommand::Release => {
                panic!("Music does not accept a release command.");
            }
            SfxCommand::Stop => {
                // if let Some(chan) = channel {
                //     let chan = self.sfx_channels[chan as usize];
                //     self.commands
                //         .queue(AudioCommand::Stop(SfxDest::Channel(chan), Some(PlaybackMode::Loop)));
                // } else {
                self.commands
                    .queue(AudioCommand::Stop(SfxDest::All, Some(PlaybackMode::Loop)));
                // }
            }
            SfxCommand::Play(n) => {
                let sfx = self
                    .bank(bank as usize)?
                    .get(n as usize)
                    .ok_or(Error::NoAsset(format!("music {n}").into()))?
                    .clone();

                if let Some(mask) = channel_mask {
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::ChannelMask(mask),
                        PlaybackSettings::LOOP,
                    ));
                } else {
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::Any,
                        PlaybackSettings::LOOP,
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::bindings::function::{
        namespace::{GlobalNamespace, NamespaceBuilder},
        script_function::FunctionCallContext,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register(
                "sfx",
                |ctx: FunctionCallContext,
                 // TODO: Need to be able to specify which audio bank.
                 n: Option<isize>,
                 channel: Option<u8>,
                 offset: Option<u8>,
                 length: Option<u8>,
                 bank: Option<u8>| {
                    let n = n.unwrap_or(0);
                    with_pico8(&ctx, move |pico8| {
                        pico8.sfx(
                            match n {
                                -2 => Ok(SfxCommand::Release),
                                -1 => Ok(SfxCommand::Stop),
                                n if n >= 0 => Ok(SfxCommand::Play(n as u8)), // TODO: Change to usize
                                x => Err(Error::InvalidArgument(
                                    format!("sfx: expected n to be -2, -1 or >= 0 but was {x}")
                                        .into(),
                                )),
                            }?,
                            channel,
                            offset,
                            length,
                            bank,
                        )
                    })
                },
            )
            .register(
                "music",
                |ctx: FunctionCallContext,
                 // TODO: Need to be able to specify which audio bank.
                 n: Option<isize>,
                 fade_ms: Option<u32>,
                 channel_mask: Option<u8>,
                 bank: Option<u8>| {
                    let n = n.unwrap_or(0);
                    with_pico8(&ctx, move |pico8| {
                        pico8.music(
                            match n {
                                -2 => Ok(SfxCommand::Release),
                                -1 => Ok(SfxCommand::Stop),
                                n if n >= 0 => Ok(SfxCommand::Play(n as u8)),
                                x => Err(Error::InvalidArgument(
                                    format!("sfx: expected n to be -2, -1 or >= 0 but was {x}")
                                        .into(),
                                )),
                            }?,
                            fade_ms,
                            channel_mask,
                            bank,
                        )
                    })
                },
            );
    }
}
