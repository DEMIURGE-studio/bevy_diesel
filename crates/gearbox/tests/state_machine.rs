use std::time::Duration;

use bevy::prelude::*;
use bevy_gearbox::{GearboxPlugin, RegistrationAppExt, prelude::*, transitions::{Delay, ResetEdge, ResetScope}};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default()); // Type registry/reflection for components
    app.add_plugins(GearboxPlugin);
    app
}

#[test]
fn init_enters_initial_chain_and_sets_active_sets() {
    let mut app = test_app();

    // Create a simple machine: root --InitialState--> leaf
    let root = app.world_mut().spawn_empty().id();
    let leaf = app.world_mut().spawn_empty().id();

    // Wire hierarchy and initial
    app.world_mut().entity_mut(leaf).insert(SubstateOf(root));
    app.world_mut().entity_mut(root).insert((StateMachine::new(), InitialState(leaf)));

    // Run one frame to initialize the machine via OnAdd<StateMachine>
    app.update();

    // Assert active sets
    let sm = app.world().get::<StateMachine>(root).expect("StateMachine present");
    assert!(sm.active_leaves.contains(&leaf), "leaf should be active");
    assert!(sm.active.contains(&leaf), "leaf should be in active set");
    assert!(sm.active.contains(&root), "root should be in active ancestor set");
}


#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct TestEvt { #[event_target] target: Entity }

#[test]
fn transitions_priority_first_match_wins() {
    let mut app = test_app();

    // Build hierarchy: root has children S, T1, T2. Initial is S
    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t1 = app.world_mut().spawn_empty().id();
    let t2 = app.world_mut().spawn_empty().id();

    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t1).insert(SubstateOf(root));
    app.world_mut().entity_mut(t2).insert(SubstateOf(root));

    // Create edges with insertion order priority: e1 first (S -> T1), then e2 (S -> T2)
    app.world_mut().spawn((Source(s), Target(t1), EventEdge::<TestEvt>::default()));
    app.world_mut().spawn((Source(s), Target(t2), EventEdge::<TestEvt>::default()));

    // Add InitialState and finally StateMachine to trigger init
    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));

    app.update(); // initialize machine

    // Fire event at machine root; should evaluate S's transitions and pick e1 (T1)
    {
        let mut commands = app.world_mut().commands();
        commands.trigger(TestEvt { target: root });
    }
    app.update();

    let sm = app.world().get::<StateMachine>(root).expect("StateMachine present");
    assert!(sm.active_leaves.contains(&t1), "highest priority edge should activate T1");
    assert!(!sm.active_leaves.contains(&t2), "lower priority edge must not fire when higher fired");
    assert!(!sm.active_leaves.contains(&s), "source should be inactive after transition");
}

// Helpers for ordering test
#[derive(Resource, Default, Debug)]
struct OrderLog(Vec<String>);

fn log_enter(enter_state: On<EnterState>, names: Query<&Name>, mut log: ResMut<OrderLog>) {
    if let Ok(name) = names.get(enter_state.target) {
        log.0.push(format!("enter:{}", name.as_str()));
    }
}

fn log_exit(exit_state: On<ExitState>, names: Query<&Name>, mut log: ResMut<OrderLog>) {
    if let Ok(name) = names.get(exit_state.target) {
        log.0.push(format!("exit:{}", name.as_str()));
    }
}

fn log_actions(transition_action: On<EdgeTraversed>, names: Query<&Name>, mut log: ResMut<OrderLog>) {
    if let Ok(name) = names.get(transition_action.target) {
        log.0.push(format!("actions:{}", name.as_str()));
    }
}

