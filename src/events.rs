//! Position-typed events for the ability pipeline.
//! Backends alias these with their concrete position type (e.g. `OnRepeat<Vec3>`).

use std::fmt::Debug;

use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy_gearbox::{AcceptAll, GearboxMessage, Matched, SideEffect};

use crate::effect::GoOffOrigin;
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
#[derive(Message, Clone, Debug, Reflect)]
pub struct OnRepeat<P: PosBound> {
    pub entity: Entity,
    pub target: Target<P>,
}

impl<P: PosBound> GearboxMessage for OnRepeat<P> {
    type Validator = AcceptAll;
    fn target(&self) -> Entity { self.entity }
}

impl<P: PosBound> OnRepeat<P> {
    pub fn new(entity: Entity, target: Target<P>) -> Self {
        Self { entity, target }
    }
}

// ---------------------------------------------------------------------------
// StartInvoke<P> - trigger an ability
// ---------------------------------------------------------------------------

/// Triggers an ability invocation with the invoker's current targets.
#[derive(Message, Clone, Debug, Reflect)]
pub struct StartInvoke<P: PosBound> {
    pub entity: Entity,
    pub target: Target<P>,
}

impl<P: PosBound> GearboxMessage for StartInvoke<P> {
    type Validator = AcceptAll;
    fn target(&self) -> Entity { self.entity }
}

impl<P: PosBound> StartInvoke<P> {
    pub fn new(entity: Entity, target: Target<P>) -> Self {
        Self { entity, target }
    }
}

// ---------------------------------------------------------------------------
// StopInvoke<P> - stop a channeled/held ability
// ---------------------------------------------------------------------------

/// Stops a held or channeled ability invocation.
#[derive(Message, Clone, Debug, Reflect)]
pub struct StopInvoke<P: PosBound> {
    pub entity: Entity,
    pub target: Target<P>,
}

impl<P: PosBound> GearboxMessage for StopInvoke<P> {
    type Validator = AcceptAll;
    fn target(&self) -> Entity { self.entity }
}

impl<P: PosBound> StopInvoke<P> {
    pub fn new(entity: Entity, target: Target<P>) -> Self {
        Self { entity, target }
    }
}

// ---------------------------------------------------------------------------
// CollidedEntity<P> - collision event carrying entity targets
// ---------------------------------------------------------------------------

/// Collision with an entity target.
#[derive(Message, Clone, Debug, Reflect)]
pub struct CollidedEntity<P: PosBound> {
    pub entity: Entity,
    pub target: Target<P>,
}

impl<P: PosBound> GearboxMessage for CollidedEntity<P> {
    type Validator = AcceptAll;
    fn target(&self) -> Entity { self.entity }
}

impl<P: PosBound> CollidedEntity<P> {
    pub fn new(entity: Entity, target: Target<P>) -> Self {
        Self { entity, target }
    }
}

// ---------------------------------------------------------------------------
// CollidedPosition<P> - collision event carrying contact position
// ---------------------------------------------------------------------------

/// Collision with a contact point position.
#[derive(Message, Clone, Debug, Reflect)]
pub struct CollidedPosition<P: PosBound> {
    pub entity: Entity,
    pub target: Target<P>,
}

impl<P: PosBound> GearboxMessage for CollidedPosition<P> {
    type Validator = AcceptAll;
    fn target(&self) -> Entity { self.entity }
}

impl<P: PosBound> CollidedPosition<P> {
    pub fn new(entity: Entity, target: Target<P>) -> Self {
        Self { entity, target }
    }
}

// ---------------------------------------------------------------------------
// SideEffect impls: all diesel transition messages produce GoOffOrigin<P>
// ---------------------------------------------------------------------------

macro_rules! impl_go_off_side_effect {
    ($($Msg:ident),*) => {$(
        impl<P: PosBound> SideEffect<$Msg<P>> for GoOffOrigin<P> {
            fn produce(matched: &Matched<$Msg<P>>) -> Option<Self> {
                Some(GoOffOrigin::new(matched.target, matched.message.target))
            }
        }
    )*};
}

impl_go_off_side_effect!(OnRepeat, StartInvoke, StopInvoke, CollidedEntity, CollidedPosition);
