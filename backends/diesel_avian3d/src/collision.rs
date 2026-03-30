use avian3d::prelude::{CollisionStart, Collisions, Position};
use bevy::prelude::*;

use bevy_diesel::prelude::*;

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
    mut entity_writer: MessageWriter<CollidedEntity<Vec3>>,
    mut position_writer: MessageWriter<CollidedPosition<Vec3>>,
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
    writer: &mut MessageWriter<CollidedEntity<Vec3>>,
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
    writer: &mut MessageWriter<CollidedPosition<Vec3>>,
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
    mut entity_writer: MessageWriter<CollidedEntity<Vec3>>,
    mut position_writer: MessageWriter<CollidedPosition<Vec3>>,
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
    writer: &mut MessageWriter<CollidedEntity<Vec3>>,
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
    writer: &mut MessageWriter<CollidedPosition<Vec3>>,
    position: Vec3,
    ability: Entity,
    target: Entity,
) {
    if can_target_filtered(q_filter, q_lookup, q_invoker, ability, target) {
        writer.write(CollidedPosition::new(ability, Target::entity(target, position)));
    }
}