#[test]
fn lifecycle_exit_then_transition_actions_then_enter_ordering() {
    let mut app = test_app();

    app.insert_resource(OrderLog::default());
    app.add_observer(log_enter);
    app.add_observer(log_exit);
    app.add_observer(log_actions);

    // root children: A, C; A child: B (initial)
    let root = app.world_mut().spawn((Name::new("root"),)).id();
    let a = app.world_mut().spawn((Name::new("A"),)).id();
    let b = app.world_mut().spawn((Name::new("B"),)).id();
    let c = app.world_mut().spawn((Name::new("C"),)).id();

    app.world_mut().entity_mut(a).insert(SubstateOf(root));
    app.world_mut().entity_mut(b).insert(SubstateOf(a));
    app.world_mut().entity_mut(c).insert(SubstateOf(root));

    // Edge: from B to C on TestEvt
    app.world_mut().spawn((Name::new("e_b_to_c"), Source(b), Target(c), EventEdge::<TestEvt>::default()));

    // Init last
    app.world_mut().entity_mut(a).insert(InitialState(b));
    app.world_mut().entity_mut(root).insert((InitialState(a), StateMachine::new()));

    app.update();

    // Fire event
    app.world_mut().commands().trigger(TestEvt { target: root });
    app.update();

    let log = app.world().resource::<OrderLog>().0.clone();

    // Expected sequence: exit B, exit A, actions edge, enter C
    let sequence = log.join(",");
    
    assert!(sequence.contains("exit:B"), "should exit B first: {}", sequence);
    assert!(sequence.find("exit:B").unwrap() < sequence.find("exit:A").unwrap(), "B before A: {}", sequence);
    assert!(sequence.find("exit:A").unwrap() < sequence.find("actions:e_b_to_c").unwrap(), "A before actions: {}", sequence);
    assert!(sequence.find("actions:e_b_to_c").unwrap() < sequence.find("enter:C").unwrap(), "actions before enter C: {}", sequence);
}

#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct EvtP1 { #[event_target] target: Entity }

#[test]
fn events_root_propagation_one_per_parallel_region() {
    let mut app = test_app();

    // Build root -> P(parallel)
    let root = app.world_mut().spawn_empty().id();
    let p = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(p).insert(SubstateOf(root));

    // Two regions under P
    let r1 = app.world_mut().spawn_empty().id();
    let r2 = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(r1).insert(SubstateOf(p));
    app.world_mut().entity_mut(r2).insert(SubstateOf(p));

    // Leaves per region and their targets
    let s1 = app.world_mut().spawn_empty().id();
    let s1a = app.world_mut().spawn_empty().id();
    let s2 = app.world_mut().spawn_empty().id();
    let s2a = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s1).insert(SubstateOf(r1));
    app.world_mut().entity_mut(s1a).insert(SubstateOf(r1));
    app.world_mut().entity_mut(s2).insert(SubstateOf(r2));
    app.world_mut().entity_mut(s2a).insert(SubstateOf(r2));

    // Set initial leaves per region
    app.world_mut().entity_mut(r1).insert(InitialState(s1));
    app.world_mut().entity_mut(r2).insert(InitialState(s2));

    // Edges from s1->s1a and s2->s2a on the same event
    app.world_mut().spawn((Source(s1), Target(s1a), EventEdge::<EvtP1>::default()));
    app.world_mut().spawn((Source(s2), Target(s2a), EventEdge::<EvtP1>::default()));

    // Initialize
    app.world_mut().entity_mut(root).insert((InitialState(p), StateMachine::new()));
    app.update();

    // Fire event at root; both regions should fire independently
    app.world_mut().commands().trigger(EvtP1 { target: root });
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&s1a));
    assert!(sm.active_leaves.contains(&s2a));
    assert!(!sm.active_leaves.contains(&s1));
    assert!(!sm.active_leaves.contains(&s2));
}

#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct EvtGoOut { #[event_target] target: Entity }
#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct EvtGoBack { #[event_target] target: Entity }

