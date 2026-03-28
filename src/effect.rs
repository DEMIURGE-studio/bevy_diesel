use std::fmt::Debug;

use bevy::prelude::*;

use crate::target::Target;

// ---------------------------------------------------------------------------
// GoOff<P>
// ---------------------------------------------------------------------------

/// Fires on an effect entity with resolved targets, triggering sub-effect propagation.
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

/// Collection of child effects.
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

/// Points a child effect to its parent.
#[derive(Component, Clone, PartialEq, Eq, Debug)]
#[relationship(relationship_target = SubEffects)]
pub struct SubEffectOf(#[entities] pub Entity);

impl FromWorld for SubEffectOf {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        SubEffectOf(Entity::PLACEHOLDER)
    }
}
