use bevy::prelude::*;
use bevy::reflect::Reflect;
use bevy_gearbox::prelude::*;
use bevy_gearbox::transitions::EventEdge;
use bevy_gearbox::GearboxPlugin;
use bevy_gearbox::server::ServerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GearboxPlugin)
        .add_plugins(ServerPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, input_system)
        .add_observer(on_enter_repeating_emit_events)
        .add_observer(reset_repeater)
        .add_observer(print_enter_state_messages)
        .add_observer(print_onrepeat)
        .add_observer(print_oncomplete)
        .register_type::<AbilityMachine>()
        .register_type::<Repeater>()
        // ResetEdge/ResetScope are provided by core
        .register_type::<EventEdge<CastAbility>>()
        .register_type::<EventEdge<OnRepeat>>()
        .register_type::<EventEdge<OnComplete>>()
        .run();
}

#[derive(SimpleTransition, EntityEvent, Clone, Reflect)]
struct CastAbility {
    #[event_target]
    pub target: Entity,
}

#[derive(SimpleTransition, EntityEvent, Clone, Reflect)]
struct OnRepeat {
    #[event_target]
    pub target: Entity,
}

#[derive(SimpleTransition, EntityEvent, Clone, Reflect)]
struct OnComplete {
    #[event_target]
    pub target: Entity,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
struct AbilityMachine;

// Component to attach to the Repeat state
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
struct Repeater {
    remaining: u32,
    initial: u32,
}

impl Default for Repeater {
    fn default() -> Self {
        Self {
            remaining: 5,
            initial: 5,
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // Load the scene built to mirror editor example
    commands.spawn((
        Name::new("State machine (from scene)"),
        DynamicSceneRoot(asset_server.load("repeater.scn.ron")),
    ));
}

fn input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q_machine: Query<Entity, With<AbilityMachine>>,
    mut commands: Commands,
) {
    let Ok(machine) = q_machine.single() else {
        println!("No machine found");
        return;
    };
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        println!("\n--- 'C' Pressed: Sending CastAbility event! ---");
        commands.trigger(CastAbility { target: machine });
    }
}

// Emits OnRepeat/OnComplete when entering a state with Repeater
fn on_enter_repeating_emit_events(
    enter_state: On<EnterState>,
    mut q_repeater: Query<&mut Repeater>,
    mut commands: Commands,
) {
    let Ok(mut repeater) = q_repeater.get_mut(enter_state.target) else {
        return;
    };
    let root = enter_state.state_machine;
    repeater.remaining -= 1;
    if repeater.remaining > 0 {
        commands.trigger(OnRepeat { target: root });
    } else {
        commands.trigger(OnComplete { target: root });
    }
}

fn reset_repeater(reset: On<Reset>, mut q_repeater: Query<&mut Repeater>) {
    let state = reset.target;

    println!("Resetting repeater for state: {:?}", state);

    let Ok(mut repeater) = q_repeater.get_mut(state) else {
        return;
    };
    repeater.remaining = repeater.initial;
}

// Debug helpers
fn print_enter_state_messages(enter_state: On<EnterState>, q_name: Query<&Name>) {
    if let Ok(name) = q_name.get(enter_state.target) {
        println!("[STATE ENTERED]: {}", name);
    }
}

fn print_onrepeat(_t: On<OnRepeat>) {
    println!("OnRepeat event emitted");
}

fn print_oncomplete(_t: On<OnComplete>) {
    println!("OnComplete event emitted");
}