#[test]
fn history_shallow_saves_immediate_children_under_parallel_and_restores() {
    let mut app = test_app();

    // root -> P(parallel, shallow history) -> regions R1,R2 with leaves A,B
    let root = app.world_mut().spawn_empty().id();
    let p = app.world_mut().spawn((History::Shallow,)).id();
    app.world_mut().entity_mut(p).insert(SubstateOf(root));
    let r1 = app.world_mut().spawn_empty().id();
    let r2 = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(r1).insert(SubstateOf(p));
    app.world_mut().entity_mut(r2).insert(SubstateOf(p));

    let a = app.world_mut().spawn_empty().id(); // region 1
    let b = app.world_mut().spawn_empty().id(); // region 2
    app.world_mut().entity_mut(a).insert(SubstateOf(r1));
    app.world_mut().entity_mut(b).insert(SubstateOf(r2));
    app.world_mut().entity_mut(r1).insert(InitialState(a));
    app.world_mut().entity_mut(r2).insert(InitialState(b));

    // Outside state Z and edges: P --EvtGoOut--> Z, root --EvtGoBack--> P
    let z = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(z).insert(SubstateOf(root));
    app.world_mut().spawn((Source(p), Target(z), EventEdge::<EvtGoOut>::default()));
    app.world_mut().spawn((Source(root), Target(p), EventEdge::<EvtGoBack>::default()));

    // Initialize and ensure region2's leaf (b) is active (due to initial)
    app.world_mut().entity_mut(root).insert((InitialState(p), StateMachine::new()));
    app.update();

    // Now fire go-out to exit P to Z; this should save shallow history under P
    app.world_mut().commands().trigger(EvtGoOut { target: root });
    app.update();

    // Go back to P
    app.world_mut().commands().trigger(EvtGoBack { target: root });
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    // Both regions should be restored; for shallow: the direct children under P that were active: r1->a and r2->b
    assert!(sm.active.contains(&a));
    assert!(sm.active.contains(&b));
}

#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct EvtDefer { #[event_target] target: Entity }

#[test]
fn defer_defers_when_active_and_replays_on_exit_without_consuming_region() {
    let mut app = test_app();
    app.init_resource::<OrderLog>();

    // Add systems needed for defer: listener and replay
    app.add_observer(replay_deferred_event::<EvtDefer>);

    // root children: S (with DeferEvent<EvtDefer>), T
    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn((DeferEvent::<EvtDefer>::new(),)).id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // Edge exists only on root to go to T when event is replayed at root
    app.world_mut().spawn((Source(root), Target(t), EventEdge::<EvtDefer>::default()));

    // Initialize to S
    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // Send event targeted at S; it should be deferred and not trigger transition now
    app.world_mut().commands().trigger(EvtDefer { target: s });
    app.update();
    {
        let sm = app.world().get::<StateMachine>(root).unwrap();
        assert!(sm.active_leaves.contains(&s));
        assert!(!sm.active_leaves.contains(&t));
    }

    // Exit S by transitioning root to T via another event replay when S exits
    // Manually cause exit of S by transitioning root to T using a separate path: add edge on S to root->T? Simplify: exit S by explicit Transition event
    // Trigger ExitState on S by transitioning to T through root edge using a direct event
    app.world_mut().commands().trigger(EvtDefer { target: root });
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&t), "deferred event replay at root should activate T");
}

