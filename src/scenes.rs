//! Reusable BSN scene helpers for diesel's invoker/effect hierarchies.

use std::time::Duration;

use bevy::ecs::template::EntityTemplate;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};

use bevy_gearbox::{
    AlwaysEdge, Delay, Done, InitialState, MessageEdge, StateMachine, Substates, Target,
    TerminalState, Transitions,
};

use crate::backend::SpatialBackend;
use crate::effect::GoOffConfig;
use crate::events::{OnRepeat, PosBound, StartInvoke};
use crate::gearbox_ext::repeater::Repeater;
use crate::invoke::Ability;
use crate::invoker::InvokedBy;

/// Ready ↔ Invoking ↔ Cooldown ability shell.
///
/// `make_inner` receives the ability root (`#Root`, threaded for `InvokedBy`)
/// and returns the Invoking state's inner sub-chart. The Invoking state leaves 
/// for Cooldown when its inner chart emits `Done` (e.g. a `TerminalState`); 
/// Cooldown returns to Ready after `cooldown` elapses.
pub fn invoked<P, F, S>(name: &'static str, cooldown: Duration, make_inner: F) -> impl Scene
where
    P: PosBound + Unpin,
    F: Fn(EntityTemplate) -> S + Send + Sync + 'static,
    S: Scene,
{
    bsn! {
        #Root Ability StateMachine InitialState(#Ready)
            Name::new(name)
        Substates [
            #Ready Transitions [
                (Target(#Invoking) MessageEdge::<StartInvoke<P>>::default())
            ],

            #Invoking InitialState(#Inner) Transitions [
                (Target(#Cooldown) MessageEdge::<Done>::default())
            ] Substates [
                #Inner make_inner(#Root)
            ],

            #Cooldown Transitions [
                (Target(#Ready) AlwaysEdge Delay::new(cooldown))
            ],
        ]
    }
}

/// Counted volley sub-chart (the declarative `template_repeater`).
///
/// `root` is the ability root (threaded for `InvokedBy`); `count_expr` is a gauge
/// expression for the repeat count (`"RepeatCount"`); `on_fire` is merged onto
/// the `Fire` state and runs once per tick. When the count is exhausted the
/// repeater emits `Done` to its parent.
pub fn repeater<P>(
    root: EntityTemplate,
    count_expr: &'static str,
    delay_secs: f32,
    on_fire: impl Scene,
) -> impl Scene
where
    P: PosBound + Unpin,
{
    bsn! {
        #Repeater InvokedBy(root) InitialState(#Idle)
            Repeater::new(1)
            template(move |_| Ok(crate::gauge::attributes! { "RepeatCount" => count_expr }))
        Substates [
            #Idle Transitions [
                (Target(#Fire) MessageEdge::<OnRepeat<P>>::default())
            ],
            #Fire InvokedBy(root) { on_fire }
            Transitions [
                (Target(#Repeater) AlwaysEdge Delay::from_secs_f32(delay_secs))
            ],
        ]
    }
}

/// One-shot terminal sub-state.
///
/// `TerminalState` makes entering it emit `Done` to the parent; `GoOffConfig`
/// fires the effect on entry; `on_fire` is merged onto it (e.g. a spawn config).
/// Designed to be the `#Inner` slot of [`invoked`] for abilities with no volley.
/// `root` is the ability root, threaded as `InvokedBy(root)` so the effect's
/// invoker resolves up to the caller (spawn position / targeting) rather than
/// stopping at this state entity.
pub fn single_shot<B>(root: EntityTemplate, on_fire: impl Scene) -> impl Scene
where
    B: SpatialBackend + Clone + Unpin,
    B::Pos: Unpin,
    B::Offset: Unpin,
    B::Gatherer: Unpin,
    B::Filter: Unpin,
{
    bsn! {
        #Fire InvokedBy(root) TerminalState
            GoOffConfig::<B>::default()
            { on_fire }
    }
}
