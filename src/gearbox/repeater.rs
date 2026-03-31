use std::time::Duration;

use bevy::prelude::*;
use bevy_gearbox::prelude::*;

use crate::events::{OnRepeat, PosBound};
use crate::invoker::InvokedBy;
use crate::target::Target as DieselTarget;

// ---------------------------------------------------------------------------
// Repeater component
// ---------------------------------------------------------------------------

/// Counter-driven repetition. Place on a superstate whose substates should
/// be re-entered on each cycle.
///
/// Internally the repeater creates an Idle → Apply cycle driven by
/// [`OnRepeat`]. Each cycle, the repeater system writes `OnRepeat` which
/// fires the `MessageEdge`, producing `GoOffOrigin` via the normal
/// `SideEffect` pipeline. When the counter reaches 0, [`OnComplete`] is
/// written instead so the user can transition away.
///
/// The counter resets when the Repeater gains `Active` (fresh entry from parent).
#[derive(Component, Clone, Debug, Reflect, Default)]
#[reflect(Component, Default)]
pub struct Repeater {
    pub remaining: u32,
    pub initial: u32,
}

impl Repeater {
    pub fn new(count: u32) -> Self {
        Self {
            remaining: count,
            initial: count,
        }
    }
}

// ---------------------------------------------------------------------------
// OnComplete message
// ---------------------------------------------------------------------------

/// Written when a repeater's counter reaches zero.
#[derive(Message, Debug, Clone, Reflect)]
pub struct OnComplete {
    pub entity: Entity,
}

impl GearboxMessage for OnComplete {
    type Validator = AcceptAll;
    fn machine(&self) -> Entity {
        self.entity
    }
}

impl OnComplete {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

// ---------------------------------------------------------------------------
// Repeater system
// ---------------------------------------------------------------------------

/// Repeater lifecycle system. Uses `Changed<Active>` with `Ref` to
/// distinguish initial entry from re-entry via the Apply→Repeater bounce:
///
/// - `is_added()` (initial entry): reset counter, write first [`OnRepeat`]
/// - `!is_added()` (re-entry): decrement, write [`OnRepeat`] if remaining > 0,
///   else [`OnComplete`]
pub fn repeater_tick<P: PosBound>(
    q_changed: Query<(Entity, &Active, Ref<Active>), (Changed<Active>, With<Repeater>)>,
    q_substate_of: Query<&SubstateOf>,
    mut q_repeater: Query<&mut Repeater>,
    mut writer_repeat: MessageWriter<OnRepeat<P>>,
    mut writer_complete: MessageWriter<OnComplete>,
) {
    for (entity, active, active_ref) in &q_changed {
        let Ok(mut repeater) = q_repeater.get_mut(entity) else {
            continue;
        };

        let root = q_substate_of.root_ancestor(entity);
        let target = DieselTarget::entity(root, P::default());

        if active_ref.is_added() {
            // Initial entry — reset counter, decrement, fire first tick
            repeater.remaining = repeater.initial - 1;
            writer_repeat.write(OnRepeat::new(active.machine, target));
        } else if repeater.remaining > 0 {
            // Re-entry via Apply→Repeater bounce — decrement and fire
            repeater.remaining -= 1;
            writer_repeat.write(OnRepeat::new(active.machine, target));
        } else {
            // Exhausted
            writer_complete.write(OnComplete::new(active.machine));
        }
    }
}
