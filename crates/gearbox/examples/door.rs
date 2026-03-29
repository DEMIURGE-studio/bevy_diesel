// Ported from bevy_gearbox_editor/examples/door.rs
// Uses protocol server to enable optional remote editor connection
use bevy::prelude::*;
use bevy_gearbox::prelude::*;
use bevy_gearbox::transitions::{Source, Delay, DeferEvent};
use bevy_gearbox::GearboxPlugin;
use std::time::Duration;
use bevy_gearbox::server::ServerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GearboxPlugin)
        .add_plugins(ServerPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, input_system)
        .add_observer(print_enter_state_messages)
        .add_observer(print_exit_state_messages)
        .add_observer(replay_deferred_event::<RequestClose>)
        .run();
}

// --- State Machine Definition ---

/// The root of our door's state machine.
#[derive(Component)]
struct DoorMachine;

// --- State Marker Components ---

/// Marker component for when the door is closed
#[derive(Component, Reflect, Clone)]
#[state_component]
struct DoorClosed;

/// Marker component for when the door is opening
#[derive(Component, Reflect, Clone)]
#[state_component]
struct DoorOpening;

/// Marker component for when the door is open
#[derive(Component, Reflect, Clone)]
#[state_component]
struct DoorOpen;

/// Marker component for when the door is closing
#[derive(Component, Reflect, Clone)]
#[state_component]
struct DoorClosing;

// --- Events ---

/// Event triggered when requesting the door to open (W key)
#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct RequestOpen {
    #[event_target]
    pub target: Entity,
}

/// Event triggered when requesting the door to close (E key)
#[derive(SimpleTransition, EntityEvent, Reflect, Clone)]
struct RequestClose {
    #[event_target]
    pub target: Entity,
}

/// Creates the door state machine hierarchy.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let machine_entity = commands.spawn_empty().id();
    commands.entity(machine_entity).with_children(|c| {
        let closed = c.spawn_empty().id();
        let opening = c.spawn_empty().id();
        let open = c.spawn_empty().id();
        let closing = c.spawn_empty().id();

        let commands = c.commands_mut();

        commands.entity(closed).insert((
            Name::new("Closed"),
            SubstateOf(machine_entity),
            StateComponent(DoorClosed),
        ));

        commands.entity(opening).insert((
            Name::new("Opening"),
            SubstateOf(machine_entity),
            StateComponent(DoorOpening),
            DeferEvent::<RequestClose>::new(), // Defer RequestClose while opening
        ));

        commands.entity(open).insert((
            Name::new("Open"),
            SubstateOf(machine_entity),
            StateComponent(DoorOpen),
        ));

        commands.entity(closing).insert((
            Name::new("Closing"),
            SubstateOf(machine_entity),
            StateComponent(DoorClosing),
        ));

        commands.entity(machine_entity).insert((
            Name::new("DoorStateMachine"),
            DoorMachine,
            StateMachine::new(),
            InitialState(closed),
        ));

        c.spawn((
            Name::new("RequestOpen"),
            Target(opening),
            EventEdge::<RequestOpen>::default(),
            EdgeKind::External,
            Source(closed),
        ));

        c.spawn((
            Name::new("Always"),
            Target(open),
            Source(opening),
            Delay { duration: Duration::from_secs(1) }, // 1 second opening delay
            AlwaysEdge,
        ));

        c.spawn((
            Name::new("RequestClose"),
            Target(closing),
            EventEdge::<RequestClose>::default(),
            EdgeKind::External,
            Source(open),
        ));

        c.spawn((
            Name::new("Always"),
            Target(closed),
            Source(closing),
            Delay { duration: Duration::from_secs(1) }, // 1 second closing delay
            AlwaysEdge,
        ));

        c.spawn((
            Name::new("RequestOpen"),
            Target(opening),
            EventEdge::<RequestOpen>::default(),
            EdgeKind::External,
            Source(closing),
        ));
    });
}

/// Handles keyboard input for door control events.
fn input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    query: Query<Entity, With<DoorMachine>>,
    mut commands: Commands
) {
    let Ok(machine) = query.single() else { return };
    
    // Press 'W' to request door open
    if keyboard_input.just_pressed(KeyCode::KeyW) {
        println!("\n--- 'W' Pressed: Request door open (RequestOpen event) ---");
        commands.trigger(RequestOpen { target: machine });
    }
    
    // Press 'E' to request door close
    if keyboard_input.just_pressed(KeyCode::KeyE) {
        println!("\n--- 'E' Pressed: Request door close (RequestClose event) ---");
        commands.trigger(RequestClose { target: machine });
    }
}

/// Debug system to print messages when states are entered.
fn print_enter_state_messages(enter_state: On<EnterState>, query: Query<&Name>) {
    if let Ok(name) = query.get(enter_state.target) {
        println!("[STATE ENTERED]: {}", name);
    }
}

/// Debug system to print messages when states are exited.
fn print_exit_state_messages(exit_state: On<ExitState>, query: Query<&Name>) {
    if let Ok(name) = query.get(exit_state.target) {
        println!("[STATE EXITED]: {}", name);
    }
}


