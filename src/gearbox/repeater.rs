use bevy::prelude::*;
use bevy_gearbox::prelude::*;
use bevy_gauge::AttributeComponent;

use crate::events::{OnRepeat, PosBound};
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
/// `SideEffect` pipeline. When the counter reaches 0, [`Done`] is
/// written targeting the parent so the parent can transition away.
///
/// The counter resets when the Repeater gains `Active` (fresh entry from parent).
#[derive(Component, Clone, Debug, Reflect, Default, AttributeComponent)]
#[reflect(Component, Default)]
pub struct Repeater {
    #[init_from("RepeatCount")]
    pub remaining: u32,
    #[read("RepeatCount")]
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
// Repeater system
// ---------------------------------------------------------------------------

/// Repeater lifecycle system. Uses `Changed<Active>` with `Ref` to
/// distinguish initial entry from re-entry via the Apply→Repeater bounce:
///
/// - `is_added()` (initial entry): reset counter, write first [`OnRepeat`]
/// - `!is_added()` (re-entry): decrement, write [`OnRepeat`] if remaining > 0,
///   else [`Done`] targeting the parent
pub fn repeater_tick<P: PosBound>(
    q_changed: Query<(Entity, &Active, Ref<Active>), (Changed<Active>, With<Repeater>)>,
    q_substate_of: Query<&SubstateOf>,
    mut q_repeater: Query<&mut Repeater>,
    mut writer_repeat: MessageWriter<OnRepeat<P>>,
    mut writer_done: MessageWriter<Done>,
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
            writer_repeat.write(OnRepeat::new(entity, target));
        } else if repeater.remaining > 0 {
            // Re-entry via Apply→Repeater bounce — decrement and fire
            repeater.remaining -= 1;
            writer_repeat.write(OnRepeat::new(entity, target));
        } else {
            // Exhausted — reset for next cycle and emit Done to parent
            repeater.remaining = repeater.initial;
            let parent = q_substate_of
                .get(entity)
                .map(|s| s.0)
                .unwrap_or(entity);
            writer_done.write(Done::new(parent));
        }
    }
}
