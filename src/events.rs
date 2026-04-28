//! Position-typed events for the ability pipeline.
//! Backends alias these with their concrete position type (e.g. `OnRepeat<Vec3>`).

use std::fmt::Debug;

use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy_gearbox::{AcceptAll, BlockedEdges, GearboxMessage, Matched};

use crate::effect::{GoOffOrigin, MessageScope};
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
// GoOffOrigin side-effect system
// ---------------------------------------------------------------------------

/// Trait for diesel messages that carry a `Target<P>` payload.
pub trait HasDieselTarget<P: PosBound>: GearboxMessage {
    fn diesel_target(&self) -> Target<P>;
}

impl<P: PosBound> HasDieselTarget<P> for OnRepeat<P> {
    fn diesel_target(&self) -> Target<P> { self.target }
}
impl<P: PosBound> HasDieselTarget<P> for StartInvoke<P> {
    fn diesel_target(&self) -> Target<P> { self.target }
}
impl<P: PosBound> HasDieselTarget<P> for StopInvoke<P> {
    fn diesel_target(&self) -> Target<P> { self.target }
}

impl<P: PosBound> MessageScope for OnRepeat<P> {}
impl<P: PosBound> MessageScope for StartInvoke<P> {}
impl<P: PosBound> MessageScope for StopInvoke<P> {}

/// Reads surviving `Matched<M>` and writes `GoOffOrigin<P>` with the
/// message's `MessageScope` payload attached.
pub fn go_off_side_effect<M, P>(
    mut reader: MessageReader<Matched<M>>,
    blocked: Res<BlockedEdges>,
    mut writer: MessageWriter<GoOffOrigin<P>>,
)
where
    M: HasDieselTarget<P> + GearboxMessage + MessageScope,
    P: PosBound,
{
    for matched in reader.read() {
        if blocked.is_blocked(matched.edge) { continue; }
        writer.write(GoOffOrigin::with_scope(
            matched.target,
            matched.message.diesel_target(),
            matched.message.scope(),
        ));
    }
}
