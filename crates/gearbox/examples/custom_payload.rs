// Ported from bevy_gearbox_editor/examples/custom_payload.rs
// Uses protocol server to enable optional remote editor connection
use bevy::prelude::*;
use bevy_gearbox::StateMachineId;
use bevy_gearbox::prelude::*;
use bevy_gearbox::GearboxPlugin;
use bevy::math::primitives::{Plane3d, Sphere, Cuboid};
use bevy_gearbox::transitions::{AlwaysEdge, Delay};
use bevy_gearbox::transitions::EdgeKind;
use bevy_gearbox::server::ServerPlugin;

// This example focuses on TransitionEvent payloads: mapping a trigger event into
// typed Entry/Exit/Effect phase events that carry data (like target and damage).

// --- Events ---

#[derive(EntityEvent, Reflect, Clone)]
#[transition_event]
struct Attack {
    #[event_target]
    pub target: Entity, // shooter state machine root
    pub victim: Entity,
    pub damage: f32,
}

#[derive(EntityEvent, Reflect, Clone)]
#[transition_event]
struct TryDamage {
    #[event_target]
    pub target: Entity,
    pub amount: f32,
}

#[derive(EntityEvent, Reflect, Clone)]
struct TakeDamage {
    #[event_target]
    pub target: Entity,
    pub amount: f32,
}

// Parameter marker for Life -> used to gate death via a param edge
#[derive(Clone)]
#[gearbox_param(kind = "float", source = Life)]
struct LifeParam;

impl FloatParamBinding<Life> for LifeParam {
    fn extract(source: &Life) -> f32 { source.0 }
}

// Map the trigger into a phase payload that targets the victim with TryDamage.
impl TransitionEvent for Attack {
    type EntryEvent = TryDamage;

    type EdgeEvent = NoEvent;
    type ExitEvent = NoEvent;
    type Validator = AcceptAll;

    fn to_entry_event(
        &self,
        _entering: Entity,
        _exiting: Entity,
        _edge: Entity,
    ) -> Option<Self::EntryEvent> {
        Some(TryDamage { target: self.victim, amount: self.damage })
    }
}

impl TransitionEvent for TryDamage {
    type EntryEvent = TakeDamage;

    type EdgeEvent = NoEvent;
    type ExitEvent = NoEvent;
    type Validator = AcceptAll;

    fn to_entry_event(
        &self,
        _entering: Entity,
        _exiting: Entity,
        _edge: Entity,
    ) -> Option<Self::EntryEvent> {
        Some(TakeDamage { target: self.target, amount: self.amount })
    }
}

// --- State markers ---

#[derive(Component, Reflect, Clone)]
#[state_component]
struct Waiting;

#[derive(Component, Reflect, Clone)]
#[state_component]
struct Attacking;

#[derive(Component, Reflect, Clone)]
#[state_component]
struct TargetWaiting;

#[derive(Component, Reflect, Clone)]
#[state_component]
struct TakingDamageState;

#[derive(Component, Reflect, Clone)]
#[state_component]
struct Dead;

#[derive(Component, Reflect, Clone)]
struct DummyTarget;

#[derive(Component, Reflect, Clone)]
struct Shooter;

#[derive(Component, Reflect, Clone)]
struct DamageAmount(pub f32);

#[derive(Component, Reflect, Clone)]
struct Life(pub f32);

#[derive(Component)]
struct BounceTowards { home: Vec3, goal: Vec3, out_speed: f32, return_speed: f32, phase: BouncePhase }

#[derive(Clone, Copy, PartialEq, Eq)]
enum BouncePhase { Out, Return }

#[derive(Resource, Default)]
struct RespawnQueue(Vec<RespawnRequest>);

struct RespawnRequest { position: Vec3, delay: f32, timer: f32 }

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GearboxPlugin)
        .add_plugins(ServerPlugin::default())
        .init_resource::<RespawnQueue>()
        .add_observer(print_enter_state)
        .add_observer(on_enter_taking_damage_color)
        .add_observer(on_exit_taking_damage_color)
        .add_observer(do_damage_on_entry)
        .add_observer(on_add_dead_despawn)
        .add_systems(Startup, setup)
        .add_systems(Update, (input_attack_event, drive_bounces, process_respawn_queue))
        .run();
}

