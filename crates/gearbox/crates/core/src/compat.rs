/// Compatibility module re-exports for code using core-era module paths.
pub mod guards {
    pub use crate::components::{Guard, GuardProvider, Guards};
}

/// Re-exports for `bevy_gearbox::transitions::*`.
pub mod transitions {
    pub use crate::components::{
        AlwaysEdge, Delay, EdgeKind, Guards, Source, Target, Transitions,
        ResetEdge, ResetScope,
    };
    pub use crate::messages::{
        AcceptAll, GearboxMessage, MessageEdge, MessageValidator,
    };
    pub use crate::registration::DeferEvent;
}

/// Prelude module.
pub mod prelude {
    pub use crate::components::{
        Active,
        SubstateOf, Substates, StateMachine, InitialState,
        Source, Target, Transitions, AlwaysEdge, EdgeKind,
        Guards, Delay,
        Guard, GuardProvider,
        ResetEdge, ResetScope,
    };
    pub use crate::state_component::{
        StateComponent, StateInactiveComponent,
        state_component_enter, state_component_exit,
        state_inactive_component_enter, state_inactive_component_exit,
    };
    pub use crate::history::{History, HistoryState};
    pub use crate::registration::DeferEvent;
    pub use crate::messages::{
        GearboxMessage, MessageValidator, AcceptAll, MessageEdge,
    };
    pub use crate::commands::{
        SpawnSubstate, SpawnTransition, BuildTransition,
        TransitionExt, InitStateMachine, WriteMessageExt,
    };
    pub use crate::{GearboxSchedule, GearboxPhase, GearboxSet};
    pub use crate::resolve::{
        TransitionMessage,
        EnterState, ExitState,
    };
    pub use crate::registration::RegistrationAppExt;
    pub use crate::parameters::{
        FloatParam, IntParam, BoolParam,
        FloatParamBinding, IntParamBinding, BoolParamBinding,
        FloatInRange, IntInRange, BoolEquals,
        sync_float_param, sync_int_param, sync_bool_param,
        apply_float_param_guards, apply_int_param_guards, apply_bool_param_guards,
    };
}