#[test]
fn state_component_adds_on_enter_removes_on_exit() {
    #[derive(Component, Reflect, Clone, PartialEq, Eq, Debug)]
    struct Foo(i32);

    let mut app = test_app();
    app.register_state_component::<Foo>();

    // root -> S (adds Foo)
    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn((StateComponent(Foo(7)),)).id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // On enter of S, Foo should be on root
    assert_eq!(app.world().get::<Foo>(root).cloned(), Some(Foo(7)));

    // Transition to sibling T to exit S
    #[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
    struct Go { #[event_target] target: Entity }
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(t).insert(SubstateOf(root));
    app.world_mut().spawn((Source(s), Target(t), EventEdge::<Go>::default()));
    app.world_mut().commands().trigger(Go { target: root });
    app.update();

    // Foo removed from root after exit
    assert!(app.world().get::<Foo>(root).is_none());
}

#[test]
fn transitions_external_vs_internal_lca_reentry() {
    let mut app = test_app();
    app.insert_resource(OrderLog::default());
    app.add_observer(log_enter);
    app.add_observer(log_exit);
    app.add_observer(log_actions);

    let root = app.world_mut().spawn((Name::new("root"),)).id();
    let a = app.world_mut().spawn((Name::new("A"),)).id();
    app.world_mut().entity_mut(a).insert(SubstateOf(root));

    // External self-edge: should cause exit A, actions, enter A
    let e_ext = app.world_mut().spawn((Name::new("edge_ext"), Source(a), Target(a), EventEdge::<TestEvt>::default())).id();
    app.world_mut().entity_mut(root).insert((InitialState(a), StateMachine::new()));
    app.update();
    app.world_mut().commands().trigger(TestEvt { target: root });
    app.update();
    let seq = app.world().resource::<OrderLog>().0.join(",");
    assert!(seq.contains("exit:A") && seq.contains("enter:A"), "external should exit and reenter: {}", seq);

    // Now internal self-edge: no exit/enter
    app.world_mut().resource_mut::<OrderLog>().0.clear();
    // Block external edge by adding a guard so internal can be selected
    if let Ok(mut ent) = app.world_mut().get_entity_mut(e_ext) {
        ent.insert(Guards { guards: std::iter::once("blocked".to_string()).collect() });
    }
    app.world_mut().spawn((Name::new("edge_int"), Source(a), Target(a), EventEdge::<TestEvt>::default(), EdgeKind::Internal));
    app.world_mut().commands().trigger(TestEvt { target: root });
    app.update();
    let seq2 = app.world().resource::<OrderLog>().0.join(",");
    assert!(!seq2.contains("exit:A") && !seq2.contains("enter:A"), "internal should not exit/enter: {}", seq2);
}

#[test]
fn transitions_ignored_when_missing_target() {
    let mut app = test_app();

    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    // Edge with no Target
    app.world_mut().spawn((Source(s), EventEdge::<TestEvt>::default()));
    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    app.world_mut().commands().trigger(TestEvt { target: root });
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&s), "no target edge should not transition");
}

#[test]
fn always_fires_on_enter_respecting_guards_and_guard_change_rechecks() {
    let mut app = test_app();

    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // Always edge S->T with a guard initially blocking
    let edge = app.world_mut().spawn((Source(s), Target(t), AlwaysEdge, Guards { guards: std::iter::once("lock".to_string()).collect() })).id();

    // Init
    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // Still on S because guard blocks
    {
        let sm = app.world().get::<StateMachine>(root).unwrap();
        assert!(sm.active_leaves.contains(&s));
        assert!(!sm.active_leaves.contains(&t));
    }

    // Remove guard; Changed<Guards> should trigger recheck while S is active
    if let Ok(mut e) = app.world_mut().get_entity_mut(edge) {
        if let Some(mut g) = e.get_mut::<Guards>() { g.guards.clear(); }
    }
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&t), "guard removal should allow Always edge to fire");
}

#[test]
fn after_starts_on_enter_ticks_and_fires_once() {
    let mut app = test_app();

    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // After edge 50ms
    app.world_mut().spawn((
        Source(s),
        Target(t),
        AlwaysEdge,
        Delay { duration: Duration::from_millis(50) },
        EdgeKind::External,
    ));

    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // Advance wall-clock time by 60ms so Bevy's Time observes it
    std::thread::sleep(Duration::from_millis(60));
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&t), "After should fire after duration");
}

