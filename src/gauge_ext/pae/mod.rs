pub mod state_machine;

use bevy::prelude::*;
use bevy_gauge::prelude::*;
use bevy_gearbox::prelude::*;

use state_machine::{
    ActivatedModifiers, ActiveState, AppliedModifiers, EffectTarget, PAESuspend,
    PersistentAttributeEffect, RequirementsOf, RequiresStatsOf,
};

pub use state_machine::{PaeEntities, pae_state, pae_state_machine};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct DieselPaePlugin;

impl Plugin for DieselPaePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                pae_exit_system.after(GearboxSet),
                pae_enter_system.after(GearboxSet),
                active_effects_watcher_system,
            ),
        );
        app.add_systems(
            bevy_gearbox::GearboxSchedule,
            attribute_requirements_blocker
                .in_set(bevy_gearbox::GearboxPhase::BlockerPhase),
        );
        app.add_observer(on_add_effect_target)
            .add_observer(on_remove_effect_target);
    }
}

// ---------------------------------------------------------------------------
// State enter/exit systems (use Active component change detection)
// ---------------------------------------------------------------------------

fn pae_exit_system(
    mut removed: RemovedComponents<Active>,
    q_active_state: Query<&SubstateOf, With<StateComponent<ActiveState>>>,
    q_activated_mods: Query<&ActivatedModifiers>,
    q_effect_target: Query<&EffectTarget>,
    mut attributes: AttributesMut,
) {
    for entity in removed.read() {
        // Only act when exiting the ActiveState, not any other PAE state.
        // The parent of a PAE ActiveState (via SubstateOf) is the PAE
        // container, which carries EffectTarget and ActivatedModifiers.
        // Single-parent-hop works for both top-level PAE (parent = chart
        // root / container) and nested PAE (parent = intermediate container).
        let Ok(parent) = q_active_state.get(entity) else {
            continue;
        };
        let container = parent.0;
        if let Ok(effect_target) = q_effect_target.get(container) {
            if let Ok(activated_modifiers) = q_activated_mods.get(container) {
                activated_modifiers.remove(effect_target.0, &mut attributes);
            }
        }
    }
}

fn pae_enter_system(
    q_newly_active: Query<
        (Entity, &SubstateOf),
        (Added<Active>, With<StateComponent<ActiveState>>),
    >,
    q_activated_mods: Query<&ActivatedModifiers>,
    q_effect_target: Query<&EffectTarget>,
    mut attributes: AttributesMut,
) {
    // See comment in pae_exit_system: the PAE container is the direct
    // SubstateOf parent of an ActiveState, regardless of whether the PAE
    // is top-level or nested inside a larger chart.
    for (_active_state, parent) in &q_newly_active {
        let container = parent.0;
        if let Ok(effect_target) = q_effect_target.get(container) {
            if let Ok(activated_modifiers) = q_activated_mods.get(container) {
                activated_modifiers.apply(effect_target.0, &mut attributes);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// EffectTarget observers (these are fine as observers - they react to
// component Add/Remove, not state machine transitions)
// ---------------------------------------------------------------------------

fn on_add_effect_target(
    add: On<Add, EffectTarget>,
    q_effect_target: Query<&EffectTarget>,
    mut commands: Commands,
) {
    let entity = add.entity;
    let Ok(effect_target) = q_effect_target.get(entity) else {
        return;
    };
    commands
        .entity(entity)
        .insert(RequiresStatsOf(effect_target.0));
}

fn on_remove_effect_target(remove: On<Remove, EffectTarget>, mut commands: Commands) {
    let entity = remove.entity;
    commands.entity(entity).try_remove::<RequiresStatsOf>();
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Blocker system: block transitions whose edge has unmet
/// [`AttributeRequirements`]. Runs in [`BlockerPhase`](bevy_gearbox::GearboxPhase::BlockerPhase).
fn attribute_requirements_blocker(
    mut mutator: MessageMutator<bevy_gearbox::TransitionMessage>,
    q_requirements: Query<(&AttributeRequirements, &RequiresStatsOf)>,
    q_attrs: Query<&Attributes>,
) {
    for msg in mutator.read() {
        if msg.blocked { continue; }
        let Some(edge) = msg.edge else { continue; };
        let Ok((reqs, stats_of)) = q_requirements.get(edge) else { continue; };
        let Ok(attrs) = q_attrs.get(stats_of.0) else {
            msg.blocked = true;
            continue;
        };
        if !reqs.met(attrs) {
            msg.blocked = true;
        }
    }
}

fn active_effects_watcher_system(
    q_changed_pae: Query<Entity, Changed<ActivatedModifiers>>,
    q_pae: Query<Option<&ActivatedModifiers>, With<PersistentAttributeEffect>>,
    q_active: Query<Entity, (With<PersistentAttributeEffect>, With<ActiveState>)>,
    mut writer: MessageWriter<PAESuspend>,
) {
    for pae_entity in q_changed_pae.iter() {
        let Ok(maybe_activated_mods) = q_pae.get(pae_entity) else {
            continue;
        };

        let should_be_active = maybe_activated_mods.is_some();
        let is_currently_active = q_active.contains(pae_entity);

        if !should_be_active && is_currently_active {
            writer.write(PAESuspend { target: pae_entity });
        }
    }
}
