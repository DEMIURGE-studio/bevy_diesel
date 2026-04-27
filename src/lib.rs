pub(crate) mod diagnostics;
pub mod ability_pool;
pub mod backend;
pub mod despawn;
pub mod dot;
pub mod effect;
pub mod events;
pub mod gauge;
pub mod gearbox;
pub mod invoke;
pub mod invoker;
pub mod pipeline;
pub mod print;
pub mod propagation;
pub mod spawn;
pub mod subeffects;
pub mod target;

// Re-export upstream dependencies
pub use bevy_gauge;
pub use bevy_gearbox;
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
    pub use crate::target::{
        GatherScope, InvokerTarget, Target, TargetGenerator, TargetMutator, TargetType,
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
    pub use crate::gauge::prelude::*;
    pub use crate::go_off;
    pub use crate::gearbox::repeater::{Repeater, repeater_tick};
    pub use crate::gearbox::templates::{
        apply_sub_effect, template_invoked, template_repeater, template_single_shot,
    };
    pub use bevy_gearbox::{RegistrationAppExt, GearboxMessage, AcceptAll};
    pub use bevy_gearbox::prelude::{
        AlwaysEdge, Delay, MessageEdge, Done, TerminalState,
        InitialState, Source, StateMachine, StateComponent, SubstateOf,
        SpawnSubstate, SpawnTransition, BuildTransition, SpawnBranch, TransitionExt, InitStateMachine,
        GearboxSet, EnterState, ExitState, Active,
    };
    pub use crate::propagation::{
        PropagationTargets, PropagationTargetOf, RegisterPropagationTargetRoot,
        RegisterPropagationTarget, PropagationRegistrar,
        register_propagation_for, propagate_event,
    };
    pub use crate::subeffects::{SpawnSubEffect, SpawnDieselSubstate};
    pub use crate::submit_propagation_for;
    pub use crate::despawn::{QueueDespawn, DelayedDespawn};
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
