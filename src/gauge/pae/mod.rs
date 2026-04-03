pub mod state_machine;

use bevy::prelude::*;
use bevy_gauge::prelude::*;
use bevy_gearbox::prelude::*;

use state_machine::{
    ActivatedModifiers, ActiveState, AppliedModifiers, EffectTarget, PAESuspend,
    PersistentAttributeEffect, RequirementsOf, RequiresStatsOf,
};

pub use state_machine::{PaeEntities, pae_state_machine};

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
            stats_change_system
                .in_set(crate::DieselSet::Effects)
                .after(crate::DieselSet::AttributeEffects),
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
    q_active_state: Query<(), With<StateComponent<ActiveState>>>,
    q_activated_mods: Query<&ActivatedModifiers>,
    q_effect_target: Query<&EffectTarget>,
    q_substate_of: Query<&SubstateOf>,
    mut attributes: AttributesMut,
) {
    for entity in removed.read() {
        // Only act when exiting the ActiveState, not any other PAE state
        if !q_active_state.contains(entity) {
            continue;
        }
        let machine = q_substate_of.root_ancestor(entity);
        if let Ok(effect_target) = q_effect_target.get(machine) {
            if let Ok(activated_modifiers) = q_activated_mods.get(machine) {
                activated_modifiers.remove(effect_target.0, &mut attributes);
            }
        }
    }
}

fn pae_enter_system(
    q_newly_active: Query<&Active, (Added<Active>, With<StateComponent<ActiveState>>)>,
    q_activated_mods: Query<&ActivatedModifiers>,
    q_effect_target: Query<&EffectTarget>,
    mut attributes: AttributesMut,
) {
    for active in &q_newly_active {
        if let Ok(effect_target) = q_effect_target.get(active.machine) {
            if let Ok(activated_modifiers) = q_activated_mods.get(active.machine) {
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

fn stats_change_system(
    q_attrs: Query<(Entity, &Attributes), Changed<Attributes>>,
    q_requirements_of: Query<&RequirementsOf>,
    mut q_edge: Query<(&mut Guards, &mut AttributeRequirements)>,
) {
    for (entity, attrs) in q_attrs.iter() {
        for requires_entity in q_requirements_of.iter_descendants(entity) {
            if let Ok((mut guards, mut requirements)) = q_edge.get_mut(requires_entity) {
                let blocker_name = "stat_req_unmet";
                let currently_blocked = guards.has_guard(blocker_name);
                let should_be_blocked = !requirements.met(attrs);

                if currently_blocked != should_be_blocked {
                    if should_be_blocked {
                        guards.add_guard(blocker_name.to_string());
                    } else {
                        guards.remove_guard(blocker_name);
                    }
                }
            }
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
