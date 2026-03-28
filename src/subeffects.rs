use bevy::prelude::*;
use bevy::ecs::hierarchy::{ChildSpawner, ChildSpawnerCommands};
use bevy::ecs::world::EntityWorldMut;
use bevy_gearbox::prelude::SubstateOf;

use crate::effect::SubEffectOf;
use crate::invoker::InvokedBy;

/// Extension trait for spawning sub-effects with less boilerplate.
///
/// Automatically inserts `SubstateOf`, `SubEffectOf`, and `InvokedBy`.
///
/// ```ignore
/// parent.spawn_subeffect(flying, root, (
///     Name::new("SpawnExplosion"),
///     SpawnConfig::at_passed("explosion"),
/// ));
/// ```
pub trait SpawnSubEffect {
    type Out<'a> where Self: 'a;

    /// Spawn a sub-effect under `state` with `invoker` as the root invoker.
    fn spawn_subeffect<B: Bundle>(
        &mut self,
        state: Entity,
        invoker: Entity,
        bundle: B,
    ) -> Self::Out<'_>;
}

impl SpawnSubEffect for ChildSpawnerCommands<'_> {
    type Out<'a> = EntityCommands<'a> where Self: 'a;

    fn spawn_subeffect<B: Bundle>(
        &mut self,
        state: Entity,
        invoker: Entity,
        bundle: B,
    ) -> EntityCommands<'_> {
        self.spawn((
            SubstateOf(state),
            SubEffectOf(state),
            InvokedBy(invoker),
            bundle,
        ))
    }
}

impl SpawnSubEffect for ChildSpawner<'_> {
    type Out<'a> = EntityWorldMut<'a> where Self: 'a;

    fn spawn_subeffect<B: Bundle>(
        &mut self,
        state: Entity,
        invoker: Entity,
        bundle: B,
    ) -> EntityWorldMut<'_> {
        self.spawn((
            SubstateOf(state),
            SubEffectOf(state),
            InvokedBy(invoker),
            bundle,
        ))
    }
}

impl SpawnSubEffect for Commands<'_, '_> {
    type Out<'a> = EntityCommands<'a> where Self: 'a;

    fn spawn_subeffect<B: Bundle>(
        &mut self,
        state: Entity,
        invoker: Entity,
        bundle: B,
    ) -> EntityCommands<'_> {
        self.spawn((
            SubstateOf(state),
            SubEffectOf(state),
            InvokedBy(invoker),
            bundle,
        ))
    }
}
