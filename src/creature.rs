use bevy::prelude::*;

use crate::GRID_SIZE;
use crate::TILE_SZ;
use crate::food::{Food, FoodKind};
use crate::grid_to_world;
use crate::sim::SimParams;

#[derive(Component)]
pub struct Creature;

#[derive(Component)]
pub struct GridPos(pub IVec2);

#[derive(Component)]
pub struct WorldPos(pub Vec2);

#[derive(Component, Default)]
pub struct Facing(pub Option<IVec2>);

#[derive(Component)]
pub struct Genes {
    pub speed: f32,
}

#[derive(Component)]
pub struct Stats {
    pub energy: f32,
}

pub struct CreaturePlugin;

impl Plugin for CreaturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimParams>()
            .add_systems(Startup, spawn)
            .add_systems(Update, (update, facing))
            .add_systems(FixedUpdate, (start_step, advance, metabolize, die).chain());
    }
}

fn facing(keys: Res<ButtonInput<KeyCode>>, mut q: Query<&mut Facing, With<Creature>>) {
    let dir = if keys.pressed(KeyCode::ArrowUp) {
        Some(IVec2::new(0, 1))
    } else if keys.pressed(KeyCode::ArrowRight) {
        Some(IVec2::new(1, 0))
    } else if keys.pressed(KeyCode::ArrowDown) {
        Some(IVec2::new(0, -1))
    } else if keys.pressed(KeyCode::ArrowLeft) {
        Some(IVec2::new(-1, 0))
    } else {
        None
    };
    for mut f in &mut q {
        f.0 = dir;
    }
}

fn advance(time: Res<Time>, mut q: Query<(&mut WorldPos, &GridPos, &Genes), With<Creature>>) {
    for (mut pos, grid, gene) in &mut q {
        let to = grid_to_world(grid.0) - pos.0;
        let dist = to.length();
        let step = gene.speed * TILE_SZ * time.delta_secs();

        pos.0 = if dist <= step {
            grid_to_world(grid.0)
        } else {
            pos.0 + to / dist * step
        };
    }
}

fn start_step(mut q: Query<(&mut GridPos, &WorldPos, &Facing), With<Creature>>) {
    for (mut gpos, pos, intent) in &mut q {
        if pos.0.distance(grid_to_world(gpos.0)) > 1e-4 {
            continue;
        }

        let Some(dir) = intent.0 else { continue };
        let next = gpos.0 + dir;
        if next.cmpge(IVec2::ZERO).all() && next.cmplt(IVec2::splat(GRID_SIZE)).all() {
            gpos.0 = next;
        }
    }
}

/* TODO: update is just a general drawing function at this point,
 *       maybe abstract this to it's own render module?
 * FIXME: introduce an explicit Z-Ordering for shadows, food, creatures, etc. */
fn update(mut q: Query<(&WorldPos, &mut Transform), Or<(With<Creature>, With<Food>)>>) {
    /* Introduce z-ordering for isometric projection */
    const DEPTH_EPS: f32 = 0.001;

    for (p, mut t) in &mut q {
        t.translation = p.0.extend(1.0 + (p.0.x + p.0.y) * DEPTH_EPS);
    }
}

fn metabolize(
    time: Res<Time>,
    params: Res<SimParams>,
    mut q: Query<(&Genes, &mut Stats, &WorldPos, &GridPos), With<Creature>>,
) {
    for (genes, mut stats, pos, gpos) in &mut q {
        /* FIXME: better way to sort out moving?
         * the current system just checks whether the sprite is very close to
         * the tile center and accepts that as stationary. This could be a bug */

        let moving = pos.0.distance(gpos.0.as_vec2()) > 1e-4;

        let moving = if moving { 1.0 } else { 0.0 };

        /* TODO: possibly a better abstraction for calculating cost of particular
         * characterestics / stats? */

        let cost = params.base_cost + params.movement_cost * genes.speed * moving;

        stats.energy -= (time.delta_secs() * cost).max(0.0);
    }
}

fn die(mut commands: Commands, q: Query<(Entity, &Genes, &WorldPos, &Stats), With<Creature>>) {
    for (entity, _, pos, Stats { energy }) in &q {
        if *energy <= 0.0 {
            commands.spawn(Food::bundle(
                pos.0,
                FoodKind::Carcass,
                20.0, /* TODO: food quality proportional to size gene and energy */
            ));

            commands.entity(entity).despawn();
        }
    }
}

fn spawn(mut commands: Commands) {
    /* TODO: procedural color
     * TODO: default position */

    let gposition = IVec2::new(0, 0);
    let position = gposition.as_vec2() * TILE_SZ;
    let genes = Genes { speed: 4.0 };

    commands
        .spawn((
            Creature,
            WorldPos(position),
            GridPos(gposition),
            Facing::default(),
            genes,
            Stats { energy: 100.0 },
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|c| {
            c.spawn((
                Sprite::from_color(
                    Color::srgb(0.2, 0.7, 0.5),
                    Vec2::new(TILE_SZ * 0.5, TILE_SZ * 0.5),
                ),
                Transform::from_translation(position.extend(0.5)),
            ));
        });
}
