use bevy::{
    ecs::schedule::ScheduleLabel,
    prelude::{App, World},
};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Init;
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Update;
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Draw;

pub(crate) fn plugin(app: &mut App) {
    app.init_schedule(Init);
    app.init_schedule(Update);
    app.init_schedule(Draw);
    // app.configure_schedule(Update, |sched| {
    //     sched.set_run_if(in_state(RunState::Update));
    // });

    // app.configure_schedule(Draw, |sched| {
    //     sched.set_run_if(in_state(RunState::Update));
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
            bevy::log::error!("Cannot run schedule {:?} because {e}", &label);
        } else {
            let _frame_count = world.resource::<bevy::diagnostic::FrameCount>();
            // bevy::log::trace!("Ran schedule {:?} in frame {}", &label, &frame_count.0);
        }
    }
}
