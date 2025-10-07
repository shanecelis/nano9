use crate::run::{RunState, Steps};
use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowMode},
};

pub fn toggle_pause(state: Res<State<RunState>>, mut next_state: ResMut<NextState<RunState>>) {
    next_state.set(match **state {
        RunState::Run => RunState::Pause,
        RunState::Pause => RunState::Run,
        _ => RunState::Pause,
    });
}

pub fn one_step(
    state: Res<State<RunState>>,
    mut next_state: ResMut<NextState<RunState>>,
    mut steps: ResMut<Steps>,
) {
    **steps = Some(1);
    match **state {
        RunState::Run => {}
        RunState::Pause => {
            next_state.set(RunState::Run);
        }
        s => {
            warn!("Cannot do one_step in state {:?}.", s);
        }
    }
}

pub fn toggle_fullscreen(mut primary_windows: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut primary_window) = primary_windows.single_mut() else {
        warn!("Unable to find primary window to toggle full screen.");
        return;
    };
    primary_window.mode = match primary_window.mode {
        WindowMode::Windowed => {
            WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
        }
        _ => WindowMode::Windowed,
    }
}
