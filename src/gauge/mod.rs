pub mod modifiers;
pub mod instant;
pub mod pae;

use std::marker::PhantomData;

use bevy::prelude::*;

use crate::backend::SpatialBackend;

/// System set for sustained modifier systems. Runs in `Update` after
/// `GearboxSet` so it can react to `Active` component lifecycle.
/// The backend crate must register `sustained_modifier_apply::<B>` into
/// this set — the generic fn can only be monomorphized there.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SustainedModifierSet;

pub struct DieselGaugePlugin<B: SpatialBackend> {
    _marker: PhantomData<B>,
}

impl<B: SpatialBackend> Default for DieselGaugePlugin<B> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<B: SpatialBackend> Plugin for DieselGaugePlugin<B> {
    fn build(&self, app: &mut App) {
        app.add_systems(bevy_gearbox::GearboxSchedule, instant::instant_set_system::<B::Pos>.in_set(crate::DieselSet::AttributeEffects));

        // Configure the sustained modifier set ordering. The remove system
        // is non-generic and registered here. The apply system is generic
        // over B (needs B::Context) and must be registered by the backend
        // crate into this set.
        app.configure_sets(
            Update,
            SustainedModifierSet.after(bevy_gearbox::GearboxSet),
        );
        app.add_systems(
            Update,
            modifiers::sustained_modifier_remove
                .in_set(SustainedModifierSet),
        );
    }
}

pub mod prelude {
    pub use crate::gauge::modifiers::{
        AttributeModifiers, SustainedModifierConfig, SustainedTarget,
    };
    pub use crate::gauge::{DieselGaugePlugin, SustainedModifierSet};

    pub use crate::gauge::pae::{
        DieselPaePlugin,
        PaeEntities,
        pae_state,
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
        AttributeResolvable,
        InstantExt,
    };
    pub use bevy_gauge::mod_set;
    pub use bevy_gauge::instant;
    pub use bevy_gauge::requires;
}
