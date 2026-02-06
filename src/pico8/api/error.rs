use bevy::image::TextureAccessError;
use std::borrow::Cow;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("no such {0:?}")]
    NoSuch(Cow<'static, str>),
    #[error("{0}")]
    PalError(#[from] crate::pico8::PalError),
    #[error("no asset {0:?} loaded")]
    NoAsset(Cow<'static, str>),
    // #[error("invalid {0:?}")]
    // Invalid(Cow<'static, str>),
    #[error("texture access error: {0}")]
    TextureAccess(#[from] TextureAccessError),
    #[error("no such button: {0}")]
    NoSuchButton(u8),
    #[error("invalid argument {0}")]
    InvalidArgument(Cow<'static, str>),
    #[error("unsupported {0}")]
    Unsupported(Cow<'static, str>),
    #[error("no sfx channel {0}")]
    NoChannel(u8),
    #[error("all sfx channels are busy")]
    ChannelsBusy,
    #[error("unsupported poke at address {0}")]
    UnsupportedPoke(usize),
    #[error("unsupported peek at address {0}")]
    UnsupportedPeek(usize),
    #[error("unsupported stat at address {0}")]
    UnsupportedStat(u8),
    #[error("{0}")]
    Message(Cow<'static, str>),
}
