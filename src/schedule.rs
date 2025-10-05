use bevy::{
    prelude::{in_state, App, World},
    app::MainScheduleOrder,
    ecs::schedule::{Schedule, ScheduleLabel},
    core::FrameCount,
};
use crate::run::RunState;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Init;
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Run;
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Draw;

pub(crate) fn plugin(app: &mut App) {
    app.add_schedule(Schedule::new(Init));
    app.add_schedule(Schedule::new(Run));
    app.add_schedule(Schedule::new(Draw));
    // app.configure_schedule(Update, |sched| {
    //     sched.set_run_if(in_state(RunState::Run));
    // });

    // app.configure_schedule(Draw, |sched| {
    //     sched.set_run_if(in_state(RunState::Run));
    // });
    // {
    //     let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
    //     order.insert_after(bevy::prelude::Update, Update);
    //     order.insert_after(Update, Draw);
    // }
}

pub(crate) fn run_schedule(label: impl ScheduleLabel + Clone) -> impl FnMut(&mut World) {
    move |world: &mut World| {
        if let Err(e) = world.try_run_schedule(label.clone()) {
            // bevy::log::error!("Cannot run schedule {:?} because {e}", &label);
        } else {
            let frame_count = world.resource::<FrameCount>();
            // bevy::log::info!("Ran schedule {:?} in frame {}", &label, &frame_count.0);
        }
    }
}
