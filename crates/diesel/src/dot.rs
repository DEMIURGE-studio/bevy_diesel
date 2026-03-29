use bevy::prelude::*;
use std::marker::PhantomData;

// ================= Periodic Effect Relationships =================

/// Entities affected by a periodic effect of type T.
#[derive(Component, Debug, PartialEq, Eq, Reflect)]
#[relationship_target(relationship = PeriodicEffectTarget<T>, linked_spawn)]
#[reflect(Component, FromWorld)]
pub struct PeriodicEffectTargets<T: Reflect> {
    #[entities]
    #[relationship]
    entities: Vec<Entity>,
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T: Reflect> Default for PeriodicEffectTargets<T> {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            _pd: PhantomData,
        }
    }
}

impl<T: Reflect> PeriodicEffectTargets<T> {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.entities.iter().copied()
    }
}

/// Points an affected entity back to the periodic effect of type T.
#[derive(Component, Clone, Debug, Reflect)]
#[relationship(relationship_target = PeriodicEffectTargets<T>)]
#[reflect(Component, FromWorld, Default)]
pub struct PeriodicEffectTarget<T: Reflect> {
    #[entities]
    #[relationship]
    pub entity: Entity,
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T: Reflect> Default for PeriodicEffectTarget<T> {
    fn default() -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            _pd: PhantomData,
        }
    }
}

impl<T: Reflect> PeriodicEffectTarget<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            _pd: PhantomData,
        }
    }
}

// ================= Periodic Tick Event =================

/// Fired each tick of a periodic effect. Observe to apply damage, healing, etc.
#[derive(EntityEvent, Clone, Debug, Reflect)]
pub struct PeriodicTick<T: Reflect> {
    #[event_target]
    pub target: Entity,
    pub source: Entity,
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T: Reflect> PeriodicTick<T> {
    pub fn new(source: Entity, target: Entity) -> Self {
        Self {
            target,
            source,
            _pd: PhantomData,
        }
    }
}

// ================= Tick System =================

/// Fires `PeriodicTick<T>` for all active effects. Run on a timer to control tick rate.
pub fn periodic_tick_system<T: Reflect + Component>(
    q_effects: Query<(Entity, &PeriodicEffectTargets<T>)>,
    mut commands: Commands,
) {
    for (source, targets) in q_effects.iter() {
        for target in targets.iter() {
            commands.trigger(PeriodicTick::<T>::new(source, target));
        }
    }
}
