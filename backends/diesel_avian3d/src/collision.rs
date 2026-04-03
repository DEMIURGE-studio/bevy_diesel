use std::fmt::Debug;

use avian3d::prelude::{CollisionStart, Collisions, Position};
use bevy::prelude::*;

use bevy_diesel::prelude::*;
use bevy_diesel::bevy_gearbox::{Matched, SideEffect};
use bevy_diesel::effect::GoOffOrigin;
use bevy_diesel::target::Target as DieselTarget;

// ---------------------------------------------------------------------------
// Collision event types (concrete Vec3)
// ---------------------------------------------------------------------------

/// Collision with an entity target.
#[derive(Message, Clone, Debug, Reflect)]
pub struct CollidedEntity {
    pub entity: Entity,
    pub target: DieselTarget<Vec3>,
}

impl GearboxMessage for CollidedEntity {
    type Validator = AcceptAll;
    fn target(&self) -> Entity { self.entity }
}

impl CollidedEntity {
    pub fn new(entity: Entity, target: DieselTarget<Vec3>) -> Self {
        Self { entity, target }
    }
}

/// Collision with a contact point position.
#[derive(Message, Clone, Debug, Reflect)]
pub struct CollidedPosition {
    pub entity: Entity,
    pub target: DieselTarget<Vec3>,
}

impl GearboxMessage for CollidedPosition {
    type Validator = AcceptAll;
    fn target(&self) -> Entity { self.entity }
}

impl CollidedPosition {
    pub fn new(entity: Entity, target: DieselTarget<Vec3>) -> Self {
        Self { entity, target }
    }
}

// SideEffect impls: collision messages produce GoOffOrigin<Vec3>

impl SideEffect<CollidedEntity> for GoOffOrigin<Vec3> {
    fn produce(matched: &Matched<CollidedEntity>) -> Option<Self> {
        Some(GoOffOrigin::new(matched.target, matched.message.target))
    }
}

impl SideEffect<CollidedPosition> for GoOffOrigin<Vec3> {
    fn produce(matched: &Matched<CollidedPosition>) -> Option<Self> {
        Some(GoOffOrigin::new(matched.target, matched.message.target))
    }
}

// ---------------------------------------------------------------------------
// CollisionFilter trait + Collides marker
// ---------------------------------------------------------------------------

/// Determines whether an ability can affect a target entity.
///
/// `Self` goes on the ability entity, `Self::Lookup` is queried on invoker/target.
///
/// ```ignore
/// impl CollisionFilter for Faction {
///     type Lookup = Alliance;
///     fn can_target(&self, invoker: Option<&Alliance>, target: Option<&Alliance>) -> bool {
///         match (self, invoker, target) {
///             (Faction::Enemies, Some(i), Some(t)) => i.0 != t.0,
///             _ => true,
///         }
///     }
/// }
/// ```
pub trait CollisionFilter: Component + Clone + Debug + Send + Sync + 'static {
    /// Component queried on invoker and target entities.
    type Lookup: Component;

    /// Return `true` if the ability should affect this target.
    fn can_target(
        &self,
        invoker_data: Option<&Self::Lookup>,
        target_data: Option<&Self::Lookup>,
    ) -> bool;
}

/// Marker: every collision fires an event, no filtering.
#[derive(Component, Clone, Debug, Default)]
pub struct Collides;

// ---------------------------------------------------------------------------
// Unfiltered collision system - fires for any entity with `Collides` marker
// ---------------------------------------------------------------------------

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Update, unfiltered_collision_system);
}

fn unfiltered_collision_system(
    mut collision_events: MessageReader<CollisionStart>,
    q_collides: Query<(), With<Collides>>,
    q_invoker: Query<&InvokedBy>,
    q_position: Query<&Position>,
    collisions: Collisions,
    mut entity_writer: MessageWriter<CollidedEntity>,
    mut position_writer: MessageWriter<CollidedPosition>,
) {
    for CollisionStart { collider1, collider2, .. } in collision_events.read() {
        emit_entity_if(&q_collides, &q_invoker, &q_position, &mut entity_writer, *collider1, *collider2);
        emit_entity_if(&q_collides, &q_invoker, &q_position, &mut entity_writer, *collider2, *collider1);

        if let Some(contacts) = collisions.get(*collider1, *collider2) {
            if let Some(contact) = contacts.find_deepest_contact() {
                let position = contact.point;
                emit_position_if(&q_collides, &mut position_writer, position, *collider1, *collider2);
                emit_position_if(&q_collides, &mut position_writer, position, *collider2, *collider1);
            }
        }
    }
}

