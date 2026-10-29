use std::num::NonZeroU32;

use bevy::prelude::*;

use crate::ranged::UnitInterval;

#[derive(Resource)]
pub struct SimParams {
    pub base_cost: f32,
    pub food_decay_rate: f32,
    pub gene_pick_probability: UnitInterval,
    pub gene_mutation_factor: UnitInterval,
    pub reproduction_tick: NonZeroU32,

    pub speed_max: f32,
    pub size_max: f32,
    pub vision_radius_max: u32,

    pub w_size: f32,
    pub w_move: f32,
    pub w_vision: f32,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            base_cost: 0.8,
            food_decay_rate: 0.05,
            gene_pick_probability: UnitInterval::try_new(0.3).unwrap(),
            gene_mutation_factor: UnitInterval::try_new(0.1).unwrap(),
            reproduction_tick: NonZeroU32::new(1000).unwrap(),

            speed_max: 10.0,
            size_max: 0.95,
            vision_radius_max: 42,

            w_size: 0.5,
            w_move: 0.35,
            w_vision: 0.15,
        }
    }
}
