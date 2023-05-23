use crate::GameState;
use bevy::prelude::*;

use self::{
    end::{detect_player, spawn_end_portal, spawn_arrow},
    spawn::spawn_mobs,
};

mod end;
mod spawn;

pub struct EventsHandlerPlugin;

impl Plugin for EventsHandlerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_end_portal.in_schedule(OnEnter(GameState::Game)))
            .add_systems((detect_player, spawn_mobs, spawn_arrow).in_set(OnUpdate(GameState::Game)));
    }
}

#[derive(Component)]
pub struct EndPortal {}

#[derive(Component)]
pub struct Arrow {
    timer: Timer,
}
