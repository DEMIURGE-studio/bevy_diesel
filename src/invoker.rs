use bevy::prelude::*;

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