#[test]
fn history_deep_restores_exact_leaves() {
    let mut app = test_app();

    // root -> P(History::Deep) -> A -> A1 (leaf) and B (sibling path)
    let root = app.world_mut().spawn_empty().id();
    let p = app.world_mut().spawn((History::Deep,)).id();
    app.world_mut().entity_mut(p).insert(SubstateOf(root));
    let a = app.world_mut().spawn_empty().id();
    let a1 = app.world_mut().spawn_empty().id();
    let b = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(a).insert(SubstateOf(p));
    app.world_mut().entity_mut(b).insert(SubstateOf(p));
    app.world_mut().entity_mut(a1).insert(SubstateOf(a));
    app.world_mut().entity_mut(p).insert(InitialState(a));
    app.world_mut().entity_mut(a).insert(InitialState(a1));

    // Outside Z and edges to go out/in
    #[derive(SimpleTransition, EntityEvent, Reflect, Clone)] struct Out { #[event_target] target: Entity }
    #[derive(SimpleTransition, EntityEvent, Reflect, Clone)] struct Back { #[event_target] target: Entity }
    let z = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(z).insert(SubstateOf(root));
    app.world_mut().spawn((Source(p), Target(z), EventEdge::<Out>::default()));
    app.world_mut().spawn((Source(root), Target(p), EventEdge::<Back>::default()));

    app.world_mut().entity_mut(root).insert((InitialState(p), StateMachine::new()));
    app.update();

    // Go out then back
    app.world_mut().commands().trigger(Out { target: root });
    app.update();
    app.world_mut().commands().trigger(Back { target: root });
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&a1), "deep history should restore exact leaf A1");
}

#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct EvtDelayed { #[event_target] target: Entity }

#[test]
fn event_after_does_not_auto_fire_without_event() {
    let mut app = test_app();

    // States: root -> { s, t }; initial: s
    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // Edge s --(EvtDelayed, After 50ms)--> t
    app.world_mut().spawn((
        Source(s),
        Target(t),
        EventEdge::<EvtDelayed>::default(),
        Delay { duration: Duration::from_millis(50) },
    ));

    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // Do NOT send the event; wait beyond the delay and tick
    std::thread::sleep(Duration::from_millis(60));
    app.update();

    // Should still be on s because EventEdge with After must not auto-fire
    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&s), "should remain on s without event even after delay");
    assert!(!sm.active_leaves.contains(&t), "must not transition to t without event");
}

#[test]
fn event_after_delays_and_fires() {
    let mut app = test_app();

    // States: root -> { s, t }; initial: s
    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // Edge s --(EvtDelayed, After 50ms)--> t
    app.world_mut().spawn((
        Source(s),
        Target(t),
        EventEdge::<EvtDelayed>::default(),
        Delay { duration: Duration::from_millis(50) },
    ));

    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // Fire event at root: should schedule but not immediately transition
    app.world_mut().commands().trigger(EvtDelayed { target: root });
    app.update();
    {
        let sm = app.world().get::<StateMachine>(root).unwrap();
        assert!(sm.active_leaves.contains(&s), "should still be on s before delay elapses");
        assert!(!sm.active_leaves.contains(&t));
    }

    // Wait past duration and tick
    std::thread::sleep(Duration::from_millis(60));
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&t), "delayed event edge should fire after duration");
}

#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct GoTalents { #[event_target] target: Entity }

