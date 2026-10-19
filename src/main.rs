mod camera;
mod creature;
mod iso;
mod sim;
mod ui;

use bevy::prelude::*;

use bevy_egui::EguiPlugin;

use iso::{TILE_H, TILE_W, world_to_screen};

const GRID_SIZE: i32 = 128;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mid = (GRID_SIZE - 1) as f32 / 2.0;
    let mid = world_to_screen(mid, mid);

    commands.spawn((Camera2d, Transform::from_xyz(mid.x, mid.y, 0.0)));

    let diamond = meshes.add(Rhombus::new(TILE_W, TILE_H));
    let light = materials.add(Color::srgb(0.72, 0.78, 0.60));
    let dark = materials.add(Color::srgb(0.62, 0.69, 0.50));

    for i in 0..GRID_SIZE {
        for j in 0..GRID_SIZE {
            let material = if (i + j) % 2 == 0 {
                light.clone()
            } else {
                dark.clone()
            };
            let p = world_to_screen(i as f32, j as f32);
            commands.spawn((
                Mesh2d(diamond.clone()),
                MeshMaterial2d(material),
                Transform::from_xyz(p.x, p.y, 0.0),
            ));
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(EguiPlugin::default())
        .add_plugins(camera::CameraControlPlugin)
        .add_plugins(creature::CreaturePlugin)
        .add_plugins(ui::UiPlugin)
        .add_systems(Startup, setup)
        .run();
}
