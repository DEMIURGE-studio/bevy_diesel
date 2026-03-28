use std::fmt::Debug;
use std::time::{Duration, Instant};

use bevy::prelude::*;

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

pub fn queue_despawn_observer<P: Clone + Copy + Send + Sync + Default + Debug + 'static>(
    go_off: On<GoOff<P>>,
    query: Query<&QueueDespawn>,
    mut commands: Commands,
) {
    let trigger_entity = go_off.entity;
    let Ok(_) = query.get(trigger_entity) else {
        return;
    };

    for target in go_off.targets.iter() {
        if let Some(target_entity) = target.entity {
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

pub struct DieselDespawnPlugin<P: Clone + Copy + Send + Sync + Default + Debug + 'static> {
    _marker: std::marker::PhantomData<P>,
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> Default for DieselDespawnPlugin<P> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> Plugin for DieselDespawnPlugin<P> {
    fn build(&self, app: &mut App) {
        app.add_observer(queue_despawn_observer::<P>)
            .add_systems(PostUpdate, despawn_queue_system);
    }
}
