use bevy::prelude::*;
use bevy_gearbox::prelude::*;

use crate::invoker::InvokedBy;

// ---------------------------------------------------------------------------
// Repeater component
// ---------------------------------------------------------------------------

/// Counter-driven repetition. Place on a state entity.
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
// OnComplete event
// ---------------------------------------------------------------------------

/// Fired when a repeater's counter reaches zero.
#[derive(Message, Debug, Clone, Reflect)]
pub struct OnComplete {
    pub entity: Entity,
}

impl GearboxMessage for OnComplete {
    type Validator = AcceptAll;
    fn machine(&self) -> Entity { self.entity }
}

impl OnComplete {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

// ---------------------------------------------------------------------------
// Repeatable trait + repeater system
// ---------------------------------------------------------------------------

/// Trait for events that the repeater can emit on each tick.
pub trait Repeatable: GearboxMessage + Send + Sync + 'static {
    fn repeat_tick(entity: Entity) -> Self;
}

/// Resets repeater counters when a state with a Repeater is re-entered.
/// Runs before the tick system so a fresh entry starts from the initial count.
pub fn reset_repeater_on_entry(
    mut q_newly_active: Query<&mut Repeater, Added<Active>>,
) {
    for mut repeater in q_newly_active.iter_mut() {
        repeater.remaining = repeater.initial;
    }
}

/// Fires repeater ticks when states with Repeater are entered.
pub fn repeater_system<E: Repeatable>(
    mut q_newly_active: Query<(Entity, &Active, &mut Repeater), Added<Active>>,
    mut writer_e: MessageWriter<E>,
    mut writer_complete: MessageWriter<OnComplete>,
) {
    for (_state, active, mut repeater) in q_newly_active.iter_mut() {
        if repeater.remaining > 0 {
            writer_e.write(E::repeat_tick(active.machine));
            repeater.remaining -= 1;
        } else {
            writer_complete.write(OnComplete::new(active.machine));
        }
    }
}

// ---------------------------------------------------------------------------
// template_repeater builder
// ---------------------------------------------------------------------------

pub fn template_repeater<E: Repeatable>(
    remaining: u32,
    delay_seconds: f32,
    on_repeat: impl FnOnce(&mut EntityCommands),
) -> impl FnOnce(&mut EntityCommands) {
    move |parent_state: &mut EntityCommands| {
        let parent_entity = parent_state.id();
        parent_state.with_children(|parent| {
            let repeat = parent
                .spawn((
                    Name::new("Repeat"),
                    SubstateOf(parent_entity),
                    Repeater::new(remaining),
                ))
                .id();

            let apply = parent
                .spawn((
                    Name::new("RepeatApply"),
                    SubstateOf(parent_entity),
                    InvokedBy(parent_entity),
                ))
                .id();

            let mut apply_ec = parent.commands_mut().entity(apply);
            on_repeat(&mut apply_ec);

            parent.spawn((
                Name::new("OnRepeat"),
                Source(repeat),
                Target(apply),
                MessageEdge::<E>::default(),
            ));

            parent.spawn((
                Name::new("RepeatDelay"),
                Source(apply),
                Target(repeat),
                AlwaysEdge,
                Delay {
                    duration: std::time::Duration::from_secs_f32(delay_seconds),
                },
            ));
        });
    }
}
