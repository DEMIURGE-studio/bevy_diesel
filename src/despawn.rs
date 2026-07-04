use std::fmt::Debug;

use bevy::prelude::*;

use crate::diagnostics::diesel_debug;
use crate::effect::GoOff;

/// GoOff leaf effect: despawns the target the pipeline resolves for it.
/// The common case is a terminal `#Done` state that tears down the ability root:
///
/// ```ignore
/// #Done GoOffConfig::root() DespawnEffect
/// ```
///
/// On entry the state's `GoOffConfig` resolves the root as the target and fires
/// the pipeline; [`despawn_effect_system`] then despawns that target the same
/// frame. Despawning a state-machine root removes its whole substate/effect tree
/// via gearbox's `linked_spawn` relationships.
#[derive(Component, Reflect, Clone, Copy, Default, Debug)]
#[reflect(Component, Default)]
pub struct DespawnEffect;

/// When `GoOff` fires on an entity with [`DespawnEffect`], despawn the resolved
/// target entity (`go_off.target.entity`). Registered in
/// [`DieselSet::Effects`](crate::DieselSet::Effects) alongside the other leaf
/// effect systems, so the despawn happens the same frame the effect fires.
pub fn despawn_effect_system<P: Clone + Copy + Send + Sync + Default + Debug + 'static>(
    mut reader: MessageReader<GoOff<P>>,
    query: Query<&DespawnEffect>,
    mut commands: Commands,
) {
    for go_off in reader.read() {
        if query.get(go_off.entity).is_err() {
            diesel_debug!(
                "[bevy_diesel] despawn_effect_system: GoOff for {:?} but no DespawnEffect, skipping",
                go_off.entity,
            );
            continue;
        }

        let Some(target) = go_off.target.entity else {
            diesel_debug!(
                "[bevy_diesel] despawn_effect_system: GoOff for {:?} has a position-only \
                 target, nothing to despawn",
                go_off.entity,
            );
            continue;
        };

        if let Ok(mut ec) = commands.get_entity(target) {
            ec.try_despawn();
        }
    }
}
