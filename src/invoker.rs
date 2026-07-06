use bevy::prelude::*;
use bevy_gauge::prelude::{Attributes, AttributesMut};

// ---------------------------------------------------------------------------
// InvokedBy / Invokes
// ---------------------------------------------------------------------------

/// Relationship target: collection of abilities/effects invoked by this entity.
#[derive(Component, Default, Clone, Debug, PartialEq, Eq)]
#[relationship_target(relationship = InvokedBy, linked_spawn)]
pub struct Invokes(Vec<Entity>);

impl<'a> IntoIterator for &'a Invokes {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = std::slice::Iter<'a, Entity>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// Relationship component on an ability/effect pointing to its invoker (e.g. a character entity).
#[derive(Component, Clone, PartialEq, Eq, Debug, FromTemplate)]
#[relationship(relationship_target = Invokes)]
pub struct InvokedBy(#[entities] pub Entity);

impl FromWorld for InvokedBy {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        InvokedBy(Entity::PLACEHOLDER)
    }
}

// ---------------------------------------------------------------------------
// Resolution helpers
// ---------------------------------------------------------------------------

/// Walk the `InvokedBy` chain to find the root invoker entity.
pub fn resolve_invoker(q_invoker: &Query<&InvokedBy>, entity: Entity) -> Entity {
    q_invoker.root_ancestor(entity)
}

/// Walk the `SubstateOf` chain to the state-machine (scene) root entity.
///
/// diesel effects live in gearbox's *state* hierarchy (`SubstateOf`), so this
/// finds the ability / spawned-scene root (the entity carrying the `Transform`,
/// used by `TargetType::Root` and `root_gathering`) from any effect/sub-state
/// entity.
pub fn resolve_root(
    q_substate_of: &Query<&bevy_gearbox::SubstateOf>,
    entity: Entity,
) -> Entity {
    q_substate_of.root_ancestor(entity)
}

// ---------------------------------------------------------------------------
// Gauge source auto-registration
// ---------------------------------------------------------------------------

/// Register gauge sources when `InvokedBy` is added: `@invoker` (the root of the
/// invoker chain, the player) and `@ability` (the nearest [`Ability`] ancestor,
/// the spell), so sub-state expressions like `"Cooldown@ability"` resolve.
pub(crate) fn register_invoker_source(
    add: On<Add, InvokedBy>,
    q_invoker: Query<&InvokedBy>,
    q_ability: Query<(), With<crate::invoke::Ability>>,
    mut attributes: AttributesMut,
) {
    let entity = add.entity;
    let invoker = q_invoker.root_ancestor(entity);
    attributes.register_source(entity, "invoker", invoker);
    if let Some(ability) = crate::spawn::find_ability(entity, &q_invoker, &q_ability) {
        attributes.register_source(entity, "ability", ability);
    }
}

/// Update `@invoker`/`@ability` sources when `InvokedBy` changes on entities that
/// have `Attributes`.
pub(crate) fn on_invoker_changed_system(
    q_changed: Query<Entity, (Changed<InvokedBy>, With<Attributes>)>,
    q_invoker: Query<&InvokedBy>,
    q_ability: Query<(), With<crate::invoke::Ability>>,
    mut attributes: AttributesMut,
) {
    for entity in q_changed.iter() {
        let invoker = q_invoker.root_ancestor(entity);
        attributes.register_source(entity, "invoker", invoker);
        if let Some(ability) = crate::spawn::find_ability(entity, &q_invoker, &q_ability) {
            attributes.register_source(entity, "ability", ability);
        }
    }
}
