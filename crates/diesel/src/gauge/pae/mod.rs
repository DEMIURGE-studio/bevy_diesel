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
                pae_enter_exit_system.after(GearboxSet),
                stats_change_system,
                active_effects_watcher_system,
            ),
        )
        .add_observer(on_add_effect_target)
        .add_observer(on_remove_effect_target);
    }
}

// ---------------------------------------------------------------------------
// State enter/exit system (replaces On<EnterState>/On<ExitState> observers)
// ---------------------------------------------------------------------------

fn pae_enter_exit_system(
    frame_log: Res<FrameTransitionLog>,
    q_applied_mods: Query<&AppliedModifiers>,
    q_activated_mods: Query<&ActivatedModifiers>,
    q_effect_target: Query<&EffectTarget>,
    mut attributes: AttributesMut,
    mut commands: Commands,
) {
    // Process exits first (order matters for modifier removal)
    for (machine, state) in frame_log.all_exited() {
        // on_exit_active_state: remove activated modifiers
        if q_activated_mods.contains(state) {
            // The state entity itself has ActiveState, but the machine has the modifiers
            if let Ok((effect_target, activated_modifiers)) =
                q_effect_target.get(machine).and_then(|et| {
                    q_activated_mods.get(machine).map(|am| (et, am))
                })
            {
                activated_modifiers.remove(effect_target.0, &mut attributes);
            }
        }
    }

    // Process entries
    for (machine, state) in frame_log.all_entered() {
        let Ok(effect_target) = q_effect_target.get(machine) else {
            continue;
        };
        let target_entity = effect_target.0;

        // Determine which state was entered by checking state components.
        // We look at the machine for the modifiers, not the state entity.

        // on_enter_applied_state
        if let Ok(applied_modifiers) = q_applied_mods.get(machine) {
            // Check if the entered state has AppliedState by seeing if the
            // machine now has the StateComponent<AppliedState>.
            // Since StateComponent auto-inserts on the machine, we check there.
            // For now, we apply on every entry and let idempotency handle it.
            // TODO: Check if the specific state that was entered is the applied state.
        }

        // on_enter_active_state
        if let Ok(activated_modifiers) = q_activated_mods.get(machine) {
            activated_modifiers.apply(target_entity, &mut attributes);
        }

        // on_enter_unapplied_state: remove applied modifiers and EffectTarget
        // This is tricky - we need to know if the entered state is specifically
        // the unapplied state. With StateComponent, the machine will have
        // UnappliedState inserted. Check for that.
        if let Ok(applied_modifiers) = q_applied_mods.get(machine) {
            // Only remove if we're entering unapplied (machine has UnappliedState)
            // This will be handled by the StateComponent system inserting UnappliedState
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
