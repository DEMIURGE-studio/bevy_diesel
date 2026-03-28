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
pub mod target;

// Re-export upstream dependencies
pub use bevy_gauge;
pub use bevy_gearbox;
pub use inventory;

pub mod prelude {
    // Backend trait + core plugin
    pub use crate::backend::{SpatialBackend, DieselCorePlugin};

    // Core target types (generic — backends alias these)
    pub use crate::target::{InvokerTarget, Target, TargetGenerator, TargetMutator, TargetType};

    // Effect tree
    pub use crate::effect::{GoOff, SubEffectOf, SubEffects};

    // Generic events (position-typed — backends alias these)
    pub use crate::events::{StartInvoke, OnRepeat, CollidedEntity, CollidedPosition};

    // Invoker chain
    pub use crate::invoker::{InvokedBy, Invokes, resolve_invoker, resolve_root};

    // Pipeline
    pub use crate::pipeline::{generate_targets, propagate_observer};

    // Print effect
    pub use crate::print::PrintLn;

    // Spawn
    pub use crate::spawn::{
        OnSpawnInvoker, OnSpawnOrigin, OnSpawnTarget,
        SpawnConfig, TemplateRegistry, spawn_observer,
        on_spawn_invoker, on_spawn_origin, on_spawn_target,
    };

    // Utility types and functions
    pub use crate::filters::{
        CollisionFilter, Collides, NumberType, limit_count, sort_by_distance,
    };

    // Gauge (attributes + PAE)
    pub use crate::gauge::prelude::*;

    // Gearbox (state machines + repeaters) — excludes bevy_gearbox::Target to avoid conflict
    pub use crate::go_off;
    pub use crate::gearbox::repeater::{OnComplete, Repeater, template_repeater};
    pub use crate::gearbox::templates::apply_sub_effect;
    pub use bevy_gearbox::{SimpleTransition, RegistrationAppExt};
    pub use bevy_gearbox::prelude::{
        AlwaysEdge, Delay, EnterState, EventEdge, ExitState,
        Guards, InitialState, Source, StateMachine, StateComponent, SubstateOf,
    };
    pub use bevy_gearbox::transitions::{NoEvent, TransitionEvent, EventValidator};

    // Propagation (upward event bubbling)
    pub use crate::propagation::{
        PropagationTargets, PropagationTargetOf, RegisterPropagationTargetRoot,
        RegisterPropagationTarget, PropagationRegistrar,
        register_propagation_for, propagate_event,
    };
    pub use crate::submit_propagation_for;

    // Despawn utilities
    pub use crate::despawn::{QueueDespawn, DelayedDespawn};

    // Invoke framework
    pub use crate::invoke::{Ability, InvokeStatus, InvocationComplete, check_should_reinvoke_ability};

    // Ability pool
    pub use crate::ability_pool::{
        AvailableAbilities, AvailableAbility, RegisterAbility, UnregisterAbility,
        DieselAbilityPoolPlugin, emit_register_on_active, emit_unregister_on_inactive,
        collect_all_abilities,
    };

    // Periodic effect (DoT) infrastructure
    pub use crate::dot::{
        PeriodicEffectTargets, PeriodicEffectTarget, PeriodicTick, periodic_tick_system,
    };
}
