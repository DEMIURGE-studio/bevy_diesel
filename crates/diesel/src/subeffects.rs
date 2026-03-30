use bevy::prelude::*;
use bevy::ecs::hierarchy::{ChildSpawner, ChildSpawnerCommands};
use bevy::ecs::world::EntityWorldMut;
use bevy_gearbox::prelude::SubstateOf;

use crate::effect::SubEffectOf;
use crate::invoker::InvokedBy;

// ---------------------------------------------------------------------------
// SpawnDieselSubstate — like gearbox's SpawnSubstate but adds InvokedBy(root)
// ---------------------------------------------------------------------------

/// Extension trait for spawning diesel substates with automatic `InvokedBy`.
///
/// Uses `target_entity()` (the entity `with_children` was called on) as the
/// invoker. Since diesel templates always call `with_children` on the template
/// root, all substates get `InvokedBy(root)` automatically.
pub trait SpawnDieselSubstate {
    type Out<'a> where Self: 'a;

    fn spawn_diesel_substate<B: Bundle>(
        &mut self,
        parent: Entity,
        bundle: B,
    ) -> Self::Out<'_>;
}

impl SpawnDieselSubstate for ChildSpawnerCommands<'_> {
    type Out<'a> = EntityCommands<'a> where Self: 'a;

    fn spawn_diesel_substate<B: Bundle>(
        &mut self,
        parent: Entity,
        bundle: B,
    ) -> EntityCommands<'_> {
        let invoker = self.target_entity();
        self.spawn((SubstateOf(parent), InvokedBy(invoker), bundle))
    }
}

// ---------------------------------------------------------------------------
// SpawnSubEffect
// ---------------------------------------------------------------------------

/// Extension trait for spawning sub-effects with less boilerplate.
///
/// Automatically inserts `SubstateOf`, `SubEffectOf`, and `InvokedBy(root)`
/// using `target_entity()`.
///
/// ```ignore
/// parent.spawn_subeffect(flying, (
///     Name::new("SpawnExplosion"),
///     SpawnConfig::at_passed("explosion"),
/// ));
/// ```
pub trait SpawnSubEffect {
    type Out<'a> where Self: 'a;

    fn spawn_subeffect<B: Bundle>(
        &mut self,
        state: Entity,
        bundle: B,
    ) -> Self::Out<'_>;
}

impl SpawnSubEffect for ChildSpawnerCommands<'_> {
    type Out<'a> = EntityCommands<'a> where Self: 'a;

    fn spawn_subeffect<B: Bundle>(
        &mut self,
        state: Entity,
        bundle: B,
    ) -> EntityCommands<'_> {
        let invoker = self.target_entity();
        self.spawn((
            SubstateOf(state),
            SubEffectOf(state),
            InvokedBy(invoker),
            bundle,
        ))
    }
}
