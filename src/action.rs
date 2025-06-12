use bevy::prelude::*;
use crate::error::RunState;

pub fn toggle_pause(
    state: Res<State<RunState>>,
    mut next_state: ResMut<NextState<RunState>>,
) {
    next_state.set(match **state {
        RunState::Run => RunState::Pause,
        RunState::Pause => RunState::Run,
        _ => RunState::Pause,
    });
}
