use bevy::prelude::*;
use crate::run::{RunState, Steps};

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

pub fn one_step(
    state: Res<State<RunState>>,
    mut next_state: ResMut<NextState<RunState>>,
    mut steps: ResMut<Steps>) {
    **steps = Some(1);
    match **state {
        RunState::Run => {
        }
        RunState::Pause => {
            next_state.set(RunState::Run);
        }
        s => {
            warn!("Cannot do one_step in state {:?}.", s);
        }
    }
}
