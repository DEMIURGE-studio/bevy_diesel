pub(crate) mod diagnostics;
pub mod ability_pool;
pub mod backend;
pub mod despawn;
pub mod dot;
pub mod effect;
pub mod events;
pub mod gauge_ext;
pub mod gearbox_ext;
pub mod invoke;
pub mod invoker;
pub mod pipeline;
pub mod print;
pub mod propagation;
pub mod scenes;
pub mod spawn;
pub mod subeffects;
pub mod target;

// `inventory` is re-exported for the `submit_propagation_for!` macro, which
// expands to `inventory::submit!` in the caller's crate.
pub use inventory;

/// System sets for ordering diesel's effect pipeline inside [`GearboxSchedule`].
///
/// These run between [`GearboxPhase::EntryPhase`] and [`GearboxPhase::GaugeSync`]
/// so that sub-effects (attribute changes, spawns, etc.) resolve before
/// derived components are synced and always-edge guards are evaluated.
///
/// ```text
/// GearboxSchedule:
///   TransitionPhase → ApplyDeferred → ExitPhase → EntryPhase
///     → DieselPropagation → ApplyDeferred → DieselEffects → ApplyDeferred
///     → GaugeSync → EdgeCheckPhase
/// ```
#[derive(bevy::prelude::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DieselSet {
    /// Propagation: reads [`GoOffOrigin`], walks tree, writes [`GoOff`].
    Propagation,
    /// Leaf effect systems that consume [`GoOff`].
    Effects,
    /// Subset of Effects: systems that mutate attributes (instant, modifiers).
    /// Runs before guard evaluation so stat changes are visible to branches.
    AttributeEffects,
}

pub mod prelude {
    pub use crate::DieselSet;
    pub use crate::backend::{SpatialBackend, DieselCorePlugin};
    // diesel's aim `Target` is omitted from the prelude: the name belongs to
    // gearbox's transition `Target` (below), which dominates BSN authoring.
    // Backends expose the concrete aim target as `AbilityTarget`.
    pub use crate::target::{
        Scope, InvokerTarget, TargetGenerator, TargetMutator, TargetType,
    };
    pub use crate::effect::{GoOff, GoOffConfig, SubEffectOf, SubEffects};
    pub use crate::events::{StartInvoke, StopInvoke, OnRepeat};
    pub use crate::invoker::{InvokedBy, Invokes, resolve_invoker, resolve_root};
    pub use crate::pipeline::{generate_targets, propagate_observer};
    pub use crate::print::PrintLn;
    pub use crate::spawn::{
        OnSpawnInvoker, OnSpawnOrigin, OnSpawnTarget,
        SpawnConfig, TemplateRegistry, spawn_system,
    };
    pub use crate::gauge_ext::prelude::*;
    pub use crate::go_off;
    pub use crate::gearbox_ext::repeater::{Repeater, repeater_tick};
    #[allow(deprecated)]
    pub use crate::gearbox_ext::templates::{
        apply_sub_effect, template_invoked, template_repeater, template_single_shot,
    };
    pub use crate::scenes::{invoked, invoked_with, repeater, single_shot};
    pub use crate::propagation::{
        PropagatedMessage, PropagationTargets, PropagationTargetOf, RegisterPropagationTargetRoot,
        RegisterPropagationTarget, PropagationRegistrar, PropagationSet,
        register_propagation_for, propagate_message,
    };
    #[allow(deprecated)]
    pub use crate::subeffects::{SpawnSubEffect, SpawnDieselSubstate};
    pub use crate::submit_propagation_for;
    pub use crate::despawn::DespawnEffect;
    pub use crate::invoke::{Ability, InvokeStatus, InvocationComplete, check_should_reinvoke_ability};
    pub use crate::ability_pool::{
        AvailableAbilities, AvailableAbility, RegisterAbility, UnregisterAbility,
        DieselAbilityPoolPlugin, emit_register_on_active, emit_unregister_on_inactive,
        collect_all_abilities,
    };
    pub use crate::dot::{
        PeriodicEffectTargets, PeriodicEffectTarget, PeriodicTick, periodic_tick_system,
    };
}
