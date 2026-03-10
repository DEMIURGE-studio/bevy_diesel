use bevy::prelude::*;
use bevy_gearbox::prelude::*;
use bevy_gearbox::transitions::TransitionEvent;

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
#[derive(SimpleTransition, EntityEvent, Debug, Clone, Reflect)]
pub struct OnComplete {
    #[event_target]
    pub entity: Entity,
}

impl OnComplete {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

// ---------------------------------------------------------------------------
// Repeater observer
// ---------------------------------------------------------------------------

/// Trait for events that the repeater can emit on each tick.
pub trait Repeatable: EntityEvent + TransitionEvent + Send + Sync + 'static
where
    for<'a> <Self as Event>::Trigger<'a>: Default,
{
    fn repeat_tick(entity: Entity) -> Self;
}

pub fn repeater_observer<E: Repeatable>(
    enter_state: On<EnterState>,
    mut q_repeater: Query<&mut Repeater>,
    q_substate_of: Query<&SubstateOf>,
    mut commands: Commands,
) where
    for<'a> <E as Event>::Trigger<'a>: Default,
{
    let state = enter_state.target;
    let Ok(mut repeater) = q_repeater.get_mut(state) else {
        return;
    };
    let root = q_substate_of.root_ancestor(state);

    if repeater.remaining > 0 {
        commands.trigger(E::repeat_tick(root));
        repeater.remaining -= 1;
    } else {
        commands.trigger(OnComplete::new(root));
    }
}

pub fn reset_repeater(reset: On<Reset>, mut q_repeater: Query<&mut Repeater>) {
    let state = reset.target;
    let Ok(mut repeater) = q_repeater.get_mut(state) else {
        return;
    };
    repeater.remaining = repeater.initial;
}

// ---------------------------------------------------------------------------
// template_repeater builder
// ---------------------------------------------------------------------------

pub fn template_repeater<E: Repeatable>(
    remaining: u32,
    delay_seconds: f32,
    on_repeat: impl FnOnce(&mut EntityCommands),
) -> impl FnOnce(&mut EntityCommands)
where
    for<'a> <E as Event>::Trigger<'a>: Default,
{
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
                EventEdge::<E>::default(),
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
