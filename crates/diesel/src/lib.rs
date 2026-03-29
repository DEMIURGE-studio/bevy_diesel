pub mod ability_pool;
pub mod backend;
pub mod despawn;
pub mod dot;
pub mod effect;
pub mod events;
pub mod filters;
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

/// System sets for ordering diesel's effect pipeline in [`Update`].
///
/// ```text
/// GearboxSet          ← state machine resolution + side effect production
///     ↓
/// DieselPropagation   ← propagate_system: reads GoOffOrigin, walks tree, writes GoOff
///     ↓
/// DieselEffects       ← leaf systems: print, spawn, despawn, modifiers, etc.
/// ```
#[derive(bevy::prelude::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DieselSet {
    /// Propagation: reads [`GoOffOrigin`], writes [`GoOff`] for every descendant.
    Propagation,
    /// Leaf effect systems that consume [`GoOff`].
    Effects,
}

pub mod prelude {
    pub use crate::DieselSet;
    pub use crate::backend::{SpatialBackend, DieselCorePlugin};
    pub use crate::target::{InvokerTarget, Target, TargetGenerator, TargetMutator, TargetType};
    pub use crate::effect::{GoOff, SubEffectOf, SubEffects};
    pub use crate::events::{StartInvoke, StopInvoke, OnRepeat, CollidedEntity, CollidedPosition};
    pub use crate::invoker::{InvokedBy, Invokes, resolve_invoker, resolve_root};
    pub use crate::pipeline::{generate_targets, propagate_observer};
    pub use crate::print::PrintLn;
    pub use crate::spawn::{
        OnSpawnInvoker, OnSpawnOrigin, OnSpawnTarget,
        SpawnConfig, TemplateRegistry, spawn_observer,
        on_spawn_invoker, on_spawn_origin, on_spawn_target,
    };
    pub use crate::filters::{
        CollisionFilter, Collides, NumberType, limit_count, sort_by_distance,
    };
    pub use crate::gauge::prelude::*;
    pub use crate::go_off;
    pub use crate::gearbox::repeater::{OnComplete, Repeater, template_repeater};
    pub use crate::gearbox::templates::apply_sub_effect;
    pub use bevy_gearbox::{RegistrationAppExt, GearboxMessage, AcceptAll};
    pub use bevy_gearbox::prelude::{
        AlwaysEdge, Delay, MessageEdge,
        Guards, InitialState, Source, StateMachine, StateComponent, SubstateOf,
        SpawnSubstate, SpawnTransition, BuildTransition, TransitionExt, InitStateMachine,
        GuardProvider, WriteMessageExt,
        GearboxSet, FrameTransitionLog,
    };
    pub use crate::propagation::{
        PropagationTargets, PropagationTargetOf, RegisterPropagationTargetRoot,
        RegisterPropagationTarget, PropagationRegistrar,
        register_propagation_for, propagate_event,
    };
    pub use crate::subeffects::SpawnSubEffect;
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
