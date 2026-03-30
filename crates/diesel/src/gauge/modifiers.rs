use std::fmt::Debug;

use bevy::prelude::*;
use bevy_gauge::prelude::{AttributesMut, ModifierSet};

use crate::effect::GoOff;

/// Component wrapping a [`ModifierSet`] that is applied to target entities
/// when GoOff fires on this entity.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut)]
pub struct AttributeModifiers(pub ModifierSet);

/// When GoOff fires on an entity with [`AttributeModifiers`],
/// applies the modifier set to every target entity.
pub fn modifier_set_system<P: Clone + Copy + Send + Sync + Default + Debug + 'static>(
    mut reader: MessageReader<GoOff<P>>,
    q_attribute_modifiers: Query<&AttributeModifiers>,
    mut attributes: AttributesMut,
) {
    for go_off in reader.read() {
        let trigger_entity = go_off.entity;
        let Ok(modifier_set) = q_attribute_modifiers.get(trigger_entity) else {
            continue;
        };

        if let Some(target_entity) = go_off.target.entity {
            modifier_set.apply(target_entity, &mut attributes);
        }
    }
}
