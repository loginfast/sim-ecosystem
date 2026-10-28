use bevy::prelude::*;

use crate::creature::WorldPos;
use crate::sim::SimParams;
use crate::worldgen::WorldGrid;
use crate::world_to_grid;
use crate::TILE_SZ;
use crate::GRID_SIZE;
use rand::RngExt;

#[derive(Clone, Copy)]
pub enum FoodKind {
    Plant,
    Carcass,
}

impl FoodKind {
    pub fn color(&self) -> Color {
        match self {
            Self::Plant => Color::srgb(0.1, 0.5, 0.1),
            Self::Carcass => Color::srgb(0.8, 0.1, 0.1),
        }
    }
}

pub struct FoodPlugin;

impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (decay, decompose).chain());
    }
}

#[derive(Component)]
pub struct FoodDetails {
    /* FIXME: better name perhaps? The energy gained from consuming a
    particular food item depends on the amount of food (quality) and the
    digestive efficiency of that creature at consuming the food's kind */
    pub quality: f32,

    /* TODO: an explicit range from [-1, 1] for freshness as a newtype,
     * the nutype crate seems interesting for this */
    /* TODO: visualize freshness, lerp towards gray, ie.: -1 freshness (stale) is gray */
    pub freshness: f32,
    pub kind: FoodKind,
}

#[derive(Component)]
pub struct Food;

fn decay(
    time: Res<Time>,
    params: Res<SimParams>,
    world: Res<WorldGrid>,
    mut q: Query<(&mut FoodDetails, &WorldPos), With<Food>>,
) {
    for (mut details, pos) in &mut q {
        let cell = world_to_grid(pos.0);
        let multiplier = world.biome_at(cell).map_or(1.0, |b| b.decay_multiplier());

        details.freshness -= params.food_decay_rate * time.delta_secs() * multiplier / 5.0; /*Last number is to be edited so that the actual creatures cant finish off the food */
    }
}

fn decompose(mut commands: Commands, q: Query<(Entity, &FoodDetails, &WorldPos), With<Food>>) {
    let mut rng = rand::rng();
    let map_size = GRID_SIZE as f32 * TILE_SZ;
    for (entity, details, pos) in &q {
        if details.freshness <= -1.0 {
            commands.entity(entity).despawn();
            let roll = rng.random::<f32>();
            let offspring = if roll < 0.6 {
                1
            } else if roll < 0.8 {
                0
            } else {
                2
        };
        for _ in 0..offspring {
            let spread = TILE_SZ * 3.0;
            let offset = Vec2::new(
                rng.random_range(-spread..spread),
                rng.random_range(-spread..spread),
            );
            let new_pos = (pos.0 + offset).clamp(Vec2::ZERO, Vec2::splat(map_size - 0.01));
            let quality = rng.random_range(0.5..1.0);
            let freshness = rng.random_range(-0.5..1.0);
            commands.spawn(bundle(new_pos, FoodKind::Plant, quality, freshness));
        }
        }
    }

}

pub fn bundle(pos: Vec2, kind: FoodKind, quality: f32, freshness: f32) -> impl Bundle {
    /* TODO: food size proportional to quality */
    let sz = 8.0;

    let details = FoodDetails {
        kind,
        quality,
        freshness,
    };

    (
        Food,
        details,
        WorldPos(pos),
        Transform::default(),
        Sprite::from_color(kind.color(), Vec2::new(sz, sz / 2.0)),
    )
}
