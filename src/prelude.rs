pub use super::{
    Nano9Plugin, Nano9Plugins, PColor,
    config::{self, Config, ConfigError, pause_pico8_when_loaded, run_pico8_when_loaded},
    pico8::Pico8,
    run::RunState,
};

// macros
// pub use crate::{cls, camera, spr, print};
// pub use crate::cls;
pub use crate::pico8::{camera, canvas::cls, print::print, spr};

// pub use super::spr2 as spr;
// pub use spr2;
// Bobtail macros
// pub mod macros {
//     pub use crate::{
//     spr,
//     camera,
//     print,
//     };
// }
pub use std::str::FromStr;
