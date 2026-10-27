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
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            base_cost: 0.1,
            food_decay_rate: 0.05,
            gene_pick_probability: UnitInterval::try_new(0.3).unwrap(),
            gene_mutation_factor: UnitInterval::try_new(0.1).unwrap(),
            reproduction_tick: NonZeroU32::new(5000).unwrap(),
        }
    }
}
