use std::fmt::Debug;

use bevy::prelude::*;
use bevy_gearbox::prelude::*;

use crate::backend::SpatialBackend;
use crate::invoker::InvokedBy;
use crate::target::{InvokerTarget, Target};

// ---------------------------------------------------------------------------
// GoOffOrigin<P> — root trigger, consumed by propagate_system
// ---------------------------------------------------------------------------

/// Written by SideEffect impls when a transition message fires. The
/// propagation system walks the SubEffects tree starting at `entity` and
/// writes a [`GoOff`] for every descendant effect.
#[derive(Message, Clone)]
pub struct GoOffOrigin<P: Clone + Copy + Send + Sync + Default + Debug + 'static> {
    pub entity: Entity,
    pub target: Target<P>,
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> GoOffOrigin<P> {
    pub fn new(entity: Entity, target: Target<P>) -> Self {
        Self { entity, target }
    }
}

// ---------------------------------------------------------------------------
// GoOff<P> — resolved per-effect message, consumed by leaf systems
// ---------------------------------------------------------------------------

/// A resolved "go off" for a single effect entity. Written by
/// [`propagate_system`](crate::pipeline::propagate_system) after walking the
/// SubEffects tree. Leaf systems (print, spawn, despawn, modifiers, etc.)
/// read this.
#[derive(Message, Clone)]
pub struct GoOff<P: Clone + Copy + Send + Sync + Default + Debug + 'static> {
    pub entity: Entity,
    pub target: Target<P>,
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> GoOff<P> {
    pub fn new(entity: Entity, target: Target<P>) -> Self {
        Self { entity, target }
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

/// Configures automatic `GoOffOrigin` emission when this state gains `Active`.
/// Use this for states that enter without a message edge (e.g. via `InitialState`
/// or `AlwaysEdge`) but still need the diesel effect pipeline to fire.
///
/// The target is resolved from the root invoker's `InvokerTarget`.
#[derive(Component, Default)]
pub struct GoOffConfig;

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

// ---------------------------------------------------------------------------
// GoOffConfig system: emit GoOffOrigin on state entry
// ---------------------------------------------------------------------------

/// Emits `GoOffOrigin` for states with `GoOffConfig` when they gain `Active`.
/// Resolves the target from the root invoker's `InvokerTarget`.
pub fn go_off_on_entry<B: SpatialBackend>(
    q_new: Query<Entity, (Added<Active>, With<GoOffConfig>)>,
    q_invoker: Query<&InvokedBy>,
    q_invoker_target: Query<&InvokerTarget<B::Pos>>,
    mut writer: MessageWriter<GoOffOrigin<B::Pos>>,
) {
    for entity in &q_new {
        let invoker = q_invoker.root_ancestor(entity);
        let target = q_invoker_target
            .get(invoker)
            .map(|it| (*it).into())
            .unwrap_or_default();
        writer.write(GoOffOrigin::new(entity, target));
    }
}
