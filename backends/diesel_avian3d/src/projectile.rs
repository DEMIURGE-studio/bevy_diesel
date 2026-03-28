use avian3d::prelude::*;
use bevy::prelude::*;

use crate::ballistics::calculate_low_angle_velocity_with_speed;
use crate::prelude::Target;

// ---------------------------------------------------------------------------
// ProjectileEffect - physics-driven ballistic arc
// ---------------------------------------------------------------------------

/// Ballistic projectile. Calculates launch velocity from `Target` and `Gravity`.
/// Without a `Target`, just falls.
#[derive(Component, Clone, Debug, Reflect)]
#[require(Sensor, CollisionEventsEnabled, RigidBody::Dynamic, Collider::sphere(0.2), Mass(1.0))]
pub struct ProjectileEffect {
    pub speed: f32,
}

impl Default for ProjectileEffect {
    fn default() -> Self {
        Self { speed: 20.0 }
    }
}

impl ProjectileEffect {
    pub fn new(speed: f32) -> Self {
        Self { speed }
    }
}

// ---------------------------------------------------------------------------
// LinearProjectileEffect - straight-line constant speed
// ---------------------------------------------------------------------------

/// Straight-line projectile at constant speed, ignoring gravity.
#[derive(Component, Clone, Debug, Reflect)]
#[require(Sensor, CollisionEventsEnabled, RigidBody::Kinematic, Collider::sphere(0.2))]
pub struct LinearProjectileEffect {
    pub speed: f32,
}

impl Default for LinearProjectileEffect {
    fn default() -> Self {
        Self { speed: 20.0 }
    }
}

impl LinearProjectileEffect {
    pub fn new(speed: f32) -> Self {
        Self { speed }
    }
}

/// Inserted when a linear projectile receives its `Target`.
#[derive(Component)]
pub struct LinearProjectile {
    pub direction: Vec3,
    pub speed: f32,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(init_ballistic_target)
            .add_observer(init_linear_target)
            .add_systems(Update, move_linear_projectiles);
    }
}

// ---------------------------------------------------------------------------
// Ballistic - set velocity when Target is added
// ---------------------------------------------------------------------------

fn init_ballistic_target(
    add: On<Add, Target>,
    q_projectile: Query<(&Target, &Transform, &ProjectileEffect, Option<&GravityScale>)>,
    gravity: Res<Gravity>,
    mut commands: Commands,
) {
    let entity = add.entity;
    let Ok((target, transform, effect, gravity_scale)) = q_projectile.get(entity) else {
        return;
    };

    let scale = gravity_scale.map(|gs| gs.0).unwrap_or(1.0);
    let effective_gravity = (gravity.0 * scale).length();

    let velocity = calculate_low_angle_velocity_with_speed(
        transform.translation,
        target.position,
        effect.speed,
        effective_gravity,
    );

    commands.entity(entity).insert(LinearVelocity(velocity));
}

// ---------------------------------------------------------------------------
// Linear - calculate direction and insert runtime component
// ---------------------------------------------------------------------------

fn init_linear_target(
    add: On<Add, Target>,
    q_projectile: Query<(&Target, &Transform, &LinearProjectileEffect)>,
    mut commands: Commands,
) {
    let entity = add.entity;
    let Ok((target, transform, effect)) = q_projectile.get(entity) else {
        return;
    };

    let direction = (target.position - transform.translation).normalize_or_zero();
    let direction = if direction == Vec3::ZERO {
        Vec3::NEG_Y
    } else {
        direction
    };

    commands.entity(entity).insert(LinearProjectile {
        direction,
        speed: effect.speed,
    });
}

// ---------------------------------------------------------------------------
// Linear movement system
// ---------------------------------------------------------------------------

fn move_linear_projectiles(
    mut q_projectile: Query<(&mut Transform, &LinearProjectile)>,
    time: Res<Time>,
) {
    for (mut transform, projectile) in q_projectile.iter_mut() {
        transform.translation += projectile.direction * projectile.speed * time.delta_secs();
    }
}
