use bevy::prelude::*;
use bevy_gearbox::SimpleTransition;

/// Marker component for ability entities.
/// Automatically requires `InvokeStatus` to be present.
#[derive(Component, Default, Reflect)]
#[require(InvokeStatus)]
pub struct Ability;

/// Tracks whether an ability is being invoked by its owner.
/// Users set this to `TryInvoke` from their input/AI systems;
/// the invocation handler reads it and emits the appropriate events.
#[derive(Component, Clone, Debug, Reflect, PartialEq)]
pub enum InvokeStatus {
    Idle,
    TryInvoke,
}

impl Default for InvokeStatus {
    fn default() -> Self {
        Self::Idle
    }
}

/// Transition event fired when an ability invocation completes.
/// Use as an edge in gearbox state machines to transition back to idle/ready state.
#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
pub struct InvocationComplete {
    #[event_target]
    pub target: Entity,
}

/// Observer helper: on entering a state, re-trigger invocation if the ability
/// is still held (InvokeStatus::TryInvoke). Attach to ready/idle states to
/// allow held-button re-invocation.
pub fn check_should_reinvoke_ability(
    enter_state: On<bevy_gearbox::EnterState>,
    mut q_ability: Query<&mut InvokeStatus>,
) {
    let ability_entity = enter_state.state_machine;
    let Ok(mut invoke_status) = q_ability.get_mut(ability_entity) else {
        return;
    };

    if *invoke_status == InvokeStatus::TryInvoke {
        // Force change detection so downstream Changed<InvokeStatus> queries pick it up
        invoke_status.set_changed();
    }
}
