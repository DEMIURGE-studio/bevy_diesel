pub mod modifiers;
pub mod instant;
pub mod pae;

use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::prelude::*;

pub struct DieselGaugePlugin<P: Clone + Copy + Send + Sync + Default + Debug + 'static> {
    _marker: PhantomData<P>,
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> Default for DieselGaugePlugin<P> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<P: Clone + Copy + Send + Sync + Default + Debug + 'static> Plugin for DieselGaugePlugin<P> {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            modifiers::modifier_set_system::<P>,
            instant::instant_set_system::<P>,
        ));
    }
}

pub mod prelude {
    pub use crate::gauge::modifiers::AttributeModifiers;
    pub use crate::gauge::DieselGaugePlugin;

    pub use crate::gauge::pae::{
        DieselPaePlugin,
        PaeEntities,
        pae_state_machine,
        state_machine::{
            PersistentAttributeEffect,
            AppliedModifiers,
            ActivatedModifiers,
            EffectTarget,
            UnappliedState,
            AppliedState,
            ActiveState,
            PAETryApply,
            PAESuspend,
            PAEUnapplyApproved,
            RequiresStatsOf,
            RequirementsOf,
        },
    };

    pub use bevy_gauge::prelude::{
        ModifierSet,
        InstantModifierSet,
        Attributes,
        AttributesMut,
        AttributeRequirements,
        InstantExt,
    };
    pub use bevy_gauge::mod_set;
    pub use bevy_gauge::instant;
    pub use bevy_gauge::requires;
}
