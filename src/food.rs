use bevy::prelude::*;

use crate::creature::WorldPos;

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

#[derive(Component)]
pub struct Food {
    /* FIXME: better name perhaps? The energy gained from consuming a
    particular food item depends on the amount of food (quality) and the
    digestive efficiency of that creature at consuming the food's kind */
    pub quality: f32,

    /* TODO: decay */
    /* TODO: an explicit range from [-1, 1] for freshness as a newtype,
     * the nutype crate seems interesting for this */
    pub freshness: f32,
    pub kind: FoodKind,
}

impl Food {
    pub fn new(kind: FoodKind, quality: f32) -> Self {
        Self {
            kind,
            quality,
            freshness: 1.0,
        }
    }

    pub fn bundle(pos: Vec2, kind: FoodKind, quality: f32) -> impl Bundle {
        /* TODO: food size proportional to quality */
        let sz = 8.0;

        (
            Food::new(kind, quality),
            WorldPos(pos),
            Transform::default(),
            Sprite::from_color(kind.color(), Vec2::new(sz, sz / 2.0)),
        )
    }
}
