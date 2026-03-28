//! Position-typed events for the ability pipeline.
//! Backends alias these with their concrete position type (e.g. `OnRepeat<Vec3>`).

use std::fmt::Debug;

use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy_gearbox::transitions::TransitionEvent;

use crate::effect::GoOff;
use crate::gearbox::repeater::Repeatable;
use crate::target::Target;

// ---------------------------------------------------------------------------
// Position type bound alias (for readability)
// ---------------------------------------------------------------------------

/// Bound alias for position types.
pub trait PosBound: Clone + Copy + Send + Sync + Default + Debug + TypePath + Reflect + 'static {}
impl<T: Clone + Copy + Send + Sync + Default + Debug + TypePath + Reflect + 'static> PosBound for T {}

// ---------------------------------------------------------------------------
// OnRepeat<P> - emitted by Repeater on each tick
// ---------------------------------------------------------------------------

/// Emitted by the repeater on each iteration.
#[derive(EntityEvent, Clone, Debug, Reflect)]
pub struct OnRepeat<P: PosBound> {
    #[event_target]
    pub entity: Entity,
    #[reflect(ignore)]
    pub targets: Vec<Target<P>>,
}

impl<P: PosBound> OnRepeat<P> {
    pub fn new(entity: Entity, targets: Vec<Target<P>>) -> Self {
        Self { entity, targets }
    }
}

impl<P: PosBound> TransitionEvent for OnRepeat<P> {
    type ExitEvent = bevy_gearbox::NoEvent;
    type EdgeEvent = bevy_gearbox::NoEvent;
    type EntryEvent = GoOff<P>;
    type Validator = bevy_gearbox::AcceptAll;

    fn to_entry_event(
        &self,
        entering: Entity,
        _exiting: Entity,
        _edge: Entity,
    ) -> Option<GoOff<P>> {
        Some(GoOff::new(entering, self.targets.clone()))
    }
}

impl<P: PosBound> Repeatable for OnRepeat<P> {
    fn repeat_tick(entity: Entity) -> Self {
        Self {
            entity,
            targets: Vec::new(),
        }
    }
}

impl<P: PosBound> From<Vec<Target<P>>> for OnRepeat<P> {
    fn from(value: Vec<Target<P>>) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            targets: value,
        }
    }
}

// ---------------------------------------------------------------------------
// StartInvoke<P> - trigger an ability
// ---------------------------------------------------------------------------

/// Triggers an ability invocation with the invoker's current targets.
#[derive(EntityEvent, Clone, Debug, Reflect)]
pub struct StartInvoke<P: PosBound> {
    #[event_target]
    pub entity: Entity,
    #[reflect(ignore)]
    pub targets: Vec<Target<P>>,
}

impl<P: PosBound> StartInvoke<P> {
    pub fn new(entity: Entity, targets: Vec<Target<P>>) -> Self {
        Self { entity, targets }
    }
}

impl<P: PosBound> TransitionEvent for StartInvoke<P> {
    type ExitEvent = bevy_gearbox::NoEvent;
    type EdgeEvent = bevy_gearbox::NoEvent;
    type EntryEvent = GoOff<P>;
    type Validator = bevy_gearbox::AcceptAll;

    fn to_entry_event(
        &self,
        entering: Entity,
        _exiting: Entity,
        _edge: Entity,
    ) -> Option<GoOff<P>> {
        Some(GoOff::new(entering, self.targets.clone()))
    }
}

impl<P: PosBound> From<Vec<Target<P>>> for StartInvoke<P> {
    fn from(value: Vec<Target<P>>) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            targets: value,
        }
    }
}

// ---------------------------------------------------------------------------
// CollidedEntity<P> - collision event carrying entity targets
// ---------------------------------------------------------------------------

/// Collision with an entity target.
#[derive(EntityEvent, Clone, Debug, Reflect)]
pub struct CollidedEntity<P: PosBound> {
    #[event_target]
    pub entity: Entity,
    #[reflect(ignore)]
    pub targets: Vec<Target<P>>,
}

impl<P: PosBound> CollidedEntity<P> {
    pub fn new(entity: Entity, targets: Vec<Target<P>>) -> Self {
        Self { entity, targets }
    }
}

impl<P: PosBound> TransitionEvent for CollidedEntity<P> {
    type ExitEvent = bevy_gearbox::NoEvent;
    type EdgeEvent = bevy_gearbox::NoEvent;
    type EntryEvent = GoOff<P>;
    type Validator = bevy_gearbox::AcceptAll;

    fn to_entry_event(
        &self,
        entering: Entity,
        _exiting: Entity,
        _edge: Entity,
    ) -> Option<GoOff<P>> {
        Some(GoOff::new(entering, self.targets.clone()))
    }
}

impl<P: PosBound> From<Vec<Target<P>>> for CollidedEntity<P> {
    fn from(value: Vec<Target<P>>) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            targets: value,
        }
    }
}

// ---------------------------------------------------------------------------
// CollidedPosition<P> - collision event carrying contact position
// ---------------------------------------------------------------------------

/// Collision with a contact point position.
#[derive(EntityEvent, Clone, Debug, Reflect)]
pub struct CollidedPosition<P: PosBound> {
    #[event_target]
    pub entity: Entity,
    #[reflect(ignore)]
    pub targets: Vec<Target<P>>,
}

impl<P: PosBound> CollidedPosition<P> {
    pub fn new(entity: Entity, targets: Vec<Target<P>>) -> Self {
        Self { entity, targets }
    }
}

impl<P: PosBound> TransitionEvent for CollidedPosition<P> {
    type ExitEvent = bevy_gearbox::NoEvent;
    type EdgeEvent = bevy_gearbox::NoEvent;
    type EntryEvent = GoOff<P>;
    type Validator = bevy_gearbox::AcceptAll;

    fn to_entry_event(
        &self,
        entering: Entity,
        _exiting: Entity,
        _edge: Entity,
    ) -> Option<GoOff<P>> {
        Some(GoOff::new(entering, self.targets.clone()))
    }
}

impl<P: PosBound> From<Vec<Target<P>>> for CollidedPosition<P> {
    fn from(value: Vec<Target<P>>) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            targets: value,
        }
    }
}
