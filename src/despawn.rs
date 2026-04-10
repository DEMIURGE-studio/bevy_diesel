use std::fmt::Debug;
use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::diagnostics::diesel_debug;
use crate::effect::GoOff;

#[derive(Component, Default, Clone, Copy)]
pub struct QueueDespawn;

#[derive(Component, Reflect, Clone)]
pub struct DelayedDespawn(pub Instant);

impl DelayedDespawn {
    pub fn now() -> Self {
        Self(Instant::now())
    }

    pub fn after(delay: f32) -> Self {
        Self(Instant::now() + Duration::from_secs_f32(delay))
    }
}

pub fn queue_despawn_system<P: Clone + Copy + Send + Sync + Default + Debug + 'static>(
    mut reader: MessageReader<GoOff<P>>,
    query: Query<&QueueDespawn>,
    mut commands: Commands,
) {
    for go_off in reader.read() {
        let trigger_entity = go_off.entity;
        let Ok(_) = query.get(trigger_entity) else {
            diesel_debug!(
                "[bevy_diesel] queue_despawn_system: GoOff for {:?} but no QueueDespawn, skipping",
                trigger_entity,
            );
            continue;
        };

        if let Some(target_entity) = go_off.target.entity {
            commands.entity(target_entity).insert(DelayedDespawn::now());
        }
    }
}

pub fn despawn_queue_system(
    query: Query<(Entity, &DelayedDespawn)>,
    mut commands: Commands,
) {
    for (entity, queue_despawn) in query.iter() {
        if queue_despawn.0.elapsed().as_secs_f32() > 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}
