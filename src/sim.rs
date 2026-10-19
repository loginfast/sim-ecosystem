use bevy::prelude::*;

#[derive(Resource)]
pub struct SimParams {
    pub energy_max: f32,
    pub base_cost: f32,
    pub movement_cost: f32,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            energy_max: 100.0,
            base_cost: 0.5,
            movement_cost: 0.1,
        }
    }
}
