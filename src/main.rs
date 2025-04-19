#![allow(clippy::needless_range_loop)]

use bevy::prelude::*;

const GRID_WIDTH: usize = 200;
const GRID_HEIGHT: usize = 200;
const CELL_SIZE: f32 = 10.0;
const CAMERA_SPEED: f32 = 300.;
const AUTO_STEP_INTERVAL: f32 = 0.2;

#[derive(Component)]
struct Cell {
    x: usize,
    y: usize,
    alive: bool,
}

#[derive(Resource)]
struct CellGrid {
    current: Vec<Vec<bool>>,
    next: Vec<Vec<bool>>,
    alive_material: Handle<ColorMaterial>,
    dead_material: Handle<ColorMaterial>,
}

#[derive(Resource)]
struct GameState {
    auto_play: bool,
    timer: Timer,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            auto_play: false,
            timer: Timer::from_seconds(AUTO_STEP_INTERVAL, TimerMode::Repeating),
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<GameState>()
        .add_systems(Startup, (setup_cells, setup_camera))
        .add_systems(
            Update,
            (
                move_camera,
                toggle_auto_play,
                reset_grid,
                auto_step_game_of_life,
                update_cell_materials,
            ),
        )
        .run();
}

fn setup_cells(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(Rectangle::new(CELL_SIZE - 1.0, CELL_SIZE - 1.0));
    let alive_material = materials.add(ColorMaterial::from(Color::WHITE));
    let dead_material = materials.add(ColorMaterial::from(Color::BLACK));

    let mut current = vec![vec![false; GRID_HEIGHT]; GRID_WIDTH];
    let next = vec![vec![false; GRID_HEIGHT]; GRID_WIDTH];

    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            let alive = rand::random_bool(0.1);
            current[x][y] = alive;

            let material = if alive {
                alive_material.clone()
            } else {
                dead_material.clone()
            };

            commands.spawn((
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material),
                Transform::from_xyz(x as f32 * CELL_SIZE, y as f32 * CELL_SIZE, 0.0),
                GlobalTransform::default(),
                Cell { x, y, alive },
            ));
        }
    }

    commands.insert_resource(CellGrid {
        current,
        next,
        alive_material,
        dead_material,
    });
}

fn setup_camera(mut commands: Commands) {
    let grid_width_pixels = GRID_WIDTH as f32 * CELL_SIZE;
    let grid_height_pixels = GRID_HEIGHT as f32 * CELL_SIZE;

    commands.spawn((
        Camera2d,
        Camera {
            hdr: true,
            ..default()
        },
        Transform::from_xyz(grid_width_pixels / 2.0, grid_height_pixels / 2.0, 0.0)
            .with_scale(Vec3::new(0.1, 0.1, 1.0)),
    ));
}

fn move_camera(
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    time: Res<Time>,
    kb_input: Res<ButtonInput<KeyCode>>,
) {
    let mut direction = Vec2::ZERO;
    // let mut rotation_change = 0.0;
    let mut scale_change = 0.0;

    if kb_input.pressed(KeyCode::KeyW) {
        direction.y += 1.;
    }
    if kb_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.;
    }
    if kb_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.;
    }
    if kb_input.pressed(KeyCode::KeyD) {
        direction.x += 1.;
    }

    // if kb_input.pressed(KeyCode::KeyH) {
    //     rotation_change += 0.5;
    // }
    // if kb_input.pressed(KeyCode::KeyL) {
    //     rotation_change -= 0.5;
    // }

    if kb_input.pressed(KeyCode::KeyJ) {
        scale_change += 1.0;
    }
    if kb_input.pressed(KeyCode::KeyK) {
        scale_change -= 1.0;
    }

    let move_delta = direction.normalize_or_zero() * CAMERA_SPEED * time.delta_secs();
    // let rotation_delta = rotation_change * 2.0 * time.delta_secs();
    let scale_delta = scale_change * 0.5 * time.delta_secs();

    if let Ok(mut transform) = camera_query.get_single_mut() {
        transform.translation += move_delta.extend(0.);

        // transform.rotate_z(rotation_delta);

        let new_scale = (transform.scale + Vec3::splat(scale_delta)).max(Vec3::splat(0.1));
        transform.scale = new_scale;
    }
}

fn toggle_auto_play(kb_input: Res<ButtonInput<KeyCode>>, mut game_state: ResMut<GameState>) {
    if kb_input.just_pressed(KeyCode::Space) {
        game_state.auto_play = !game_state.auto_play;
        game_state.timer.reset();
    }
}

fn reset_grid(
    kb_input: Res<ButtonInput<KeyCode>>,
    mut grid: ResMut<CellGrid>,
    mut query: Query<&mut Cell>,
) {
    if kb_input.just_pressed(KeyCode::KeyR) {
        for x in 0..GRID_WIDTH {
            for y in 0..GRID_HEIGHT {
                let alive = rand::random_bool(0.1);
                grid.current[x][y] = alive;
                grid.next[x][y] = alive;
            }
        }

        for mut cell in &mut query {
            cell.alive = grid.current[cell.x][cell.y];
        }
    }
}

fn auto_step_game_of_life(
    time: Res<Time>,
    mut game_state: ResMut<GameState>,
    mut grid: ResMut<CellGrid>,
    mut query: Query<&mut Cell>,
) {
    if !game_state.auto_play {
        return;
    }

    game_state.timer.tick(time.delta());

    if game_state.timer.just_finished() {
        for x in 0..GRID_WIDTH {
            for y in 0..GRID_HEIGHT {
                let alive_neighbors = count_alive_neighbors(&grid.current, x, y);
                let alive = grid.current[x][y];

                grid.next[x][y] = matches!((alive, alive_neighbors), (true, 2..=3) | (false, 3));
            }
        }

        for x in 0..GRID_WIDTH {
            for y in 0..GRID_HEIGHT {
                grid.current[x][y] = grid.next[x][y];
            }
        }
        for mut cell in &mut query {
            cell.alive = grid.current[cell.x][cell.y];
        }
    }
}

fn count_alive_neighbors(grid: &[Vec<bool>], x: usize, y: usize) -> usize {
    let mut count = 0;

    for dx in [-1i32, 0, 1] {
        for dy in [-1i32, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }

            let nx = x as i32 + dx;
            let ny = y as i32 + dy;

            if nx >= 0
                && ny >= 0
                && (nx as usize) < GRID_WIDTH
                && (ny as usize) < GRID_HEIGHT
                && grid[nx as usize][ny as usize]
            {
                count += 1;
            }
        }
    }

    count
}

fn update_cell_materials(
    mut query: Query<(&Cell, &mut MeshMaterial2d<ColorMaterial>)>,
    grid: Res<CellGrid>,
) {
    for (cell, mut material) in &mut query {
        let expected = if cell.alive {
            &grid.alive_material
        } else {
            &grid.dead_material
        };
        if &material.0 != expected {
            material.0 = expected.clone();
        }
    }
}
