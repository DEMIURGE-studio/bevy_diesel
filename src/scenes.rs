//! Reusable BSN scene helpers for diesel's invoker/effect hierarchies.

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

/// Ready <-> Invoking <-> Cooldown ability shell.
///
/// `make_inner` receives the ability root (`#Root`, threaded for `InvokedBy`)
/// and returns the Invoking state's inner sub-chart. Invoking transitions to
/// Cooldown when its inner chart emits `Done` (e.g. a `TerminalState`);
/// Cooldown returns to Ready after `cooldown` elapses.
pub fn invoked<P, F, S>(name: &'static str, cooldown_secs: f32, make_inner: F) -> impl Scene
where
    P: PosBound + Unpin,
    F: Fn(EntityTemplate) -> S + Send + Sync + 'static,
    S: Scene,
{
    invoked_with::<P, F, S>(
        name,
        cooldown_secs,
        bevy_gauge::modifier_set::ModifierSet::new(),
        make_inner,
    )
}

/// Like [`invoked`], but seeds the ability root with extra base attributes
/// (merged with the shell's own `"Cooldown"` and `"Damage"`).
///
/// The shell seeds two per-ability stats every ability shares: `"Cooldown"` (the
/// fire interval, in seconds) and `"Damage"` (a `1.0`-based damage multiplier the
/// ability's effects read as `"...@ability"`). Per-ability rank-ups are gauge
/// instants on these, e.g. `instant!{ "Damage" += 0.5 }` on the ability root.
///
/// Both seeds are defaults: if `base` already defines `"Cooldown"` or `"Damage"`,
/// the caller's version wins. A game folds its own globals into an ability's
/// effective stats (e.g. `"Cooldown" => "0.8 * CooldownMult@invoker"`) with no
/// game-specific attribute names in the generic shell. The structural
/// `cooldown_secs` still drives the cooldown edge's initial delay.
pub fn invoked_with<P, F, S>(
    name: &'static str,
    cooldown_secs: f32,
    base: bevy_gauge::modifier_set::ModifierSet,
    make_inner: F,
) -> impl Scene
where
    P: PosBound + Unpin,
    F: Fn(EntityTemplate) -> S + Send + Sync + 'static,
    S: Scene,
{
    bsn! {
        #Ability Ability StateMachine InitialState(#Ready)
            Name::new(name)
            template(move |_| {
                let mut set = base.clone();
                let has = |set: &bevy_gauge::modifier_set::ModifierSet, name: &str| {
                    set.entries().iter().any(|e| e.attribute.as_str() == name)
                };
                if !has(&set, "Cooldown") {
                    set.add("Cooldown", cooldown_secs);
                }
                if !has(&set, "Damage") {
                    set.add("Damage", 1.0);
                }
                Ok(bevy_gauge::modifier_set::AttributeInitializer::new(set))
            })
        Substates [
            #Ready Transitions [
                (Target(#Invoking) MessageEdge::<StartInvoke<P>>::default())
            ],

            #Invoking InitialState(#Inner) Transitions [
                (Target(#Cooldown) MessageEdge::<Done>::default())
            ] Substates [
                #Inner make_inner(#Ability)
            ],

            // The cooldown edge's `Delay` attribute aliases the ability's
            // `Cooldown` via the `@ability` source (registered from its
            // `InvokedBy(#Ability)`), and `Delay` is gauge-derived, so
            // modifiers/instants on `Cooldown` change the fire rate live.
            #Cooldown Transitions [
                (Target(#Ready) AlwaysEdge Delay::from_secs_f32(cooldown_secs)
                    InvokedBy(#Ability)
                    template(|_| Ok(bevy_gauge::attributes! { "Delay" => "Cooldown@ability" })))
            ],
        ]
    }
}

/// Counted volley sub-chart (the declarative `template_repeater`).
///
/// `root` is the ability root (threaded for `InvokedBy`); `count_expr` and
/// `interval_expr` are gauge expressions for the repeat count (`"RepeatCount"`)
/// and the per-tick interval in seconds (the `Fire->Repeater` edge's gauge-derived
/// `"Delay"`); `on_fire` is merged onto the `Fire` state and runs once per tick.
/// When the count is exhausted the repeater emits `Done` to its parent.
///
/// Both expressions resolve against `root`'s sources (`@invoker`, `@ability`), so
/// a game scales the cadence with its own stats (e.g. `"0.12 / AttackSpeed@invoker"`)
/// with no stat names in this generic helper. The edge's literal delay is a small
/// pre-sync initial; the gauge value is in place well before the first tick.
pub fn repeater<P>(
    root: EntityTemplate,
    count_expr: &'static str,
    interval_expr: &'static str,
    on_fire: impl Scene,
) -> impl Scene
where
    P: PosBound + Unpin,
{
    bsn! {
        #Repeater InvokedBy(root) InitialState(#Idle)
            Repeater::new(1)
            template(move |_| Ok(bevy_gauge::attributes! { "RepeatCount" => count_expr }))
        Substates [
            #Idle Transitions [
                (Target(#Fire) MessageEdge::<OnRepeat<P>>::default())
            ],
            #Fire InvokedBy(root) { on_fire }
            Transitions [
                (Target(#Repeater) AlwaysEdge Delay::from_secs_f32(0.1)
                    InvokedBy(root)
                    template(move |_| Ok(bevy_gauge::attributes! { "Delay" => interval_expr })))
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
/// invoker resolves up past this state entity to the caller (spawn position /
/// targeting).
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
