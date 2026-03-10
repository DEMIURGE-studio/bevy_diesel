use std::fmt::Debug;

use bevy::prelude::*;
use bevy_gauge::prelude::{AttributesMut, ModifierSet};

use crate::effect::GoOff;

// ---------------------------------------------------------------------------
// AttributeModifiers — persistent modifier application via GoOff
// ---------------------------------------------------------------------------

/// Component wrapping a [`ModifierSet`] that is applied to target entities
/// when GoOff fires on this entity.
///
/// These are **persistent** modifiers — they remain on the target entity until
/// explicitly removed. For one-shot mutations, use [`InstantModifierSet`]
/// instead.
///
/// # Example
///
/// ```ignore
/// commands.spawn((
///     SubEffectOf(parent),
///     AttributeModifiers(mod_set! {
///         "Strength.added" => 10.0,
///         "Speed.multiplier" => 1.2,
///     }),
/// ));
/// ```
#[derive(Component, Clone, Debug, Default, Deref, DerefMut)]
pub struct AttributeModifiers(pub ModifierSet);

/// Observer: when GoOff fires on an entity with [`AttributeModifiers`],
/// applies the modifier set to every target entity.
pub fn modifier_set_observer<P: Clone + Copy + Send + Sync + Default + Debug + 'static>(
    go_off: On<GoOff<P>>,
    q_attribute_modifiers: Query<&AttributeModifiers>,
    mut attributes: AttributesMut,
) {
    let trigger_entity = go_off.entity;
    let Ok(modifier_set) = q_attribute_modifiers.get(trigger_entity) else {
        return;
    };

    for target in go_off.targets.iter() {
        if let Some(target_entity) = target.entity {
            modifier_set.apply(target_entity, &mut attributes);
        }
    }
}