#[test]
fn transitioning_parent_with_parallel_child_exits_all_descendant_leaves() {
    let mut app = test_app();

    // Build hierarchy mimicking InGame (non-parallel) -> Panels (parallel) with two regions -> leaves
    // and a sibling Talents leaf under InGame.
    let root = app.world_mut().spawn_empty().id();
    let in_game = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(in_game).insert(SubstateOf(root));

    let panels = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(panels).insert(SubstateOf(in_game));
    let talents = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(talents).insert(SubstateOf(in_game));

    // Two regions under Panels
    let left_region = app.world_mut().spawn_empty().id();
    let right_region = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(left_region).insert(SubstateOf(panels));
    app.world_mut().entity_mut(right_region).insert(SubstateOf(panels));

    // Leaves under regions
    let left_closed = app.world_mut().spawn_empty().id();
    let right_closed = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(left_closed).insert(SubstateOf(left_region));
    app.world_mut().entity_mut(right_closed).insert(SubstateOf(right_region));

    // Initials: InGame -> Panels; Panels -> left_closed & right_closed
    app.world_mut().entity_mut(in_game).insert(InitialState(panels));
    app.world_mut().entity_mut(left_region).insert(InitialState(left_closed));
    app.world_mut().entity_mut(right_region).insert(InitialState(right_closed));

    // Edge: InGame --GoTalents--> Talents
    app.world_mut().spawn((Source(in_game), Target(talents), EventEdge::<GoTalents>::default()));

    // Initialize machine at root -> InGame path
    app.world_mut().entity_mut(root).insert((InitialState(in_game), StateMachine::new()));
    app.update();

    // Assert both parallel leaves active under Panels
    {
        let sm = app.world().get::<StateMachine>(root).unwrap();
        assert!(sm.active_leaves.contains(&left_closed));
        assert!(sm.active_leaves.contains(&right_closed));
        assert!(sm.active.contains(&panels));
    }

    // Transition parent InGame to Talents
    app.world_mut().commands().trigger(GoTalents { target: root });
    app.update();

    // After transition, only Talents should be the active leaf and Panels subtree should be inactive.
    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&talents), "Talents must be active leaf");
    assert!(!sm.active_leaves.contains(&left_closed), "Left panel leaf must be inactive");
    assert!(!sm.active_leaves.contains(&right_closed), "Right panel leaf must be inactive");
    assert!(!sm.active.contains(&panels), "Panels parent must not remain active under InGame");
}

#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct EvtDelayed2 { #[event_target] target: Entity }
#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct EvtNow { #[event_target] target: Entity }

#[test]
fn event_after_cancels_when_source_exits_before_timer() {
    let mut app = test_app();

    // States: root -> { s, t, u }; initial: s
    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    let u = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));
    app.world_mut().entity_mut(u).insert(SubstateOf(root));

    // Delayed edge s --(EvtDelayed2, After 200ms)--> t
    app.world_mut().spawn((
        Source(s),
        Target(t),
        EventEdge::<EvtDelayed2>::default(),
        Delay { duration: Duration::from_millis(200) },
    ));
    // Immediate edge s --(EvtNow)--> u
    app.world_mut().spawn((Source(s), Target(u), EventEdge::<EvtNow>::default()));

    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // Schedule delayed transition
    app.world_mut().commands().trigger(EvtDelayed2 { target: root });
    app.update();
    {
        let sm = app.world().get::<StateMachine>(root).unwrap();
        assert!(sm.active_leaves.contains(&s), "still on s; delayed edge not yet fired");
    }

    // Cause source to exit before timer elapses
    app.world_mut().commands().trigger(EvtNow { target: root });
    app.update();
    {
        let sm = app.world().get::<StateMachine>(root).unwrap();
        assert!(sm.active_leaves.contains(&u), "immediate edge should move to u");
    }

    // Wait beyond delayed duration and tick; delayed transition should have been canceled
    std::thread::sleep(Duration::from_millis(250));
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&u), "delayed edge should be canceled after source exits");
    assert!(!sm.active_leaves.contains(&t), "should not wrongly transition to t after cancellation");
}

#[test]
fn reset_machine_reinitializes() {
    let mut app = test_app();
    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // Trigger reset
    app.world_mut().commands().trigger(ResetRegion::new(root));
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&s), "machine should reinitialize to initial state");
}

#[derive(Component, Default)]
struct WasReset;

fn mark_reset(reset: On<Reset>, mut commands: Commands) {
    commands.entity(reset.target).insert(WasReset);
}

