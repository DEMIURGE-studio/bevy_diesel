use std::fmt::Debug;

use bevy::prelude::*;

use crate::target::Target;

// ---------------------------------------------------------------------------
// GoOff<P>
// ---------------------------------------------------------------------------

/// The central propagation event. When triggered on an effect entity, it delivers
/// the resolved target list and kicks off sub-effect propagation.
#[derive(EntityEvent, Clone)]
pub struct GoOff<P: Clone + Copy + Send + Sync + Default + Debug + 'static> {
    #[event_target]
    pub entity: Entity,
    pub targets: Vec<Target<P>>,
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> GoOff<P> {
    pub fn new(entity: Entity, targets: Vec<Target<P>>) -> Self {
        Self { entity, targets }
    }
}

// ---------------------------------------------------------------------------
// SubEffectOf / SubEffects
// ---------------------------------------------------------------------------

/// Relationship target: collection of child effects. Placed on parent effect entities.
#[derive(Component, Default, Debug, PartialEq, Eq)]
#[relationship_target(relationship = SubEffectOf, linked_spawn)]
pub struct SubEffects(Vec<Entity>);

impl<'a> IntoIterator for &'a SubEffects {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = std::slice::Iter<'a, Entity>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl SubEffects {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

/// Relationship component on a child effect pointing to its parent.
#[derive(Component, Clone, PartialEq, Eq, Debug)]
#[relationship(relationship_target = SubEffects)]
pub struct SubEffectOf(#[entities] pub Entity);

impl FromWorld for SubEffectOf {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        SubEffectOf(Entity::PLACEHOLDER)
    }
}