fn emit_entity_if(
    q_collides: &Query<(), With<Collides>>,
    q_invoker: &Query<&InvokedBy>,
    q_position: &Query<&Position>,
    writer: &mut MessageWriter<CollidedEntity>,
    ability: Entity,
    target: Entity,
) {
    if q_collides.get(ability).is_err() {
        return;
    }
    let invoker = q_invoker.root_ancestor(ability);
    if target == invoker {
        return;
    }
    let collision_pos = q_position.get(ability).ok().map(|p| p.0)
        .or_else(|| q_position.get(target).ok().map(|p| p.0))
        .unwrap_or(Vec3::ZERO);
    writer.write(CollidedEntity::new(ability, Target::entity(target, collision_pos)));
}

fn emit_position_if(
    q_collides: &Query<(), With<Collides>>,
    writer: &mut MessageWriter<CollidedPosition>,
    position: Vec3,
    ability: Entity,
    target: Entity,
) {
    if q_collides.get(ability).is_err() {
        return;
    }
    writer.write(CollidedPosition::new(ability, Target::entity(target, position)));
}

// ---------------------------------------------------------------------------
// Filtered collision system - generic over CollisionFilter
// ---------------------------------------------------------------------------

/// Adds filtered collision handling for a `CollisionFilter` implementation.
pub struct CollisionFilterPlugin<F: CollisionFilter> {
    _marker: std::marker::PhantomData<F>,
}

impl<F: CollisionFilter> Default for CollisionFilterPlugin<F> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<F: CollisionFilter> Plugin for CollisionFilterPlugin<F> {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, filtered_collision_system::<F>);
    }
}

fn filtered_collision_system<F: CollisionFilter>(
    mut collision_events: MessageReader<CollisionStart>,
    q_filter: Query<&F>,
    q_lookup: Query<&F::Lookup>,
    q_invoker: Query<&InvokedBy>,
    q_position: Query<&Position>,
    collisions: Collisions,
    mut entity_writer: MessageWriter<CollidedEntity>,
    mut position_writer: MessageWriter<CollidedPosition>,
) {
    for CollisionStart { collider1, collider2, .. } in collision_events.read() {
        emit_entity_filtered(&q_filter, &q_lookup, &q_invoker, &q_position, &mut entity_writer, *collider1, *collider2);
        emit_entity_filtered(&q_filter, &q_lookup, &q_invoker, &q_position, &mut entity_writer, *collider2, *collider1);

        if let Some(contacts) = collisions.get(*collider1, *collider2) {
            if let Some(contact) = contacts.find_deepest_contact() {
                let position = contact.point;
                emit_position_filtered(&q_filter, &q_lookup, &q_invoker, &mut position_writer, position, *collider1, *collider2);
                emit_position_filtered(&q_filter, &q_lookup, &q_invoker, &mut position_writer, position, *collider2, *collider1);
            }
        }
    }
}

fn can_target_filtered<F: CollisionFilter>(
    q_filter: &Query<&F>,
    q_lookup: &Query<&F::Lookup>,
    q_invoker: &Query<&InvokedBy>,
    ability: Entity,
    target: Entity,
) -> bool {
    let Ok(filter) = q_filter.get(ability) else {
        return false;
    };
    let invoker = q_invoker.root_ancestor(ability);
    if target == invoker {
        return false;
    }
    let invoker_data = q_lookup.get(invoker).ok();
    let target_data = q_lookup.get(target).ok();
    filter.can_target(invoker_data, target_data)
}

fn emit_entity_filtered<F: CollisionFilter>(
    q_filter: &Query<&F>,
    q_lookup: &Query<&F::Lookup>,
    q_invoker: &Query<&InvokedBy>,
    q_position: &Query<&Position>,
    writer: &mut MessageWriter<CollidedEntity>,
    ability: Entity,
    target: Entity,
) {
    if can_target_filtered(q_filter, q_lookup, q_invoker, ability, target) {
        let collision_pos = q_position.get(ability).ok().map(|p| p.0)
            .or_else(|| q_position.get(target).ok().map(|p| p.0))
            .unwrap_or(Vec3::ZERO);
        writer.write(CollidedEntity::new(ability, Target::entity(target, collision_pos)));
    }
}

fn emit_position_filtered<F: CollisionFilter>(
    q_filter: &Query<&F>,
    q_lookup: &Query<&F::Lookup>,
    q_invoker: &Query<&InvokedBy>,
    writer: &mut MessageWriter<CollidedPosition>,
    position: Vec3,
    ability: Entity,
    target: Entity,
) {
    if can_target_filtered(q_filter, q_lookup, q_invoker, ability, target) {
        writer.write(CollidedPosition::new(ability, Target::entity(target, position)));
    }
}
