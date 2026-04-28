use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_gearbox::prelude::*;

use crate::backend::SpatialBackend;
use crate::diagnostics::diesel_debug;
use crate::invoker::{InvokedBy, resolve_invoker, resolve_root};
use crate::pipeline::generate_targets;
use crate::target::{GatherScope, InvokerTarget, Target, TargetGenerator};

// ---------------------------------------------------------------------------
// MessageScope
// ---------------------------------------------------------------------------

/// Expose a message's payload to downstream effects. Keys use the
/// `@go_off` suffix by convention. Default empty.
pub trait MessageScope {
    fn scope(&self) -> GatherScope {
        GatherScope::new()
    }
}

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
    pub gather: GatherScope,
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> GoOffOrigin<P> {
    pub fn new(entity: Entity, target: Target<P>) -> Self {
        Self {
            entity,
            target,
            gather: GatherScope::new(),
        }
    }

    pub fn with_gather(entity: Entity, target: Target<P>, gather: GatherScope) -> Self {
        Self {
            entity,
            target,
            gather,
        }
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
    pub gather: GatherScope,
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> GoOff<P> {
    pub fn new(entity: Entity, target: Target<P>) -> Self {
        Self {
            entity,
            target,
            gather: GatherScope::new(),
        }
    }

    pub fn with_gather(entity: Entity, target: Target<P>, gather: GatherScope) -> Self {
        Self {
            entity,
            target,
            gather,
        }
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
/// Carries a [`TargetGenerator`] that resolves 1..N targets when the state
/// activates. One `GoOffOrigin` is emitted per resolved target.
///
/// Defaults to `TargetType::InvokerTarget` (the invoker's current target),
/// which matches the typical "ability fires at the thing I'm aiming at"
/// behavior. Use [`GoOffConfig::invoker`] for self-targeting (e.g. a heal)
/// or a custom generator for gathered/filtered targets.
#[derive(Component, Clone, Debug)]
pub struct GoOffConfig<B: SpatialBackend> {
    pub generator: TargetGenerator<B>,
    #[allow(dead_code)]
    _phantom: PhantomData<B>,
}

impl<B: SpatialBackend> Default for GoOffConfig<B> {
    fn default() -> Self {
        Self {
            generator: TargetGenerator::at_invoker_target(),
            _phantom: PhantomData,
        }
    }
}

impl<B: SpatialBackend> GoOffConfig<B> {
    /// Use a custom generator.
    pub fn new(generator: TargetGenerator<B>) -> Self {
        Self {
            generator,
            _phantom: PhantomData,
        }
    }

    /// Target the invoker entity directly (e.g. self-heal).
    pub fn invoker() -> Self {
        Self::new(TargetGenerator::at_invoker())
    }

    /// Target the invoker's current target (default — combat abilities).
    pub fn invoker_target() -> Self {
        Self::new(TargetGenerator::at_invoker_target())
    }

    /// Target the root entity of the ability hierarchy.
    pub fn root() -> Self {
        Self::new(TargetGenerator::at_root())
    }

    /// Target the spawn position.
    pub fn spawn() -> Self {
        Self::new(TargetGenerator::at_spawn())
    }

    /// Target a fixed position.
    pub fn at_position(position: B::Pos) -> Self {
        Self::new(TargetGenerator::at_position(position))
    }

    pub fn with_gatherer(mut self, gatherer: B::Gatherer) -> Self {
        self.generator.gatherer = Some(gatherer);
        self
    }

    pub fn with_offset(mut self, offset: B::Offset) -> Self {
        self.generator.offset = offset;
        self
    }

    pub fn with_filter(mut self, filter: B::Filter) -> Self {
        self.generator.filter = filter;
        self
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

// ---------------------------------------------------------------------------
// GoOffConfig system: emit GoOffOrigin on state entry
// ---------------------------------------------------------------------------

/// Emits `GoOffOrigin` for states with `GoOffConfig` when they gain `Active`.
/// Runs the config's `TargetGenerator` to resolve 1..N targets and emits
/// one `GoOffOrigin` per target.
pub fn go_off_on_entry<B: SpatialBackend>(
    q_new: Query<(Entity, &GoOffConfig<B>), Added<Active>>,
    mut ctx: B::Context<'_, '_>,
    q_invoker: Query<&InvokedBy>,
    q_child_of: Query<&ChildOf>,
    q_invoker_target: Query<&InvokerTarget<B::Pos>>,
    mut writer: MessageWriter<GoOffOrigin<B::Pos>>,
) {
    if q_new.is_empty() {
        return;
    }
    diesel_debug!("[diesel] go_off_on_entry: {} newly-active GoOffConfig entities", q_new.iter().count());
    for (entity, config) in &q_new {
        let invoker = resolve_invoker(&q_invoker, entity);
        let root = resolve_root(&q_child_of, entity);
        let invoker_target: Target<B::Pos> = match q_invoker_target.get(invoker) {
            Ok(it) => Target::from(*it),
            Err(_) => {
                diesel_debug!(
                    "[bevy_diesel] go_off_on_entry: invoker {:?} has no InvokerTarget, defaulting to origin",
                    invoker,
                );
                Target::default()
            }
        };

        // Resolve the generator into a list of targets. The `passed` input
        // is the invoker target by default — only relevant if the generator
        // uses TargetType::Passed.
        let mut targets = generate_targets::<B>(
            &config.generator,
            &mut ctx,
            invoker,
            invoker_target,
            root,
            B::Pos::default(),
            invoker_target,
        );
        targets = B::apply_filter(
            &mut ctx,
            targets,
            &config.generator.filter,
            invoker,
            invoker_target.position,
        );

        diesel_debug!("[diesel] go_off_on_entry: entity={:?} invoker={:?} targets_count={}", entity, invoker, targets.len());
        for (target, gather) in targets {
            diesel_debug!("[diesel]   -> writing GoOffOrigin for {:?} target={:?}", entity, target);
            writer.write(GoOffOrigin::with_gather(entity, target, gather));
        }
    }
}