fn setup(mut commands: Commands) {
    commands.queue(|world: &mut World| {
        // Camera
        world.spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 8.0, 14.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));

        // Light
        world.spawn((
            DirectionalLight::default(),
            Transform::from_xyz(6.0, 10.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));

        // Ground
        {
            let ground_mesh = {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                meshes.add(Mesh::from(Plane3d::default().mesh().size(50.0, 50.0)))
            };
            let ground_mat = {
                let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.22, 0.25),
                    perceptual_roughness: 0.9,
                    ..default()
                })
            };

            world.spawn((
                Name::new("Ground"),
                Mesh3d(ground_mesh),
                MeshMaterial3d(ground_mat),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
        }

        // Shooter and Target visuals
        let shooter = {
            // Shooter assets
            let shooter_mesh = {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                meshes.add(Mesh::from(Sphere { radius: 0.5 }))
            };
            let shooter_mat = {
                let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                materials.add(StandardMaterial { base_color: Color::from(bevy::color::palettes::css::GREEN), ..default() })
            };

            world.spawn((
                Name::new("Shooter"),
                Shooter,
                Mesh3d(shooter_mesh),
                MeshMaterial3d(shooter_mat),
                Transform::from_xyz(-4.0, 0.5, 0.0),
                DamageAmount(25.0),
            )).id()
        };

        // Spawn initial defender via template
        let _ = spawn_defender(world, Vec3::new(4.0, 0.75, 0.0));

        // Shooter state machine
        let waiting = world.spawn((
            Name::new("Waiting"),
            SubstateOf(shooter),
            StateComponent(Waiting),
        )).id();

        let attacking = world.spawn((
            Name::new("Attack"),
            SubstateOf(shooter),
            StateComponent(Attacking),
        )).id();

        // Edge: Waiting --(Attack{target,damage})--> Attack
        world.spawn((
            Name::new("Attack"),
            Source(waiting),
            Target(attacking),
            EventEdge::<Attack>::default(),
        ));

        // Edge: Attack --(Always)--> Waiting (immediate return)
        world.spawn((
            Name::new("Always"),
            Source(attacking),
            Target(waiting),
            AlwaysEdge,
        ));

        // Bounce motion: on entering Attack, add BounceTowards to shooter toward defender
        world.entity_mut(attacking).observe(|enter_state: On<EnterState>,
            q_substate_of: Query<&SubstateOf>,
            q_transform: Query<&Transform>,
            q_target: Query<&Transform, With<DummyTarget>>,
            mut commands: Commands,
        |{
            let state = enter_state.target;
            let root = q_substate_of.root_ancestor(state);
            if let Ok(tf) = q_transform.get(root) {
                // Clamp bounce distance to avoid reaching the target; anchor to starting position
                let home = tf.translation;
                // Try to get current target position; fall back to a fixed point
                let goal = q_target.iter().next().map(|t| t.translation).unwrap_or(Vec3::new(4.0, 0.75, 0.0));
                let dir = (goal - home).normalize_or_zero();
                let bump = 0.8; // meters to move outward
                let goal_pos = home + dir * bump;
                commands.entity(root).insert(BounceTowards { home, goal: goal_pos, out_speed: 18.0, return_speed: 24.0, phase: BouncePhase::Out });
            }
        });

        world.entity_mut(shooter).insert(InitialState(waiting));
        world.entity_mut(shooter).insert(StateMachine::new());
    });
}

// Press Space to fire: send Attack(target, damage) to the shooter machine.
fn input_attack_event(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q_shooter: Query<(Entity, &DamageAmount), With<Shooter>>,
    q_dummy: Query<Entity, With<DummyTarget>>,
    mut commands: Commands,
) {
    if !keyboard_input.just_pressed(KeyCode::Space) { return; }
    let Ok((shooter_entity, damage)) = q_shooter.single() else { return; };
    let Ok(target) = q_dummy.single() else { return; };
    println!("\n-- Space: Attack -> target {:?}, damage {}", target, damage.0);
    commands.trigger(Attack { target: shooter_entity, victim: target, damage: damage.0 });
}

fn drive_bounces(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &mut BounceTowards)>,
) {
    for (mut tf, mut b) in &mut q {
        match b.phase {
            BouncePhase::Out => {
                let to = b.goal - tf.translation;
                let d = to.length();
                let step = b.out_speed * time.delta().as_secs_f32();
                if d <= step { tf.translation = b.goal; b.phase = BouncePhase::Return; } else if d > 0.0 { tf.translation += to.normalize() * step; }
            }
            BouncePhase::Return => {
                let to = b.home - tf.translation;
                let d = to.length();
                let step = b.return_speed * time.delta().as_secs_f32();
                if d <= step { tf.translation = b.home; }
                else if d > 0.0 { tf.translation += to.normalize() * step; }
            }
        }
    }
}

// Apply damage to Life on Entry of TakingDamage via payload event (TakeDamage).
fn do_damage_on_entry(
    do_damage: On<TakeDamage>,
    q_substate_of: Query<&SubstateOf>,
    mut q_life: Query<&mut Life>,
) {
    let amount = do_damage.amount;
    let target = do_damage.target;
    // Support either root-targeted or state-targeted entry events
    let root = if q_life.get(target).is_ok() { target } else { q_substate_of.root_ancestor(target) };
    println!("[TakeDamage] root {:?}, amount {}", root, amount);
    if let Ok(mut life) = q_life.get_mut(root) {
        life.0 -= amount;
        println!("[Damage] Applied {amount}, Life now {:.1}", life.0);
    }
}