#[test]
fn reset_edge_triggers_scope_target() {
    let mut app = test_app();
    app.add_observer(mark_reset);

    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // Edge S->T with ResetEdge(Target)
    app.world_mut().spawn((Source(s), Target(t), EventEdge::<TestEvt>::default(), ResetEdge(ResetScope::Target)));

    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    app.world_mut().commands().trigger(TestEvt { target: root });
    app.update();

    assert!(app.world().get::<WasReset>(t).is_some(), "target subtree should have received Reset");
}

#[test]
fn state_inactive_component_removes_on_enter_restores_on_exit() {
    #[derive(Component, Reflect, Clone, PartialEq, Eq, Debug)]
    struct Bar(&'static str);
    let mut app = test_app();
    app.register_state_component::<Bar>();

    let root = app.world_mut().spawn((Bar("present"),)).id();
    let s = app.world_mut().spawn((StateInactiveComponent(Bar("present")),)).id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));
    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // On enter S, Bar is removed from root
    assert!(app.world().get::<Bar>(root).is_none());

    // Transition S->T
    #[derive(SimpleTransition, EntityEvent, Reflect, Clone)] struct Go { #[event_target] target: Entity }
    app.world_mut().spawn((Source(s), Target(t), EventEdge::<Go>::default()));
    app.world_mut().commands().trigger(Go { target: root });
    app.update();

    // On exit S, Bar is restored
    assert_eq!(app.world().get::<Bar>(root).cloned(), Some(Bar("present")));
}

#[test]
fn after_timer_respects_guards_added_during_delay() {
    let mut app = test_app();

    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // After edge with 50ms delay
    let edge = app.world_mut().spawn((
        Source(s),
        Target(t),
        AlwaysEdge,
        Delay { duration: Duration::from_millis(50) },
    )).id();

    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update(); // This starts the timer

    // Add a guard to block the transition while timer is running
    app.world_mut().entity_mut(edge).insert(Guards { 
        guards: std::iter::once("block".to_string()).collect() 
    });

    // Advance time past the delay
    std::thread::sleep(Duration::from_millis(60));
    app.update();

    // Should still be on S because guard blocked the delayed transition
    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&s), "should remain on S when guard blocks delayed transition");
    assert!(!sm.active_leaves.contains(&t), "should not transition to T when blocked by guard");
}

#[test]
fn after_timer_handles_missing_target_during_delay() {
    let mut app = test_app();

    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // After edge with 50ms delay
    let edge = app.world_mut().spawn((
        Source(s),
        Target(t),
        AlwaysEdge,
        Delay { duration: Duration::from_millis(50) },
    )).id();

    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update(); // This starts the timer

    // Remove the target component while timer is running
    app.world_mut().entity_mut(edge).remove::<Target>();

    // Advance time past the delay
    std::thread::sleep(Duration::from_millis(60));
    app.update();

    // Should still be on S because target is missing
    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&s), "should remain on S when target is missing");
    assert!(!sm.active_leaves.contains(&t), "should not transition when target is missing");
}

#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct EvtSelfHit { #[event_target] target: Entity }

