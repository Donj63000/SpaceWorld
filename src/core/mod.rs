mod performance;

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

pub use performance::{
    ParametresPerformanceJeu, PerformanceJeuPlugin, PresetPerformance, StatistiquesPerformance,
};

pub const SIMULATION_HZ: f64 = 4.0;
const CAMERA_PAN_SPEED: f32 = 36.0;
const MIN_ZOOM: f32 = 22.0;
const MAX_ZOOM: f32 = 120.0;
pub const CAMERA_ORBIT_DIRECTION: Vec3 = Vec3::new(-0.85, 0.78, 0.88);

pub struct CorePlugin;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    Boot,
    Intro,
    InGame,
    Paused,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct CameraController {
    pub focus_world: Vec2,
    pub zoom: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            focus_world: Vec2::ZERO,
            zoom: 52.0,
        }
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct WorldOrigin(pub Vec2);

#[derive(Resource, Debug, Clone)]
pub struct CameraOverride {
    pub transform: Transform,
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
struct SunLight;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PerformanceJeuPlugin)
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.71, 0.54, 0.46),
                brightness: 760.0,
                ..default()
            })
            .insert_resource(CameraController::default())
            .insert_resource(WorldOrigin::default())
            .add_systems(Startup, (setup_scene, boot_into_intro).chain())
            .add_systems(
                Update,
                (toggle_pause, control_camera, sync_camera, sync_sun_light),
            );
    }
}

fn setup_scene(mut commands: Commands, perf: Res<ParametresPerformanceJeu>) {
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.94, 0.88),
            illuminance: 14_500.0,
            shadows_enabled: perf.ombres_directionnelles,
            ..default()
        },
        Transform::from_xyz(36.0, 52.0, 28.0).looking_at(Vec3::ZERO, Vec3::Y),
        SunLight,
    ));

    commands.spawn((
        Camera3d::default(),
        DistanceFog {
            color: Color::srgba(0.79, 0.55, 0.45, 0.94),
            falloff: FogFalloff::Linear {
                start: 96.0,
                end: 240.0,
            },
            ..default()
        },
        Transform::from_xyz(-32.0, 28.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
    ));
}

fn boot_into_intro(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Intro);
}

fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    match state.get() {
        GameState::Boot | GameState::Intro => {}
        GameState::InGame => next_state.set(GameState::Paused),
        GameState::Paused => next_state.set(GameState::InGame),
    }
}

fn control_camera(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    state: Res<State<GameState>>,
    mut controller: ResMut<CameraController>,
) {
    if matches!(state.get(), GameState::Boot | GameState::Intro) {
        for _ in mouse_wheel.read() {}
        return;
    }

    let mut axis = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyZ)
        || keyboard.pressed(KeyCode::KeyW)
        || keyboard.pressed(KeyCode::ArrowUp)
    {
        axis.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        axis.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyQ)
        || keyboard.pressed(KeyCode::KeyA)
        || keyboard.pressed(KeyCode::ArrowLeft)
    {
        axis.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        axis.x += 1.0;
    }

    if axis.length_squared() > 0.0 {
        controller.focus_world += camera_pan_direction(axis) * CAMERA_PAN_SPEED * time.delta_secs();
    }

    for wheel in mouse_wheel.read() {
        controller.zoom = (controller.zoom - wheel.y * 3.0).clamp(MIN_ZOOM, MAX_ZOOM);
    }
}

fn camera_pan_direction(axis: Vec2) -> Vec2 {
    let forward = Vec2::new(-CAMERA_ORBIT_DIRECTION.x, -CAMERA_ORBIT_DIRECTION.z).normalize();
    let right = Vec2::new(-forward.y, forward.x);
    (right * axis.x + forward * axis.y).normalize_or_zero()
}

fn sync_camera(
    controller: Res<CameraController>,
    origin: Res<WorldOrigin>,
    camera_override: Option<Res<CameraOverride>>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
) {
    let override_changed = camera_override
        .as_ref()
        .map(|camera_override| camera_override.is_changed())
        .unwrap_or(false);
    if !controller.is_changed() && !origin.is_changed() && !override_changed {
        return;
    }

    if let Some(camera_override) = camera_override {
        **camera = camera_override.transform;
        return;
    }

    let focus = Vec3::new(
        controller.focus_world.x - origin.0.x,
        0.0,
        controller.focus_world.y - origin.0.y,
    );
    let camera_offset = CAMERA_ORBIT_DIRECTION.normalize() * controller.zoom;
    **camera = Transform::from_translation(focus + camera_offset).looking_at(focus, Vec3::Y);
}

fn sync_sun_light(
    controller: Res<CameraController>,
    origin: Res<WorldOrigin>,
    mut light: Single<&mut Transform, (With<SunLight>, Without<MainCamera>)>,
) {
    if !controller.is_changed() && !origin.is_changed() {
        return;
    }

    let focus = Vec3::new(
        controller.focus_world.x - origin.0.x,
        0.0,
        controller.focus_world.y - origin.0.y,
    );
    **light =
        Transform::from_translation(focus + Vec3::new(36.0, 52.0, 28.0)).looking_at(focus, Vec3::Y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_input_uses_camera_forward_projection() {
        let direction = camera_pan_direction(Vec2::new(0.0, 1.0));
        let expected = Vec2::new(-CAMERA_ORBIT_DIRECTION.x, -CAMERA_ORBIT_DIRECTION.z).normalize();
        assert!((direction - expected).length() < 0.001);
    }

    #[test]
    fn right_input_uses_camera_screen_right_projection() {
        let direction = camera_pan_direction(Vec2::new(1.0, 0.0));
        let forward = Vec2::new(-CAMERA_ORBIT_DIRECTION.x, -CAMERA_ORBIT_DIRECTION.z).normalize();
        let expected = Vec2::new(-forward.y, forward.x);
        assert!((direction - expected).length() < 0.001);
    }
}
