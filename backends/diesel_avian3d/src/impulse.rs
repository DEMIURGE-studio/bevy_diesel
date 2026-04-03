use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_diesel::effect::GoOff;
use bevy_diesel::invoker::InvokedBy;

// ---------------------------------------------------------------------------
// ImpulseEffect — apply physics impulse to targets on GoOff
// ---------------------------------------------------------------------------

/// Sub-effect that applies a linear impulse to target entities, pushing them
/// away from the effect entity's position. Use as a sub-effect alongside
/// instant damage for knockback, explosions, etc.
///
/// For angular flinch, set `angular` to a non-zero value — a random-direction
/// angular impulse is applied to make targets wobble on hit.
#[derive(Component, Clone, Debug)]
pub struct ImpulseEffect {
    /// Linear impulse magnitude (direction: away from effect source).
    pub force: f32,
    /// Upward component added to the impulse direction.
    pub vertical: f32,
    /// Angular impulse magnitude (random axis).
    pub angular: f32,
}

impl ImpulseEffect {
    pub fn knockback(force: f32) -> Self {
        Self {
            force,
            vertical: 0.0,
            angular: 0.0,
        }
    }

    pub fn with_vertical(mut self, vertical: f32) -> Self {
        self.vertical = vertical;
        self
    }

    pub fn with_angular(mut self, angular: f32) -> Self {
        self.angular = angular;
        self
    }
}

/// System that reads `GoOff` messages and applies impulses to target entities.
/// Runs in `DieselSet::Effects`.
pub fn impulse_effect_system(
    mut reader: MessageReader<GoOff<Vec3>>,
    q_effect: Query<&ImpulseEffect>,
    q_child_of: Query<&ChildOf>,
    q_transform: Query<&Transform>,
    mut q_velocity: Query<&mut LinearVelocity>,
    mut q_angular: Query<&mut AngularVelocity>,
) {
    for go_off in reader.read() {
        let effect_entity = go_off.entity;
        let Ok(impulse) = q_effect.get(effect_entity) else {
            continue;
        };

        let Some(target_entity) = go_off.target.entity else {
            continue;
        };

        // Source position: the effect entity itself
        let source = q_child_of.root_ancestor(effect_entity);
        let source_pos = q_transform
            .get(source)
            .map(|t| t.translation)
            .unwrap_or(go_off.target.position);

        let target_pos = q_transform
            .get(target_entity)
            .map(|t| t.translation)
            .unwrap_or(go_off.target.position);

        // Linear knockback: away from source
        if impulse.force > 0.0 {
            let flat_dir = (target_pos - source_pos).normalize_or_zero();
            let push = Vec3::new(flat_dir.x, 0.0, flat_dir.z).normalize_or_zero() * impulse.force
                + Vec3::Y * impulse.vertical;

            if let Ok(mut vel) = q_velocity.get_mut(target_entity) {
                vel.0 += push;
            }
        }

        // Angular flinch
        if impulse.angular > 0.0 {
            if let Ok(mut ang) = q_angular.get_mut(target_entity) {
                let dir = (target_pos - source_pos).normalize_or_zero();
                let torque = Vec3::new(-dir.z, 0.0, dir.x) * impulse.angular;
                ang.0 += torque;
            }
        }
    }
}