// Visual feedback: turn red during TakingDamage, restore to gray on exit
fn on_enter_taking_damage_color(
    enter_state: On<EnterState>,
    q_name: Query<&Name>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut material_handles: Query<&mut MeshMaterial3d<StandardMaterial>>, 
) {
    let state = enter_state.target;
    if let Ok(name) = q_name.get(state) {
        if name.as_str() != "TakingDamage" { return; }
    } else { return; }
    let root = enter_state.state_machine;
    if let Ok(mat_handle) = material_handles.get_mut(root) {
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::from(bevy::color::palettes::css::RED);
        }
    }
}

fn on_exit_taking_damage_color(
    exit_state: On<ExitState>,
    q_name: Query<&Name>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut material_handles: Query<&mut MeshMaterial3d<StandardMaterial>>, 
) {
    let state = exit_state.target;
    if let Ok(name) = q_name.get(state) {
        if name.as_str() != "TakingDamage" { return; }
    } else { return; }
    let root = exit_state.state_machine;
    if let Ok(mat_handle) = material_handles.get_mut(root) {
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::from(bevy::color::palettes::css::GRAY);
        }
    }
}

// On entering Dead, schedule respawn and despawn the defender root
fn on_add_dead_despawn(
    add: On<Add, Dead>,
    q_transform: Query<&Transform>,
    mut respawns: ResMut<RespawnQueue>,
    mut commands: Commands,
) {
    let root = add.entity;
    let mut pos = Vec3::ZERO;
    if let Ok(tf) = q_transform.get(root) { pos = tf.translation; }
    respawns.0.push(RespawnRequest { position: pos, delay: 1.0, timer: 0.0 });
    commands.entity(root).try_despawn();
}

fn process_respawn_queue(
    time: Res<Time>,
    mut respawns: ResMut<RespawnQueue>,
    mut commands: Commands,
) {
    // Tick timers and spawn new defenders when due
    let mut spawn_positions: Vec<Vec3> = Vec::new();
    for req in respawns.0.iter_mut() {
        req.timer += time.delta().as_secs_f32();
        if req.timer >= req.delay { spawn_positions.push(req.position); }
    }
    respawns.0.retain(|r| r.timer < r.delay);
    if spawn_positions.is_empty() { return; }
    commands.queue(move |world: &mut World| {
        for pos in spawn_positions {
            spawn_defender(world, pos);
        }
    });
}

fn spawn_defender(world: &mut World, position: Vec3) -> Entity {
    // Target assets then spawn
    let target_mesh = {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        meshes.add(Mesh::from(Cuboid::new(1.0, 1.5, 1.0)))
    };
    let target_mat = {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        materials.add(StandardMaterial { base_color: Color::from(bevy::color::palettes::css::GRAY), ..default() })
    };

    let defender = world.spawn((
        Name::new("DummyTarget"),
        DummyTarget,
        Mesh3d(target_mesh),
        MeshMaterial3d(target_mat),
        Transform::from_translation(position),
        Life(60.0),
        FloatParam::<LifeParam>::default(),
        StateMachineId::new("dummy_target"), // ID lets the editor connect a sidecar file to the state machine
    )).id();

    // Defender state machine (root = defender)
    let target_waiting = world.spawn((
        Name::new("TargetWaiting"),
        SubstateOf(defender),
        StateComponent(TargetWaiting),
    )).id();

    let taking_damage = world.spawn((
        Name::new("TakingDamage"),
        SubstateOf(defender),
        StateComponent(TakingDamageState),
    )).id();

    let dead = world.spawn((
        Name::new("Dead"),
        SubstateOf(defender),
        StateComponent(Dead),
    )).id();

    // Edge: TargetWaiting --(TryDamage)--> TakingDamage
    world.spawn((
        Name::new("TakeDamage"),
        Source(target_waiting),
        Target(taking_damage),
        EventEdge::<TryDamage>::default(),
        EdgeKind::External,
    ));

    // Edge: Defender root --(Always, when Life <= 0)--> Dead via param guard
    world.spawn((
        Name::new("DieByParam"),
        Source(defender),
        Target(dead),
        AlwaysEdge,
        Guards::new(),
        FloatInRange::<LifeParam>::new(f32::MIN, 0.0, 0.0),
    ));

    // Edge: TakingDamage --(Always, After 0.2s)--> TargetWaiting
    world.spawn((
        Name::new("Always"),
        Source(taking_damage),
        Target(target_waiting),
        AlwaysEdge,
        Delay { duration: std::time::Duration::from_millis(200) },
    ));

    world.entity_mut(defender).insert(InitialState(target_waiting));
    world.entity_mut(defender).insert(StateMachine::new());

    defender
}

fn print_enter_state(enter_state: On<EnterState>, names: Query<&Name>) {
    if let Ok(name) = names.get(enter_state.target) {
        println!("[EnterState] {}", name);
    }
}


