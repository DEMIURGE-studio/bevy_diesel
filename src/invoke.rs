use bevy::prelude::*;
use bevy_gearbox::SimpleTransition;

/// Ability marker. Requires `InvokeStatus`.
#[derive(Component, Default, Reflect)]
#[require(InvokeStatus)]
pub struct Ability;

/// Set to `TryInvoke` from input/AI to trigger an ability.
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

/// Fired when an ability invocation completes.
#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
pub struct InvocationComplete {
    #[event_target]
    pub target: Entity,
}

/// Re-triggers invocation on state entry if still held (`TryInvoke`).
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
