use avian3d::prelude::{CollisionStart, Collisions, Position};
use bevy::prelude::*;

use bevy_diesel::prelude::*;

bevy_diesel::go_off!(CollidedEntity, Vec3);
bevy_diesel::go_off!(CollidedPosition, Vec3);

pub fn plugin(app: &mut App) {
    app.add_systems(Update, invoke_on_collision_system);
}

pub fn invoke_on_collision_system(
    mut collision_events: MessageReader<CollisionStart>,
    q_team_filter: Query<&TeamFilter>,
    q_invoker: Query<&InvokedBy>,
    q_team: Query<&Team>,
    q_position: Query<&Position>,
    collisions: Collisions,
    mut commands: Commands,
) {
    for CollisionStart { collider1, collider2, .. } in collision_events.read() {
        handle_entity_collision(&q_team_filter, &q_invoker, &q_team, &q_position, &mut commands, *collider1, *collider2);
        handle_entity_collision(&q_team_filter, &q_invoker, &q_team, &q_position, &mut commands, *collider2, *collider1);

        if let Some(contacts) = collisions.get(*collider1, *collider2) {
            if let Some(contact) = contacts.find_deepest_contact() {
                let position = contact.point;
                handle_position_collision(&q_team_filter, &q_invoker, &q_team, &mut commands, position, *collider1, *collider2);
                handle_position_collision(&q_team_filter, &q_invoker, &q_team, &mut commands, position, *collider2, *collider1);
            }
        }
    }
}

fn can_target(
    q_team_filter: &Query<&TeamFilter>,
    q_invoker: &Query<&InvokedBy>,
    q_team: &Query<&Team>,
    ability: Entity,
    target: Entity,
) -> bool {
    let Ok(team_filter) = q_team_filter.get(ability) else {
        return false;
    };
    let invoker = q_invoker.root_ancestor(ability);
    let Ok(invoker_team) = q_team.get(invoker) else {
        return false;
    };
    match q_team.get(target) {
        Ok(target_team) => match team_filter {
            TeamFilter::Both => true,
            TeamFilter::Allies => invoker_team.0 == target_team.0,
            TeamFilter::Enemies => invoker_team.0 != target_team.0,
            TeamFilter::Specific(id) => target_team.0 == *id,
        },
        Err(_) => true, // no team component — allow
    }
}

fn handle_entity_collision(
    q_team_filter: &Query<&TeamFilter>,
    q_invoker: &Query<&InvokedBy>,
    q_team: &Query<&Team>,
    q_position: &Query<&Position>,
    commands: &mut Commands,
    ability: Entity,
    target: Entity,
) {
    if can_target(q_team_filter, q_invoker, q_team, ability, target) {
        if let Ok(pos) = q_position.get(target) {
            commands.trigger(CollidedEntity::new(ability, vec![Target::entity(target, pos.0)]));
        }
    }
}

fn handle_position_collision(
    q_team_filter: &Query<&TeamFilter>,
    q_invoker: &Query<&InvokedBy>,
    q_team: &Query<&Team>,
    commands: &mut Commands,
    position: Vec3,
    ability: Entity,
    target: Entity,
) {
    if can_target(q_team_filter, q_invoker, q_team, ability, target) {
        commands.trigger(CollidedPosition::new(ability, vec![Target::entity(target, position)]));
    }
}
