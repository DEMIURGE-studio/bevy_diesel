//! Message-based event propagation.
//!
//! A propagated event is a [`Message`] that carries the entity it is addressed
//! to (via [`PropagatedMessage`]). Diesel forwards it along a per-message-type
//! subscription graph: when a `T` is written for a source entity, a retargeted
//! copy is written for every entity subscribed to that source.
//!
//! [`propagate_message`] runs inside [`GearboxSchedule`] and bumps the gearbox
//! fixpoint loop's work counter for each forwarded copy, so a subscribing state
//! machine consumes the forwarded message the same frame it is emitted. A
//! multi-stage user pipeline (`Attack -> Hit -> Damage -> Killed`) is a chain of
//! `.chain()`-ordered systems, each a distinct message type reading one and
//! writing the next.

use bevy::prelude::*;
use bevy_gearbox::resolve::PendingCount;
use bevy_gearbox::{GearboxPhase, GearboxSchedule};
use std::marker::PhantomData;

// ================= PropagatedMessage =================

/// A [`Message`] that carries the entity it is addressed to, so propagation can
/// re-target a copy for each subscriber.
///
/// ```ignore
/// #[derive(Message, Clone, Reflect)]
/// struct Hit { defender: Entity, amount: f32 }
///
/// impl PropagatedMessage for Hit {
///     fn target(&self) -> Entity { self.defender }
///     fn set_target(&mut self, e: Entity) { self.defender = e; }
/// }
/// ```
pub trait PropagatedMessage: Message + Clone {
    /// The entity this message is addressed to.
    fn target(&self) -> Entity;
    /// Re-address this message to `entity` (used when forwarding to a subscriber).
    fn set_target(&mut self, entity: Entity);
}

// ================= Subscription graph =================

/// Subscribers of an entity's `T` messages. Relationship target of
/// [`PropagationTargetOf`].
#[derive(Component, Debug, PartialEq, Eq, Reflect)]
#[relationship_target(relationship = PropagationTargetOf<T>, linked_spawn)]
#[reflect(Component, FromWorld)]
pub struct PropagationTargets<T: Message> {
    #[entities]
    #[relationship]
    entities: Vec<Entity>,
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T: Message> Default for PropagationTargets<T> {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            _pd: PhantomData,
        }
    }
}

impl<T: Message> PropagationTargets<T> {
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }
}

/// Points a subscriber at the source whose `T` messages it receives.
#[derive(Component, Clone, Debug, Reflect)]
#[relationship(relationship_target = PropagationTargets<T>)]
#[reflect(Component, FromWorld, Default)]
pub struct PropagationTargetOf<T: Message> {
    #[entities]
    #[relationship]
    pub entity: Entity,
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T: Message> Default for PropagationTargetOf<T> {
    fn default() -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            _pd: PhantomData,
        }
    }
}

impl<T: Message> PropagationTargetOf<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            _pd: PhantomData,
        }
    }
}

// ================= Wiring the graph =================

/// Subscribe `target` to `source`'s `T` messages. Trigger it to wire one edge
/// of the subscription graph at runtime.
#[derive(EntityEvent, Clone, Debug, Reflect)]
pub struct RegisterPropagationTarget<T: Message> {
    #[event_target]
    pub target: Entity,
    pub source: Entity,
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T: Message> RegisterPropagationTarget<T> {
    pub fn new(target: Entity, source: Entity) -> Self {
        Self {
            target,
            source,
            _pd: PhantomData,
        }
    }
}

pub fn register_propagation_target<T: Message>(
    e: On<RegisterPropagationTarget<T>>,
    mut commands: Commands,
) {
    commands
        .entity(e.target)
        .insert(PropagationTargetOf::<T>::new(e.source));
}

/// Marker: subscribe this entity to `T` messages emitted on its `ChildOf` root.
/// The marker is consumed once the subscription is wired.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct RegisterPropagationTargetRoot<T: Message> {
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T: Message> Default for RegisterPropagationTargetRoot<T> {
    fn default() -> Self {
        Self { _pd: PhantomData }
    }
}

pub fn register_propagation_target_root<T: Message>(
    q_register: Query<Entity, With<RegisterPropagationTargetRoot<T>>>,
    q_child_of: Query<&ChildOf>,
    mut commands: Commands,
) {
    for entity in q_register.iter() {
        let owner_root = q_child_of.root_ancestor(entity);
        commands
            .entity(entity)
            .insert(PropagationTargetOf::<T>::new(owner_root));
        commands
            .entity(entity)
            .try_remove::<RegisterPropagationTargetRoot<T>>();
    }
}

// ================= Propagation system =================

/// System set holding every `propagate_message::<T>` system, for ordering user
/// systems relative to propagation.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PropagationSet;

/// Forwards `T` to subscribers: for each `T` addressed to a source that has
/// [`PropagationTargets`], write a retargeted copy for every subscriber.
///
/// The copies are written through [`Commands`] (deferred) rather than a
/// `MessageWriter<T>`, because a system cannot both read and write the same
/// message type. A subscriber with no `PropagationTargets<T>` of its own does
/// not re-propagate, so one-level subscriptions terminate naturally.
///
/// Runs inside [`GearboxSchedule`], and bumps [`PendingCount`] for every copy it
/// forwards. That keeps the gearbox fixpoint loop iterating, so a forwarded copy
/// is delivered — and consumed by a subscribing state machine's message edge —
/// within the *same* frame, instead of a frame later.
pub fn propagate_message<T: PropagatedMessage>(
    mut reader: MessageReader<T>,
    q_targets: Query<&PropagationTargets<T>>,
    mut pending: ResMut<PendingCount>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        let Ok(targets) = q_targets.get(msg.target()) else {
            continue;
        };
        for &subscriber in targets.iter() {
            let mut copy = msg.clone();
            copy.set_target(subscriber);
            commands.write_message(copy);
            pending.0 += 1;
        }
    }
}

// ================= Inventory-backed registration =================

pub struct PropagationRegistrar {
    pub register: fn(&mut App),
}

inventory::collect!(PropagationRegistrar);

/// Register the message buffer, subscription graph, and propagation system for
/// message type `T`.
///
/// `propagate_message::<T>` is added to [`GearboxSchedule`] (after the gearbox
/// phases) so it runs inside the fixpoint loop; the subscription-wiring system
/// stays in [`Update`], where its one-shot latency is irrelevant.
///
/// Requires [`GearboxPlugin`](bevy_gearbox::GearboxPlugin) to have been added
/// first (so `GearboxSchedule` and [`PendingCount`] exist) — `DieselCorePlugin`
/// guarantees this.
pub fn register_propagation_for<T: PropagatedMessage + Reflect + TypePath>(app: &mut App) {
    app.add_message::<T>()
        .add_observer(register_propagation_target::<T>)
        .register_type::<PropagationTargets<T>>()
        .register_type::<PropagationTargetOf<T>>()
        .add_systems(Update, register_propagation_target_root::<T>);

    app.configure_sets(
        GearboxSchedule,
        PropagationSet.after(GearboxPhase::SideEffectPhase),
    );
    app.add_systems(
        GearboxSchedule,
        propagate_message::<T>.in_set(PropagationSet),
    );
}

/// Applies all inventory-submitted propagation registrations.
pub fn plugin(app: &mut App) {
    for reg in inventory::iter::<PropagationRegistrar> {
        (reg.register)(app);
    }
}
