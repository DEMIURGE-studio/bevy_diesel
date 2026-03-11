pub mod backend;
pub mod effect;
pub mod filters;
pub mod gauge;
pub mod gearbox;
pub mod invoker;
pub mod pipeline;
pub mod print;
pub mod spawn;
pub mod target;

// Re-export upstream dependencies
pub use bevy_gauge;
pub use bevy_gearbox;

pub mod prelude {
    // Backend trait
    pub use crate::backend::SpatialBackend;

    // Core target types (generic — backends alias these)
    pub use crate::target::{InvokerTarget, Target, TargetGenerator, TargetMutator, TargetType};

    // Effect tree
    pub use crate::effect::{GoOff, SubEffectOf, SubEffects};

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
        NumberType, Team, TeamFilter, filter_by_team, limit_count, sort_by_distance,
    };

    // Gauge (attributes + PAE)
    pub use crate::gauge::prelude::*;

    // Gearbox (state machines + repeaters) — excludes bevy_gearbox::Target to avoid conflict
    pub use crate::go_off;
    pub use crate::gearbox::DieselGearboxPlugin;
    pub use crate::gearbox::repeater::{OnComplete, Repeatable, Repeater, template_repeater};
    pub use crate::gearbox::templates::apply_sub_effect;
    pub use bevy_gearbox::{SimpleTransition, RegistrationAppExt};
    pub use bevy_gearbox::prelude::{
        AlwaysEdge, Delay, EnterState, EventEdge, ExitState,
        Guards, InitialState, Source, StateMachine, StateComponent, SubstateOf,
    };
    pub use bevy_gearbox::transitions::{NoEvent, TransitionEvent, EventValidator};
}
