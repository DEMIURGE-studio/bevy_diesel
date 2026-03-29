use bevy::prelude::*;
use bevy_gearbox::prelude::*;

use crate::gauge::pae::state_machine::EffectTarget;
use crate::invoke::Ability;

// ================= Types =================

#[derive(Clone, Debug, Reflect)]
pub struct AvailableAbility {
    pub ability_entity: Entity,
    pub source_entity: Entity,
}

#[derive(Component, Default, Debug, Reflect)]
pub struct AvailableAbilities {
    pub abilities: Vec<AvailableAbility>,
}

impl AvailableAbilities {
    pub fn add_ability(&mut self, ability: AvailableAbility) {
        if !self
            .abilities
            .iter()
            .any(|a| a.ability_entity == ability.ability_entity)
        {
            self.abilities.push(ability);
        }
    }

    pub fn remove_ability(&mut self, ability_entity: Entity) {
        self.abilities
            .retain(|a| a.ability_entity != ability_entity);
    }

    pub fn get_sorted_abilities(&self) -> Vec<AvailableAbility> {
        let mut sorted = self.abilities.clone();
        sorted.sort_by(|a, b| {
            let display_a = format!("{} - {}", a.source_entity, a.ability_entity);
            let display_b = format!("{} - {}", b.source_entity, b.ability_entity);
            display_a.cmp(&display_b)
        });
        sorted
    }
}

// ================= Events =================

#[derive(EntityEvent, Clone)]
pub struct RegisterAbility {
    #[event_target]
    pub target: Entity,
    pub ability_entity: Entity,
    pub source_entity: Entity,
}

#[derive(EntityEvent, Clone)]
pub struct UnregisterAbility {
    #[event_target]
    pub target: Entity,
    pub ability_entity: Entity,
}

// ================= Plugin =================

pub struct DieselAbilityPoolPlugin;

impl Plugin for DieselAbilityPoolPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_register_ability)
            .add_observer(handle_unregister_ability);
    }
}

// ================= PAE Integration Observers =================

/// On enter, register all Ability children with the PAE's target.
/// Now reads FrameTransitionLog instead of using On<EnterState>.
pub fn emit_register_on_active(
    frame_log: Res<FrameTransitionLog>,
    q_children: Query<&Children>,
    q_ability: Query<Entity, With<Ability>>,
    q_effect_target: Query<&EffectTarget>,
    mut commands: Commands,
) {
    for (machine, _state) in frame_log.all_entered() {
        let effect_entity = machine;
        let Ok(effect_target) = q_effect_target.get(effect_entity) else {
            continue;
        };
        let target_entity = effect_target.0;

        let mut abilities = Vec::new();
        collect_all_abilities(effect_entity, &q_children, &q_ability, &mut abilities);
        for ability_entity in abilities {
            commands.trigger(RegisterAbility {
                target: target_entity,
                ability_entity,
                source_entity: effect_entity,
            });
        }
    }
}

/// On exit, unregister all Ability children from the PAE's target.
/// Now reads FrameTransitionLog instead of using On<ExitState>.
pub fn emit_unregister_on_inactive(
    frame_log: Res<FrameTransitionLog>,
    q_children: Query<&Children>,
    q_ability: Query<Entity, With<Ability>>,
    q_effect_target: Query<&EffectTarget>,
    mut commands: Commands,
) {
    for (machine, _state) in frame_log.all_exited() {
        let effect_entity = machine;
        let Ok(effect_target) = q_effect_target.get(effect_entity) else {
            continue;
        };
        let target_entity = effect_target.0;

        let mut abilities = Vec::new();
        collect_all_abilities(effect_entity, &q_children, &q_ability, &mut abilities);
        for ability_entity in abilities {
            commands.trigger(UnregisterAbility {
                target: target_entity,
                ability_entity,
            });
        }
    }
}

// ================= Internal =================

fn handle_register_ability(
    register_ability: On<RegisterAbility>,
    mut q_available_abilities: Query<&mut AvailableAbilities>,
) {
    let root = register_ability.target;
    match q_available_abilities.get_mut(root) {
        Ok(mut available) => {
            available.add_ability(AvailableAbility {
                ability_entity: register_ability.ability_entity,
                source_entity: register_ability.source_entity,
            });
        }
        Err(_) => {
            warn!("RegisterAbility: Target {} has no AvailableAbilities", root);
        }
    }
}

fn handle_unregister_ability(
    unregister_ability: On<UnregisterAbility>,
    mut q_available_abilities: Query<&mut AvailableAbilities>,
) {
    let root = unregister_ability.target;
    match q_available_abilities.get_mut(root) {
        Ok(mut available) => {
            available.remove_ability(unregister_ability.ability_entity);
        }
        Err(_) => {
            warn!(
                "UnregisterAbility: Target {} has no AvailableAbilities",
                root
            );
        }
    }
}

/// Recursively collect all `Ability` entities under a hierarchy.
pub fn collect_all_abilities(
    entity: Entity,
    q_children: &Query<&Children>,
    q_ability: &Query<Entity, With<Ability>>,
    out: &mut Vec<Entity>,
) {
    if q_ability.contains(entity) {
        out.push(entity);
    }
    if let Ok(children) = q_children.get(entity) {
        for child in children.iter() {
            collect_all_abilities(child, q_children, q_ability, out);
        }
    }
}
