use std::fmt::Debug;

use bevy::prelude::*;
use bevy_gauge::prelude::{AttributesMut, InstantExt, InstantModifierSet};

use crate::effect::GoOff;
use crate::invoker::InvokedBy;

/// Observer: when GoOff fires on an entity with [`InstantModifierSet`],
/// applies it with role-based expression evaluation.
///
/// Roles:
/// - `"attacker"` / `"invoker"` → root ancestor via `InvokedBy` chain
/// - `"defender"` / `"target"` → each target entity from the GoOff
/// - `"ability"` → the effect entity itself
pub fn instant_set_observer<P: Clone + Copy + Send + Sync + Default + Debug + 'static>(
    go_off: On<GoOff<P>>,
    q_instant_set: Query<&InstantModifierSet>,
    q_invoked_by: Query<&InvokedBy>,
    mut attributes: AttributesMut,
) {
    let effect_entity = go_off.entity;
    let Ok(instant) = q_instant_set.get(effect_entity) else {
        return;
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
