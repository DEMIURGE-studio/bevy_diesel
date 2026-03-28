use bevy::prelude::*;
use bevy_gauge::prelude::{Attributes, AttributesMut};

// ---------------------------------------------------------------------------
// InvokedBy / Invokes
// ---------------------------------------------------------------------------

/// Relationship target: collection of abilities/effects invoked by this entity.
#[derive(Component, Default, Debug, PartialEq, Eq)]
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

impl Invokes {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

/// Relationship component on an ability/effect pointing to its invoker (e.g. a character entity).
#[derive(Component, Clone, PartialEq, Eq, Debug)]
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

/// Walk the `ChildOf` chain to find the root ancestor entity.
pub fn resolve_root(q_child_of: &Query<&ChildOf>, entity: Entity) -> Entity {
    q_child_of.root_ancestor(entity)
}

// ---------------------------------------------------------------------------
// Gauge source auto-registration
// ---------------------------------------------------------------------------

/// Register the root invoker as a gauge attribute source when `InvokedBy` is added.
pub(crate) fn register_invoker_source(
    add: On<Add, InvokedBy>,
    q_invoker: Query<&InvokedBy>,
    mut attributes: AttributesMut,
) {
    let entity = add.entity;
    let invoker = q_invoker.root_ancestor(entity);
    attributes.register_source(entity, "invoked_by", invoker);
}

/// Update gauge sources when `InvokedBy` changes on entities that have `Attributes`.
pub(crate) fn on_invoked_by_changed_system(
    q_changed: Query<Entity, (Changed<InvokedBy>, With<Attributes>)>,
    q_invoker: Query<&InvokedBy>,
    mut attributes: AttributesMut,
) {
    for entity in q_changed.iter() {
        let invoker = q_invoker.root_ancestor(entity);
        attributes.register_source(entity, "invoked_by", invoker);
    }
}
