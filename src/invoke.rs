use bevy::prelude::*;
use bevy_gearbox::{AcceptAll, GearboxMessage, FrameTransitionLog, Machine, WriteMessageExt};

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
#[derive(Message, Reflect, Clone)]
pub struct InvocationComplete {
    pub target: Entity,
}

impl GearboxMessage for InvocationComplete {
    type Validator = AcceptAll;
    fn machine(&self) -> Entity { self.target }
}

/// Re-triggers invocation on state entry if still held (`TryInvoke`).
/// Replaces the old `On<EnterState>` observer.
pub fn check_should_reinvoke_ability(
    frame_log: Res<FrameTransitionLog>,
    q_machine: Query<Entity, With<Machine>>,
    mut q_ability: Query<&mut InvokeStatus>,
) {
    for (machine, _state) in frame_log.all_entered() {
        // The old observer used enter_state.state_machine to find the ability entity
        if let Ok(mut invoke_status) = q_ability.get_mut(machine) {
            if *invoke_status == InvokeStatus::TryInvoke {
                invoke_status.set_changed();
            }
        }
    }
}
