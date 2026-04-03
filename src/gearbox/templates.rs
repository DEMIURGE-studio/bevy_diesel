use std::time::Duration;

use bevy::prelude::*;
use bevy_gearbox::prelude::*;

use crate::effect::{GoOffConfig, SubEffectOf};
use crate::events::{OnRepeat, PosBound, StartInvoke};
use crate::gearbox::repeater::Repeater;
use crate::invoke::Ability;
use crate::subeffects::SpawnDieselSubstate;
use crate::target::TargetMutator;
use crate::backend::SpatialBackend;

/// Convenience builder that wraps a component in a sub-effect node with a
/// `TargetMutator::invoker()` for target resolution.
pub fn apply_sub_effect<B: SpatialBackend>(
    effect: impl Component,
) -> impl FnOnce(&mut EntityCommands) {
    move |apply: &mut EntityCommands| {
        apply.insert(TargetMutator::<B>::invoker());
        let apply_entity = apply.id();
        apply.with_children(|parent| {
            parent.spawn((
                Name::new("SubEffect"),
                SubEffectOf(apply_entity),
                effect,
            ));
        });
    }
}

// ---------------------------------------------------------------------------
// template_invoked: Ready → Invoking → Cooldown → Ready
// ---------------------------------------------------------------------------

/// Outermost ability wrapper. Builds a Ready ↔ Invoking ↔ Cooldown state machine.
/// `configure_invoking` receives the Invoking entity's `EntityCommands` to attach
/// sub-machines (repeaters, spawn configs, etc.).
///
/// The Invoking state transitions to Cooldown when it receives a `Done` message
/// from a child terminal state. For single-shot abilities (no repeater), a
/// terminal "Done" substate is automatically added with an always-edge.
pub fn template_invoked<P: PosBound, F>(
    commands: &mut Commands,
    entity: Option<Entity>,
    cooldown: Duration,
    configure_invoking: F,
) -> Entity
where
    F: FnOnce(&mut EntityCommands),
{
    let entity = entity.unwrap_or_else(|| commands.spawn_empty().id());

    commands.entity(entity).with_children(|parent| {
        let ready = parent.spawn_diesel_substate(entity, Name::new("Ready")).id();

        let invoking = parent
            .spawn_diesel_substate(entity, Name::new("Invoking"))
            .id();

        // Let the caller configure the Invoking node
        let mut invoking_ec = parent.commands_mut().entity(invoking);
        configure_invoking(&mut invoking_ec);

        let cooldown_state = parent
            .spawn_diesel_substate(entity, Name::new("Cooldown"))
            .id();

        // Ready → Invoking on StartInvoke
        parent.spawn_transition::<StartInvoke<P>>(ready, invoking);

        // Invoking → Cooldown when a child terminal state emits Done
        parent.spawn_transition::<Done>(invoking, cooldown_state);

        // Cooldown → Ready after delay
        parent
            .spawn_transition_always(cooldown_state, ready)
            .with_delay(cooldown);

        parent
            .commands_mut()
            .entity(entity)
            .insert((Ability,))
            .init_state_machine(ready);
    });

    entity
}

// ---------------------------------------------------------------------------
// template_repeater: counted volley inside a parent state
// ---------------------------------------------------------------------------

/// Builds a Repeater sub-machine inside a parent state. The count is driven
/// by `count_expr` (e.g. `"ProjectileCount@invoker"`). Each tick calls
/// `on_tick` on the Fire/Apply node.
///
/// When the repeater exhausts its count, it transitions to a `TerminalState`
/// child which emits `Done` to the parent.
pub fn template_repeater<P: PosBound, F>(
    count_expr: &str,
    delay_secs: f32,
    on_tick: F,
) -> impl FnOnce(&mut EntityCommands)
where
    F: FnOnce(&mut EntityCommands),
{
    let count_expr = count_expr.to_string();
    move |invoking: &mut EntityCommands| {
        let invoking_entity = invoking.id();

        invoking.with_children(|parent| {
            let repeater = parent
                .spawn_diesel_substate(
                    invoking_entity,
                    (
                        Name::new("Repeater"),
                        Repeater::new(1),
                        bevy_gauge::attributes! {
                            "RepeatCount" => count_expr.as_str(),
                        },
                    ),
                )
                .id();

            let idle = parent
                .spawn_diesel_substate(repeater, Name::new("Idle"))
                .id();

            let fire = parent
                .spawn_diesel_substate(repeater, Name::new("Fire"))
                .id();

            // Configure the Fire node with whatever happens each tick
            let mut fire_ec = parent.commands_mut().entity(fire);
            on_tick(&mut fire_ec);

            // Idle → Fire on each repeat tick
            parent.spawn_transition::<OnRepeat<P>>(idle, fire);

            // Fire → Repeater (bounce back for next cycle)
            parent
                .spawn_transition_always(fire, repeater)
                .with_delay(Duration::from_secs_f32(delay_secs));

            // When exhausted, repeater emits Done directly to its parent (Invoking)

            parent
                .commands_mut()
                .entity(repeater)
                .insert(InitialState(idle));

            parent
                .commands_mut()
                .entity(invoking_entity)
                .insert(InitialState(repeater));
        });
    }
}

// ---------------------------------------------------------------------------
// template_single_shot: one-time action then terminal
// ---------------------------------------------------------------------------

/// Spawns a single terminal child state inside the parent. The child gets the
/// user's configuration applied, plus `TerminalState` — so entering it
/// immediately emits `Done` to the parent.
///
/// Use this for one-shot abilities (no repeater). The child becomes the
/// `InitialState` of the parent.
pub fn template_single_shot<F>(on_fire: F) -> impl FnOnce(&mut EntityCommands)
where
    F: FnOnce(&mut EntityCommands),
{
    move |invoking: &mut EntityCommands| {
        let invoking_entity = invoking.id();

        invoking.with_children(|parent| {
            let fire = parent
                .spawn_diesel_substate(invoking_entity, (Name::new("Fire"), TerminalState, GoOffConfig))
                .id();

            let mut fire_ec = parent.commands_mut().entity(fire);
            on_fire(&mut fire_ec);

            parent
                .commands_mut()
                .entity(invoking_entity)
                .insert(InitialState(fire));
        });
    }
}
