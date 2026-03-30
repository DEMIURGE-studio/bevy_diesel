use bevy::prelude::*;
use bevy_gearbox::{AcceptAll, Active, GearboxMessage};

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
pub fn check_should_reinvoke_ability(
    q_newly_active: Query<&Active, Added<Active>>,
    mut q_ability: Query<&mut InvokeStatus>,
) {
    for active in &q_newly_active {
        if let Ok(mut invoke_status) = q_ability.get_mut(active.machine) {
            if *invoke_status == InvokeStatus::TryInvoke {
                invoke_status.set_changed();
            }
        }
    }
}
