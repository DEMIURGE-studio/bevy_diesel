use bevy::prelude::*;
use bevy_gearbox::{prelude::*, GearboxPlugin, RegistrationAppExt};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(GearboxPlugin);
    app
}

#[derive(States, Component, Clone, Copy, Eq, PartialEq, Hash, Debug, Default)]
#[states(scoped_entities)]
enum TestState {
    #[default]
    A,
    B,
}


#[derive(EntityEvent, Reflect, Clone, SimpleTransition)]
struct Go(Entity);

#[test]
fn bridge_sets_bevy_state_on_enter_and_updates_on_transition() {
    let mut app = test_app();

    // Enable Bevy state and bridge for TestState
    app.init_state::<TestState>();
    app.register_state_bridge::<TestState>();

    // Build a simple machine: root -> s_a (initial) and sibling s_b
    let root = app.world_mut().spawn_empty().id();
    let s_a = app.world_mut().spawn((TestState::A,)).id();
    let s_b = app.world_mut().spawn((TestState::B,)).id();
    app.world_mut().entity_mut(s_a).insert(SubstateOf(root));
    app.world_mut().entity_mut(s_b).insert(SubstateOf(root));

    // Initial to A, and edge A --Go--> B
    app.world_mut().entity_mut(root).insert((InitialState(s_a), StateMachine::new()));
    app.world_mut().spawn((Source(s_a), Target(s_b), EventEdge::<Go>::default()));

    // Initialize; bridge should set Bevy state to A
    app.update();
    let bevy_state = app.world().resource::<State<TestState>>();
    assert_eq!(**bevy_state, TestState::A);

    // Send Go event at root; bridge should update Bevy state to B
    app.world_mut().commands().trigger(Go(root));
    app.update();
    let bevy_state = app.world().resource::<State<TestState>>();
    assert_eq!(**bevy_state, TestState::B);
}

#[test]
fn state_scoped_entities_are_despawned_on_exit_of_chart_state() {
    let mut app = test_app();
    app.init_state::<TestState>();
    app.register_state_bridge::<TestState>();

    let root = app.world_mut().spawn_empty().id();
    let s_a = app.world_mut().spawn((TestState::A,)).id();
    let s_b = app.world_mut().spawn((TestState::B,)).id();
    app.world_mut().entity_mut(s_a).insert(SubstateOf(root));
    app.world_mut().entity_mut(s_b).insert(SubstateOf(root));
    app.world_mut().entity_mut(root).insert((InitialState(s_a), StateMachine::new()));

    // An entity scoped to TestState::A
    let scoped = app.world_mut().spawn((bevy::state::state_scoped::DespawnOnExit(TestState::A), Name::new("scoped"))).id();

    // Initialize to A
    app.update();
    {
        // Ensure entity exists before transition
        assert!(app.world().get_entity(scoped).is_ok());
    }

    // Edge A -> B and fire Go
    app.world_mut().spawn((Source(s_a), Target(s_b), EventEdge::<Go>::default()));
    app.world_mut().commands().trigger(Go(root));
    app.update();

    // Entity should be despawned on exit of the A node
    assert!(app.world().get_entity(scoped).is_err(), "StateScoped<A> entity should be despawned on exit");
}

#[derive(Component)]
struct RootMarker;

#[test]
fn commands_helper_emits_to_marked_chart_root() {
    let mut app = test_app();

    // Build root with marker and two children S -> T
    let root = app.world_mut().spawn((RootMarker,)).id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));
    app.world_mut().entity_mut(root).insert((InitialState(s), StateMachine::new()));
    app.world_mut().spawn((Source(s), Target(t), EventEdge::<Go>::default()));

    // Initialize
    app.update();
    {
        let sm = app.world().get::<StateMachine>(root).unwrap();
        assert!(sm.active_leaves.contains(&s));
    }

    // Use commands helper to emit Go to the chart with RootMarker
    {
        let mut commands = app.world_mut().commands();
        use bevy_gearbox::prelude::GearboxCommandsExt;
        commands.emit_to_chart::<RootMarker>(|root| Go(root));
    }
    app.update();

    let sm = app.world().get::<StateMachine>(root).unwrap();
    assert!(sm.active_leaves.contains(&t));
}


