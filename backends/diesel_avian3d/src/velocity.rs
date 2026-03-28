use avian3d::prelude::*;
use bevy::prelude::*;

use bevy_diesel::invoker::InvokedBy;
use crate::ballistics::{
    calculate_high_angle_velocity_with_speed, calculate_low_angle_velocity_with_speed,
};
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
}

impl Default for VelocityEffect {
    fn default() -> Self {
        Self {
            speed: 15.0,
            gravity: 9.81,
            trajectory: Trajectory::default(),
        }
    }
}

impl VelocityEffect {
    pub fn new(speed: f32, gravity: f32, trajectory: Trajectory) -> Self {
        Self {
            speed,
            gravity,
            trajectory,
        }
    }
}

pub struct VelocityEffectPlugin;

impl Plugin for VelocityEffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(velocity_effect_observer);
    }
}

fn velocity_effect_observer(
    go_off: On<GoOff>,
    q_velocity: Query<&VelocityEffect>,
    q_invoker: Query<&InvokedBy>,
    mut q_target: Query<(&Transform, &mut LinearVelocity)>,
) {
    let effect_entity = go_off.entity;

    let Some(target) = go_off.targets.first() else {
        return;
    };

    let Ok(velocity_effect) = q_velocity.get(effect_entity) else {
        return;
    };

    let invoker_entity = q_invoker.root_ancestor(effect_entity);

    let Ok((transform, mut linear_velocity)) = q_target.get_mut(invoker_entity) else {
        return;
    };

    let target_position = target.position;

    let calculated_velocity = match velocity_effect.trajectory {
        Trajectory::LowAngle => calculate_low_angle_velocity_with_speed(
            transform.translation,
            target_position,
            velocity_effect.speed,
            velocity_effect.gravity,
        ),
        Trajectory::HighAngle => calculate_high_angle_velocity_with_speed(
            transform.translation,
            target_position,
            velocity_effect.speed,
            velocity_effect.gravity,
        ),
    };

    linear_velocity.0 = calculated_velocity;
}
