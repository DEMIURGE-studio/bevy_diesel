use avian3d::prelude::*;
use bevy::prelude::*;

use bevy_diesel::invoker::InvokedBy;
use crate::prelude::GoOff;

/// Which ballistic trajectory to use when launching.
#[derive(Clone, Debug, Default, Reflect)]
pub enum Trajectory {
    #[default]
    LowAngle,
    HighAngle,
}

/// Sets `LinearVelocity` on the invoker when GoOff fires.
/// Does not manage character controller state.
#[derive(Component, Clone, Reflect)]
pub struct VelocityEffect {
    pub speed: f32,
    pub gravity: f32,
    pub trajectory: Trajectory,
    /// If set, the target position is clamped to exactly this distance from
    /// the invoker in the XZ plane. The arc always covers the same ground
    /// distance regardless of how far away the actual target is.
    pub fixed_range: Option<f32>,
}

impl Default for VelocityEffect {
    fn default() -> Self {
        Self {
            speed: 15.0,
            gravity: 9.81,
            trajectory: Trajectory::default(),
            fixed_range: None,
        }
    }
}

impl VelocityEffect {
    pub fn new(speed: f32, gravity: f32, trajectory: Trajectory) -> Self {
        Self {
            speed,
            gravity,
            trajectory,
            fixed_range: None,
        }
    }

    pub fn with_fixed_range(mut self, range: f32) -> Self {
        self.fixed_range = Some(range);
        self
    }
}

pub struct VelocityEffectPlugin;

impl Plugin for VelocityEffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, velocity_effect_system);
    }
}

fn velocity_effect_system(
    mut reader: MessageReader<GoOff>,
    q_velocity: Query<&VelocityEffect>,
    q_invoker: Query<&InvokedBy>,
    mut q_target: Query<(&Transform, &mut LinearVelocity)>,
) {
    for go_off in reader.read() {
        let effect_entity = go_off.entity;

        let target = &go_off.target;

        let Ok(velocity_effect) = q_velocity.get(effect_entity) else {
            continue;
        };

        let invoker_entity = q_invoker.root_ancestor(effect_entity);

        let Ok((transform, mut linear_velocity)) = q_target.get_mut(invoker_entity) else {
            continue;
        };

        let target_position = if let Some(range) = velocity_effect.fixed_range {
            // Clamp to fixed distance in XZ from the invoker.
            let origin = transform.translation;
            let dir_xz = Vec3::new(
                target.position.x - origin.x,
                0.0,
                target.position.z - origin.z,
            ).normalize_or_zero();
            origin + dir_xz * range
        } else {
            target.position
        };

        let calculated_velocity = match velocity_effect.trajectory {
            Trajectory::LowAngle => crate::ballistics::calculate_low_angle_velocity_with_speed(
                transform.translation,
                target_position,
                velocity_effect.speed,
                velocity_effect.gravity,
            ),
            Trajectory::HighAngle => crate::ballistics::calculate_high_angle_velocity_with_speed(
                transform.translation,
                target_position,
                velocity_effect.speed,
                velocity_effect.gravity,
            ),
        };

        linear_velocity.0 = calculated_velocity;
    }
}
