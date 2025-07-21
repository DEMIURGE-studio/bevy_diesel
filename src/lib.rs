#![allow(incomplete_features)]
#![feature(associated_type_defaults)]

use std::time::Duration;

use bevy::{platform::collections::HashSet, prelude::*};

/// A trait that represents the target type used by the ability system.
/// Usually, only one EffectTarget implmentation is used in a project.
pub trait EffectTarget: Clone + Send + Sync + 'static { }

/// Default target implementation.
#[derive(Clone, Component)]
pub struct Target {
    pub entity: Option<Entity>,
    pub position: Vec3,
}

impl EffectTarget for Target { }

/// Cues are events that are triggered against a target, and they propagate
/// to the children of that target. Cues are 
pub trait Cue: Event + Clone {
    type Target: EffectTarget + Send + Sync + 'static + Clone = Target;

    fn from_target(target: Self::Target) -> Self;
    fn get_target(&self) -> &Self::Target;
}

/// An event that is triggered when a cue is triggered against an entity
/// that has the corresponding listener component.
#[derive(Event, Clone)]
pub struct TriggerEffect<T: EffectTarget>(pub T);

pub fn propagate_cue<C, L>(
    trigger: Trigger<C>,
    children_query: Query<(Option<&PropagationBlockers>, &Children)>,
    listener_query: Query<&L>,
    mut commands: Commands,
) where C: Cue + Clone, L: Component {
    let entity = trigger.target();
    let event = trigger.event();

    if let Ok((propagation_blockers, children)) = children_query.get(entity) {
        // If the target is blocked, don't propagate the cue.
        if let Some(propagation_blockers) = propagation_blockers {
            if propagation_blockers.is_blocked() {
                return;
            }
        }

        // If the target has an appropriate listener, trigger the TriggerEffect event against it.
        if listener_query.contains(entity) {
            commands.trigger_targets(TriggerEffect(event.get_target().clone()), entity);
        }

        // If the target has children, trigger the cue against each child.
        for child in children.iter() {
            commands.trigger_targets(event.clone(), child);
        }
    }
}

/// A component that aggregates the different block conditions for an effect.
#[derive(Component)]
pub struct PropagationBlockers(pub HashSet<String>);

impl PropagationBlockers {
    pub fn is_blocked(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn add_blocker(&mut self, blocker: String) {
        self.0.insert(blocker);
    }

    pub fn remove_blocker(&mut self, blocker: String) {
        self.0.remove(&blocker);
    }
}

pub struct SpawnConfig {

}

pub struct TargetConfig {

}

#[derive(Component)]
pub struct Repeater {
    pub interval: Duration,
    pub count: Option<u32>,
    pub duration: Option<Duration>,
}

#[derive(Component, Default)]
pub struct Dormant;

#[derive(Component)]
pub struct Repeating {
    pub elapsed_from_trigger: Duration,
    pub elapsed_from_start: Duration,
    // count?
    // duration?
}

#[derive(Event, Clone)]
pub struct StartRepeatCue<T: EffectTarget>(pub T);

impl<T: EffectTarget> Cue for StartRepeatCue<T> {
    type Target = T;

    fn from_target(target: Self::Target) -> Self {
        StartRepeatCue(target)
    }

    fn get_target(&self) -> &Self::Target {
        &self.0
    }
}

// TODO a derive macro for Cue. #[cue(Target = Target)] or #[cue] for the default target. 

#[derive(Event, Clone)]
pub struct OnRepeatCue<T: EffectTarget>(pub T);

impl<T: EffectTarget> Cue for OnRepeatCue<T> {
    type Target = T;

    fn from_target(target: Self::Target) -> Self {
        OnRepeatCue(target)
    }

    fn get_target(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Event, Clone)]
pub struct StopRepeatCue<T: EffectTarget>(pub T);

impl<T: EffectTarget> Cue for StopRepeatCue<T> {
    type Target = T;

    fn from_target(target: Self::Target) -> Self {
        StopRepeatCue(target)
    }

    fn get_target(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Event, Clone)]
pub struct ExampleCue(pub Target);

impl Cue for ExampleCue {
    fn from_target(target: Self::Target) -> Self {
        todo!()
    }

    fn get_target(&self) -> &Self::Target {
        todo!()
    }
}