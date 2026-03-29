use std::fmt::Debug;

use bevy::prelude::*;
use bevy_gauge::prelude::{AttributesMut, InstantExt, InstantModifierSet};

use crate::effect::GoOff;
use crate::invoker::InvokedBy;

/// When GoOff fires on an entity with [`InstantModifierSet`],
/// applies it with role-based expression evaluation.
pub fn instant_set_system<P: Clone + Copy + Send + Sync + Default + Debug + 'static>(
    mut reader: MessageReader<GoOff<P>>,
    q_instant_set: Query<&InstantModifierSet>,
    q_invoked_by: Query<&InvokedBy>,
    mut attributes: AttributesMut,
) {
    for go_off in reader.read() {
        let effect_entity = go_off.entity;
        let Ok(instant) = q_instant_set.get(effect_entity) else {
            continue;
        };

        let attacker = q_invoked_by.root_ancestor(effect_entity);

        for target in go_off.targets.iter() {
            let Some(defender) = target.entity else {
                continue;
            };

            let roles = [
                ("attacker", attacker),
                ("invoker", attacker),
                ("defender", defender),
                ("target", defender),
                ("ability", effect_entity),
            ];

            attributes.apply_instant(instant, &roles, defender);
        }
    }
}
