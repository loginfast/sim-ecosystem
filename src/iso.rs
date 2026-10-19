use bevy::prelude::*;

pub const TILE_W: f32 = 64.0;
pub const TILE_H: f32 = 32.0;

pub fn world_to_screen(x: f32, y: f32) -> Vec2 {
    Vec2::new((x - y) * TILE_W / 2.0, -(x + y) * TILE_H / 2.0)
}

#[allow(dead_code)]
pub fn screen_to_world(p: Vec2) -> Vec2 {
    Vec2::new(p.x / TILE_W - p.y / TILE_H, -p.x / TILE_W - p.y / TILE_H)
}
