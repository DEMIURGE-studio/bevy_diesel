use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::prelude::*;

use crate::backend::SpatialBackend;

// ---------------------------------------------------------------------------
// InvokerTarget<P>
// ---------------------------------------------------------------------------

/// The invoker's current aim target. Placed on character/AI entities.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct InvokerTarget<P: Clone + Copy + Send + Sync + Default + Debug + 'static> {
    pub entity: Option<Entity>,
    pub position: P,
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> InvokerTarget<P> {
    pub fn entity(entity: Entity, position: P) -> Self {
        Self {
            entity: Some(entity),
            position,
        }
    }

    pub fn position(position: P) -> Self {
        Self {
            entity: None,
            position,
        }
    }
}

// ---------------------------------------------------------------------------
// Target<P>
// ---------------------------------------------------------------------------

/// A resolved target: optionally an entity, always a position.
#[derive(Component, Clone, Copy, Debug)]
pub struct Target<P: Clone + Copy + Send + Sync + Default + Debug + 'static> {
    pub entity: Option<Entity>,
    pub position: P,
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> Default for Target<P> {
    fn default() -> Self {
        Self {
            entity: None,
            position: P::default(),
        }
    }
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> Target<P> {
    /// Create a target referencing a specific entity at a position.
    pub fn entity(entity: Entity, position: P) -> Self {
        Self {
            entity: Some(entity),
            position,
        }
    }

    /// Create a position-only target with no entity reference.
    pub fn position(position: P) -> Self {
        Self {
            entity: None,
            position,
        }
    }
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> From<InvokerTarget<P>>
    for Target<P>
{
    fn from(it: InvokerTarget<P>) -> Self {
        Target {
            entity: it.entity,
            position: it.position,
        }
    }
}

// ---------------------------------------------------------------------------
// TargetType<P>
// ---------------------------------------------------------------------------

/// Determines which entity/position is used as the base for the pipeline.
#[derive(Clone, Debug)]
pub enum TargetType<P: Clone + Copy + Send + Sync + Default + Debug + 'static> {
    /// The entity that invoked the ability.
    Invoker,
    /// The root entity of the ability hierarchy (via ChildOf chain).
    Root,
    /// The invoker's current target.
    InvokerTarget,
    /// The spawn position (passed as context).
    Spawn,
    /// The incoming target from the parent effect's GoOff event.
    Passed,
    /// A fixed position.
    Position(P),
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> Default for TargetType<P> {
    fn default() -> Self {
        Self::Passed
    }
}

// ---------------------------------------------------------------------------
// TargetGenerator<B>
// ---------------------------------------------------------------------------

/// Targeting pipeline config: target type, offset, gatherer, filter.
/// Plain data. See `TargetMutator<B>` for the Component wrapper.
#[derive(Clone, Debug)]
pub struct TargetGenerator<B: SpatialBackend> {
    pub target_type: TargetType<B::Pos>,
    pub offset: B::Offset,
    /// `None` = identity gather (return resolved target as-is).
    /// `Some` = delegate to backend gather closure.
    pub gatherer: Option<B::Gatherer>,
    pub filter: B::Filter,
}

impl<B: SpatialBackend> Default for TargetGenerator<B> {
    fn default() -> Self {
        Self {
            target_type: TargetType::Passed,
            offset: B::Offset::default(),
            gatherer: None,
            filter: B::Filter::default(),
        }
    }
}

impl<B: SpatialBackend> TargetGenerator<B> {
    pub fn at_invoker() -> Self {
        Self {
            target_type: TargetType::Invoker,
            ..Default::default()
        }
    }

    pub fn at_invoker_target() -> Self {
        Self {
            target_type: TargetType::InvokerTarget,
            ..Default::default()
        }
    }

    pub fn at_spawn() -> Self {
        Self {
            target_type: TargetType::Spawn,
            ..Default::default()
        }
    }

    pub fn at_root() -> Self {
        Self {
            target_type: TargetType::Root,
            ..Default::default()
        }
    }

    pub fn at_passed() -> Self {
        Self {
            target_type: TargetType::Passed,
            ..Default::default()
        }
    }

    pub fn at_position(position: B::Pos) -> Self {
        Self {
            target_type: TargetType::Position(position),
            ..Default::default()
        }
    }

    pub fn with_gatherer(mut self, gatherer: B::Gatherer) -> Self {
        self.gatherer = Some(gatherer);
        self
    }

    pub fn with_offset(mut self, offset: B::Offset) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_filter(mut self, filter: B::Filter) -> Self {
        self.filter = filter;
        self
    }
}

// ---------------------------------------------------------------------------
// TargetMutator<B>
// ---------------------------------------------------------------------------

/// Component wrapper around `TargetGenerator<B>` for effect entities.
#[derive(Component, Clone, Debug)]
pub struct TargetMutator<B: SpatialBackend> {
    pub generator: TargetGenerator<B>,
    #[allow(dead_code)]
    _phantom: PhantomData<B>,
}

impl<B: SpatialBackend> TargetMutator<B> {
    fn new(target_type: TargetType<B::Pos>) -> Self {
        Self {
            generator: TargetGenerator {
                target_type,
                ..Default::default()
            },
            _phantom: PhantomData,
        }
    }

    // -- Constructors: set TargetType, gatherer defaults to None (identity) --

    /// Target the invoker entity directly.
    pub fn invoker() -> Self {
        Self::new(TargetType::Invoker)
    }

    /// Target the invoker's current target.
    pub fn invoker_target() -> Self {
        Self::new(TargetType::InvokerTarget)
    }

    /// Target the root entity.
    pub fn root() -> Self {
        Self::new(TargetType::Root)
    }

    /// Target the spawn position.
    pub fn spawn() -> Self {
        Self::new(TargetType::Spawn)
    }

    /// Target the passed target from the parent effect.
    pub fn passed() -> Self {
        Self::new(TargetType::Passed)
    }

    /// Target a fixed position.
    pub fn at_position(position: B::Pos) -> Self {
        Self::new(TargetType::Position(position))
    }

    // -- Builder methods --

    pub fn with_offset(mut self, offset: B::Offset) -> Self {
        self.generator.offset = offset;
        self
    }

    /// Set the gatherer (`None` = identity).
    pub fn with_gatherer(mut self, gatherer: B::Gatherer) -> Self {
        self.generator.gatherer = Some(gatherer);
        self
    }

    pub fn with_filter(mut self, filter: B::Filter) -> Self {
        self.generator.filter = filter;
        self
    }
}
