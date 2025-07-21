#![allow(incomplete_features)]
#![feature(associated_type_defaults)]

use bevy::{platform::collections::HashSet, prelude::*};

pub trait EffectTarget { }

pub trait Cue: Event + Clone {
    type Target: EffectTarget = Target;

    fn from_target(target: Self::Target) -> Self;
    fn get_target(&self) -> Self::Target;
}

pub fn propagate_cue<C, L>(
    trigger: Trigger<C>,
    children_query: Query<(Option<&PropagationBlockers>, &Children)>,
    listener_query: Query<&L>,
    mut commands: Commands,
) where C: Cue, L: Component {
    let entity = trigger.target();
    let event = trigger.event();

    if let Ok((propagation_blockers, children)) = children_query.get(entity) {
        // If there is a blocker, don't propagate the cue.
        let Some(propagation_blockers) = propagation_blockers else{
            return;
        };

        if propagation_blockers.is_blocked() {
            return;
        }

        for child in children.iter() {
            if listener_query.contains(child) {
                return;
            }

            commands.trigger_targets(event.clone(), child);
        }
    }
}

#[derive(Component)]
pub struct PropagationBlockers(pub HashSet<String>);

impl PropagationBlockers {
    pub fn is_blocked(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn add_blocker(&mut self, blocker: String) {

    }

    pub fn remove_blocker(&mut self, blocker: String) {
        
    }
}

#[derive(Clone)]
pub struct Target {
    pub entity: Option<Entity>,
    pub position: Vec3,
}

impl EffectTarget for Target { }