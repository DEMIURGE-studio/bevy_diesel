pub mod state_machine;

use bevy::prelude::*;
use bevy_gauge::prelude::*;
use bevy_gearbox::prelude::*;

use state_machine::{
    ActivatedModifiers, AppliedModifiers, EffectTarget, PersistentAttributeEffect,
    RequirementsOf, RequiresStatsOf, ActiveState, PAESuspend,
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
            (stats_change_system, active_effects_watcher_system),
        )
        .add_observer(on_add_effect_target)
        .add_observer(on_remove_effect_target);
    }
}

// ---------------------------------------------------------------------------
// State enter/exit observers
// ---------------------------------------------------------------------------

pub(crate) fn on_enter_applied_state(
    enter_state: On<EnterState>,
    q_effect: Query<&AppliedModifiers>,
    q_effect_target: Query<&EffectTarget>,
    mut attributes: AttributesMut,
) {
    let effect_entity = enter_state.state_machine;
    let Ok(effect_target) = q_effect_target.get(effect_entity) else {
        return;
    };
    let target_entity = effect_target.0;

    if let Ok(applied_modifiers) = q_effect.get(effect_entity) {
        applied_modifiers.apply(target_entity, &mut attributes);
    }
}

pub(crate) fn on_enter_active_state(
    enter_state: On<EnterState>,
    q_effect: Query<&ActivatedModifiers>,
    q_effect_target: Query<&EffectTarget>,
    mut attributes: AttributesMut,
) {
    let effect_entity = enter_state.state_machine;
    let Ok(effect_target) = q_effect_target.get(effect_entity) else {
        return;
    };
    let target_entity = effect_target.0;

    if let Ok(activated_modifiers) = q_effect.get(effect_entity) {
        activated_modifiers.apply(target_entity, &mut attributes);
    }
}

pub(crate) fn on_exit_active_state(
    exit_state: On<ExitState>,
    q_effect: Query<(&EffectTarget, &ActivatedModifiers)>,
    mut attributes: AttributesMut,
) {
    let effect_entity = exit_state.state_machine;
    if let Ok((effect_target, activated_modifiers)) = q_effect.get(effect_entity) {
        let target_entity = effect_target.0;
        activated_modifiers.remove(target_entity, &mut attributes);
    }
}

pub(crate) fn on_enter_unapplied_state(
    enter_state: On<EnterState>,
    q_effect: Query<&AppliedModifiers>,
    q_effect_target: Query<&EffectTarget>,
    mut attributes: AttributesMut,
    mut commands: Commands,
) {
    let effect_entity = enter_state.state_machine;
    let Ok(effect_target) = q_effect_target.get(effect_entity) else {
        return;
    };
    let target_entity = effect_target.0;

    if let Ok(applied_modifiers) = q_effect.get(effect_entity) {
        applied_modifiers.remove(target_entity, &mut attributes);
    }

    commands.entity(effect_entity).try_remove::<EffectTarget>();
}

// ---------------------------------------------------------------------------
// EffectTarget observers
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
    mut commands: Commands,
) {
    for pae_entity in q_changed_pae.iter() {
        let Ok(maybe_activated_mods) = q_pae.get(pae_entity) else {
            continue;
        };

        let should_be_active = maybe_activated_mods.is_some();
        let is_currently_active = q_active.contains(pae_entity);

        if !should_be_active && is_currently_active {
            commands.trigger(PAESuspend {
                target: pae_entity,
            });
        }
    }
}
