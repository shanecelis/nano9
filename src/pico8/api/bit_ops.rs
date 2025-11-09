use super::*;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

#[cfg(feature = "fixed-point")]
mod fixed_point {
    use fixed::FixedI32;
    use fixed::types::extra::U16;
    impl super::Pico8<'_, '_> {
        pub fn shl(a: f32, b: u8) -> f32 {
            let a = FixedI32::<U16>::from_num(a);
            let c = a << b;
            c.to_num()
        }

        pub fn shr(a: f32, b: u8) -> f32 {
            let a = FixedI32::<U16>::from_num(a);
            let c = a >> b;
            c.to_num()
        }

        pub fn lshr(a: f32, b: u8) -> f32 {
            let c: u32 = FixedI32::<U16>::from_num(a).to_bits() as u32;
            let d = c >> b;
            FixedI32::<U16>::from_bits(d as i32).to_num()
        }

        pub fn rotr(a: f32, b: u8) -> f32 {
            let c: u32 = FixedI32::<U16>::from_num(a).to_bits() as u32;
            let d = (c << (32 - b)) | (c >> b);
            FixedI32::<U16>::from_bits(d as i32).to_num()
        }

        pub fn rotl(a: f32, b: u8) -> f32 {
            let c: u32 = FixedI32::<U16>::from_num(a).to_bits() as u32;
            let d = (c << b) | (c >> (32 - b));
            FixedI32::<U16>::from_bits(d as i32).to_num()
        }
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;

    use bevy_mod_scripting::bindings::function::{
        namespace::{GlobalNamespace, NamespaceBuilder},
        script_function::FunctionCallContext,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register("shl", |_ctx: FunctionCallContext, a: f32, b: u8| {
                Pico8::shl(a, b)
            })
            .register("shr", |_ctx: FunctionCallContext, a: f32, b: u8| {
                Pico8::shr(a, b)
            })
            .register("lshr", |_ctx: FunctionCallContext, a: f32, b: u8| {
                Pico8::lshr(a, b)
            })
            .register("rotl", |_ctx: FunctionCallContext, a: f32, b: u8| {
                Pico8::rotl(a, b)
            })
            .register("rotr", |_ctx: FunctionCallContext, a: f32, b: u8| {
                Pico8::rotr(a, b)
            });
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(feature = "fixed-point")]
    mod fixed {
        use super::*;
        #[test]
        fn test_shr() {
            assert_eq!(0.5, Pico8::shr(1.0, 1));
            assert_eq!(-0.5, Pico8::shr(-1.0, 1));
        }

        #[test]
        fn test_lshr() {
            assert_eq!(0.5, Pico8::lshr(1.0, 1));
            assert_eq!(32767.5, Pico8::lshr(-1.0, 1));
            assert_eq!(8191.875, Pico8::lshr(-1.0, 3));
        }

        #[test]
        fn test_shl() {
            assert_eq!(2.0, Pico8::shl(1.0, 1));
        }

        #[test]
        fn test_rotr() {
            assert_eq!(Pico8::rotr(64.0, 3), 8.0);
            assert_eq!(Pico8::rotr(1.0, 3), 0.125);
            assert_eq!(Pico8::rotr(-4096.0, 12), 15.0);
        }

        #[test]
        fn test_rotl() {
            assert_eq!(Pico8::rotl(8.0, 3), 64.0);
            assert_eq!(Pico8::rotl(0.125, 3), 1.0);
            assert_eq!(Pico8::rotl(-4096.0, 12), 0.05859375);
        }
    }
}
