use bevy::{ecs::event::SetEntityEventTarget, prelude::*};
use std::marker::PhantomData;

// ================= Propagation =================

#[derive(Component, Debug, PartialEq, Eq, Reflect)]
#[relationship_target(relationship = PropagationTargetOf<T>, linked_spawn)]
#[reflect(Component, FromWorld)]
pub struct PropagationTargets<T: EntityEvent> {
    #[entities]
    #[relationship]
    entities: Vec<Entity>,
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T: EntityEvent> Default for PropagationTargets<T> {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            _pd: PhantomData,
        }
    }
}

impl<T: EntityEvent> PropagationTargets<T> {
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[relationship(relationship_target = PropagationTargets<T>)]
#[reflect(Component, FromWorld, Default)]
pub struct PropagationTargetOf<T: EntityEvent> {
    #[entities]
    #[relationship]
    pub entity: Entity,
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T: EntityEvent> Default for PropagationTargetOf<T> {
    fn default() -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            _pd: PhantomData,
        }
    }
}

impl<T: EntityEvent> PropagationTargetOf<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            _pd: PhantomData,
        }
    }
}

#[derive(EntityEvent, Clone, Debug, Reflect)]
pub struct RegisterPropagationTarget<T: EntityEvent> {
    #[event_target]
    pub target: Entity,
    pub source: Entity,
    _pd: PhantomData<T>,
}

pub fn register_propagation_target<T: EntityEvent>(
    e: On<RegisterPropagationTarget<T>>,
    mut commands: Commands,
) {
    commands
        .entity(e.target)
        .insert(PropagationTargetOf::<T>::new(e.source));
}

pub fn propagate_event<T: EntityEvent + SetEntityEventTarget + Clone>(
    event: On<T>,
    q_targets: Query<&PropagationTargets<T>>,
    mut commands: Commands,
) where
    <T as bevy::prelude::Event>::Trigger<'static>: std::default::Default,
{
    let source = event.event_target();
    let Ok(targets) = q_targets.get(source) else {
        return;
    };
    for &target in targets.entities.iter() {
        let mut new_event = event.clone();
        new_event.set_event_target(target);
        commands.trigger(new_event);
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct RegisterPropagationTargetRoot<T: EntityEvent> {
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T: EntityEvent> Default for RegisterPropagationTargetRoot<T> {
    fn default() -> Self {
        Self { _pd: PhantomData }
    }
}

pub fn register_propagation_target_root<T: EntityEvent>(
    q_register: Query<Entity, With<RegisterPropagationTargetRoot<T>>>,
    q_child_of: Query<&ChildOf>,
    mut commands: Commands,
) {
    for entity in q_register.iter() {
        let owner_root = q_child_of.root_ancestor(entity);
        commands
            .entity(entity)
            .insert(PropagationTargetOf::<T>::new(owner_root));
        commands
            .entity(entity)
            .try_remove::<RegisterPropagationTargetRoot<T>>();
    }
}

// ================= Inventory-backed Registration =================

pub struct PropagationRegistrar {
    pub register: fn(&mut App),
}

inventory::collect!(PropagationRegistrar);

/// Registers observers and reflected types needed for propagation for a given event type `T`.
pub fn register_propagation_for<
    T: EntityEvent + SetEntityEventTarget + Clone + Reflect + TypePath,
>(
    app: &mut App,
) where
    <T as bevy::prelude::Event>::Trigger<'static>: std::default::Default,
{
    app.add_observer(register_propagation_target::<T>)
        .add_observer(propagate_event::<T>)
        .register_type::<PropagationTargets<T>>()
        .register_type::<PropagationTargetOf<T>>()
        .add_systems(Update, register_propagation_target_root::<T>);
}

/// Function plugin that consumes all inventory submissions and applies their registrations.
pub fn plugin(app: &mut App) {
    for reg in inventory::iter::<PropagationRegistrar> {
        (reg.register)(app);
    }
}

#[macro_export]
macro_rules! submit_propagation_for {
    ($t:ty) => {
        inventory::submit! {
            $crate::propagation::PropagationRegistrar {
                register: |app| {
                    $crate::propagation::register_propagation_for::<$t>(app);
                },
            }
        }
    };
}