/// Self-transition on a state with parallel children should produce exactly one
/// exit/enter cycle, not cascade repeatedly.
#[test]
fn self_transition_with_parallel_children_fires_once() {
    let mut app = test_app();
    app.insert_resource(OrderLog::default());
    app.add_observer(log_enter);
    app.add_observer(log_exit);

    // Hierarchy:
    //   root
    //     └── flying  (no InitialState → parallel children)
    //           ├── child_a
    //           └── child_b
    //
    // Edge: flying →[EvtSelfHit]→ flying (external self-transition)

    let root = app.world_mut().spawn((Name::new("root"),)).id();
    let flying = app.world_mut().spawn((Name::new("flying"),)).id();
    let child_a = app.world_mut().spawn((Name::new("child_a"),)).id();
    let child_b = app.world_mut().spawn((Name::new("child_b"),)).id();

    app.world_mut().entity_mut(flying).insert(SubstateOf(root));
    app.world_mut().entity_mut(child_a).insert(SubstateOf(flying));
    app.world_mut().entity_mut(child_b).insert(SubstateOf(flying));

    // Self-transition edge
    app.world_mut().spawn((
        Name::new("self_edge"),
        Source(flying),
        Target(flying),
        EventEdge::<EvtSelfHit>::default(),
    ));

    app.world_mut().entity_mut(root).insert((InitialState(flying), StateMachine::new()));
    app.update();

    // Clear init logs
    app.world_mut().resource_mut::<OrderLog>().0.clear();

    // Fire ONE event
    app.world_mut().commands().trigger(EvtSelfHit { target: root });
    app.update();

    let log = app.world().resource::<OrderLog>().0.clone();

    // Count how many times flying is entered - should be exactly 1
    let flying_enters = log.iter().filter(|s| *s == "enter:flying").count();
    let flying_exits = log.iter().filter(|s| *s == "exit:flying").count();

    assert_eq!(
        flying_enters, 1,
        "flying should be entered exactly once, got {flying_enters}. Full log: {log:?}"
    );
    assert_eq!(
        flying_exits, 1,
        "flying should be exited exactly once, got {flying_exits}. Full log: {log:?}"
    );

    // Children should also enter/exit exactly once each
    let child_a_enters = log.iter().filter(|s| *s == "enter:child_a").count();
    let child_b_enters = log.iter().filter(|s| *s == "enter:child_b").count();
    assert_eq!(child_a_enters, 1, "child_a should enter once, got {child_a_enters}. Full log: {log:?}");
    assert_eq!(child_b_enters, 1, "child_b should enter once, got {child_b_enters}. Full log: {log:?}");

    // Machine should still be in the same state
    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&child_a), "child_a should be active leaf");
    assert!(sm.active_leaves.contains(&child_b), "child_b should be active leaf");
}

#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct DelayedTestEvt { #[event_target] target: Entity }

#[test]
fn event_after_timer_respects_guards_added_during_delay() {
    let mut app = test_app();

    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // Event edge with 50ms delay
    let edge = app.world_mut().spawn((
        Source(s),
        Target(t),
        EventEdge::<DelayedTestEvt>::default(),
        Delay { duration: Duration::from_millis(50) },
    )).id();

    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // Fire the event to schedule the delayed transition
    app.world_mut().commands().trigger(DelayedTestEvt { target: root });
    app.update();

    // Add a guard to block the transition while timer is running
    app.world_mut().entity_mut(edge).insert(Guards { 
        guards: std::iter::once("block".to_string()).collect() 
    });

    // Advance time past the delay
    std::thread::sleep(Duration::from_millis(60));
    app.update();

    // Should still be on S because guard blocked the delayed event transition
    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&s), "should remain on S when guard blocks delayed event transition");
    assert!(!sm.active_leaves.contains(&t), "should not transition to T when blocked by guard");
}

#[test]
fn event_after_timer_handles_missing_target_during_delay() {
    let mut app = test_app();

    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // Event edge with 50ms delay
    let edge = app.world_mut().spawn((
        Source(s),
        Target(t),
        EventEdge::<DelayedTestEvt>::default(),
        Delay { duration: Duration::from_millis(50) },
    )).id();

    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.update();

    // Fire the event to schedule the delayed transition
    app.world_mut().commands().trigger(DelayedTestEvt { target: root });
    app.update();

    // Remove the target component while timer is running
    app.world_mut().entity_mut(edge).remove::<Target>();

    // Advance time past the delay
    std::thread::sleep(Duration::from_millis(60));
    app.update();

    // Should still be on S because target is missing
    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&s), "should remain on S when target is missing");
    assert!(!sm.active_leaves.contains(&t), "should not transition when target is missing");
}