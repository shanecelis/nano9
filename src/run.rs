use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States, Reflect)]
pub enum RunState {
    #[default]
    Uninit,
    Loaded,
    Init,
    Run,
    Pause,
    /// Error messages
    ///
    /// XXX: Change name to suit.
    Messages,
}

#[derive(Debug, Resource, Default, Deref, DerefMut)]
pub struct Steps(Option<u32>);

pub(crate) fn plugin(app: &mut App) {
    app.init_state::<RunState>()
        .init_resource::<Steps>()
        .add_systems(Last, pause_after_steps);
}

fn pause_after_steps(
    run_state: Res<State<RunState>>,
    mut next_run_state: ResMut<NextState<RunState>>,
    mut steps: ResMut<Steps>,
) {
    if **run_state == RunState::Run
        && let Some(count) = &mut steps.0
    {
        if *count <= 1 {
            steps.0 = None;
            next_run_state.set(RunState::Pause);
        } else {
            *count -= 1;
        }
    }
}
