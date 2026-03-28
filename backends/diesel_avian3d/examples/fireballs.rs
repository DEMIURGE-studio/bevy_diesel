//! Fireball & Firestorm example
//!
//! Templates: explosion, explosive_projectile (shared), fireball, firestorm_zone, firestorm
//!
//! Left click: fireball at cursor | Right click: firestorm at cursor

use std::time::Duration;

use avian3d::prelude::*;
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;
use diesel_avian3d::prelude::*;

// ---------------------------------------------------------------------------
// Team / collision filtering
// ---------------------------------------------------------------------------

/// Team marker. Same team = allies.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct Team(u32);

/// Filter on projectiles. Determines which teams they can hit.
#[derive(Component, Clone, Debug)]
enum TeamFilter {
    Enemies,
}

impl CollisionFilter for TeamFilter {
    type Lookup = Team;

    fn can_target(&self, invoker: Option<&Team>, target: Option<&Team>) -> bool {
        match (self, invoker, target) {
            (TeamFilter::Enemies, Some(i), Some(t)) => i.0 != t.0,
            _ => true, // no team info → allow (e.g. hitting terrain)
        }
    }
}

const PLAYER_TEAM: Team = Team(0);

// ---------------------------------------------------------------------------
// Collision layers
// ---------------------------------------------------------------------------

#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
enum Layer {
    #[default]
    Terrain,
    Character,
    Projectile,
}

// ---------------------------------------------------------------------------
// Marker components - we won't need these with bsn!
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Player;

#[derive(Component)]
struct ExplosionMarker;

#[derive(Component)]
struct ProjectileMarker;

#[derive(Component)]
struct FirestormZoneMarker;

// ===========================================================================
// TEMPLATES
// ===========================================================================

fn register_templates(mut registry: ResMut<TemplateRegistry>) {
    registry.register("explosion", explosion_template);
    registry.register("explosive_projectile", explosive_projectile_template);
    registry.register("fireball", fireball_ability_template);
    registry.register("firestorm_zone", firestorm_zone_template);
    registry.register("firestorm", firestorm_ability_template);
}

// ---------------------------------------------------------------------------
// explosion - expanding sphere, despawns after a short time
// ---------------------------------------------------------------------------

fn explosion_template(commands: &mut Commands, entity: Option<Entity>) -> Entity {
    let entity = entity.unwrap_or_else(|| commands.spawn_empty().id());

    commands.entity(entity).insert((
        Name::new("Explosion"),
        ExplosionMarker,
        Visibility::Inherited,
        DelayedDespawn::after(0.4),
    ));

    entity
}

// ---------------------------------------------------------------------------
// explosive_projectile - projectile that spawns "explosion" on collision
// ---------------------------------------------------------------------------

fn explosive_projectile_template(commands: &mut Commands, entity: Option<Entity>) -> Entity {
    let entity = entity.unwrap_or_else(|| commands.spawn_empty().id());

    commands.entity(entity).with_children(|parent| {
        let flying = parent
            .spawn((Name::new("Flying"), SubstateOf(entity), InvokedBy(entity)))
            .id();

        // Spawn explosion at collision point
        parent.spawn((
            Name::new("SpawnExplosion"),
            SubstateOf(flying),
            SubEffectOf(flying),
            InvokedBy(entity),
            SpawnConfig::at_passed("explosion"),
        ));

        // Decrement ProjectileLife on root
        let life_targeting = parent
            .spawn((
                Name::new("DecrementLife"),
                SubstateOf(flying),
                SubEffectOf(flying),
                InvokedBy(entity),
                TargetMutator::root(),
            ))
            .id();

        parent.spawn((
            Name::new("DecrementLifeInstant"),
            SubstateOf(life_targeting),
            SubEffectOf(life_targeting),
            InvokedBy(entity),
            bevy_diesel::bevy_gauge::instant! {
                "ProjectileLife" -= 1.0,
            },
        ));

        let done = parent
            .spawn((
                Name::new("Done"),
                SubstateOf(entity),
                StateComponent(DelayedDespawn::now()),
                PrintLn::new("Despawn projectile!!"),
            ))
            .id();

        // Flying →[collision]→ Flying (self-transition re-fires sub-effects)
        parent.spawn((
            Name::new("Flying→Flying (collision)"),
            Source(flying),
            bevy_gearbox::prelude::Target(flying),
            EventEdge::<CollidedEntity>::default(),
        ));

        // Root →[always + guard]→ Done (when ProjectileLife depleted)
        parent.spawn((
            Name::new("Root→Done (depleted)"),
            Source(entity),
            bevy_gearbox::prelude::Target(done),
            Guards::init(["stat_req_unmet"]),
            bevy_diesel::bevy_gauge::requires! { "ProjectileLife <= 0" },
            RequiresStatsOf(entity),
            AlwaysEdge,
        ));

        // Root
        let commands = parent.commands_mut();
        commands.entity(entity).insert((
            Name::new("ExplosiveProjectile"),
            ProjectileMarker,
            ProjectileEffect::new(20.0),
            TeamFilter::Enemies,
            CollisionLayers::new([Layer::Projectile], [Layer::Terrain, Layer::Character]),
            Visibility::Inherited,
            bevy_diesel::bevy_gauge::attributes! {
                "ProjectileLife" => 1.0,
            },
            StateMachine::new(),
            InitialState(flying),
        ));
    });

    entity
}

// ---------------------------------------------------------------------------
// fireball (ability) - spawns explosive_projectile at invoker → target
// ---------------------------------------------------------------------------

fn fireball_ability_template(commands: &mut Commands, entity: Option<Entity>) -> Entity {
    let entity = entity.unwrap_or_else(|| commands.spawn_empty().id());

    commands.entity(entity).with_children(|parent| {
        let ready = parent.spawn((Name::new("Ready"), SubstateOf(entity))).id();

        let invoke = parent.spawn((Name::new("Invoke"), SubstateOf(entity))).id();

        // Spawn projectile at invoker, aimed at invoker's target
        parent.spawn((
            Name::new("SpawnProjectile"),
            SubstateOf(invoke),
            SubEffectOf(invoke),
            InvokedBy(entity),
            SpawnConfig::at_invoker("explosive_projectile")
                .with_offset(Vec3Offset::Fixed(Vec3::Y * 1.5))
                .with_target_generator(TargetGenerator::at_invoker_target()),
        ));

        // Ready →[StartInvoke]→ Invoke
        parent.spawn((
            Name::new("Ready→Invoke"),
            Source(ready),
            bevy_gearbox::prelude::Target(invoke),
            EventEdge::<StartInvoke>::default(),
        ));

        // Invoke →[always]→ Ready (re-arm)
        parent.spawn((
            Name::new("Invoke→Ready"),
            Source(invoke),
            bevy_gearbox::prelude::Target(ready),
            AlwaysEdge,
        ));

        // Root
        let commands = parent.commands_mut();
        commands.entity(entity).insert((
            Name::new("Fireball Ability"),
            Ability,
            StateMachine::new(),
            InitialState(ready),
        ));
    });

    entity
}

// ---------------------------------------------------------------------------
// firestorm_zone - repeater that drops explosive_projectiles in a circle
// ---------------------------------------------------------------------------

fn firestorm_zone_template(commands: &mut Commands, entity: Option<Entity>) -> Entity {
    let entity = entity.unwrap_or_else(|| commands.spawn_empty().id());

    commands.entity(entity).with_children(|parent| {
        let repeating = parent
            .spawn((Name::new("Repeating"), SubstateOf(entity), Repeater::new(5)))
            .id();

        let spawn_wave = parent
            .spawn((
                Name::new("SpawnWave"),
                SubstateOf(entity),
                InvokedBy(entity),
                // Spawn explosive_projectiles in a circle around this zone's position
                SpawnConfig::at_root("explosive_projectile").with_gatherer(AvianGatherer::Circle {
                    radius: 4.0,
                    count: NumberType::Fixed(3),
                }),
            ))
            .id();

        let done = parent
            .spawn((
                Name::new("Done"),
                SubstateOf(entity),
                StateComponent(DelayedDespawn::now()),
            ))
            .id();

        // Repeating →[OnRepeat]→ SpawnWave
        parent.spawn((
            Name::new("Repeating→SpawnWave"),
            Source(repeating),
            bevy_gearbox::prelude::Target(spawn_wave),
            EventEdge::<OnRepeat>::default(),
        ));

        // SpawnWave →[always+delay]→ Repeating
        parent.spawn((
            Name::new("SpawnWave→Repeating"),
            Source(spawn_wave),
            bevy_gearbox::prelude::Target(repeating),
            AlwaysEdge,
            Delay {
                duration: Duration::from_millis(600),
            },
        ));

        // Repeating →[OnComplete]→ Done
        parent.spawn((
            Name::new("Repeating→Done"),
            Source(repeating),
            bevy_gearbox::prelude::Target(done),
            EventEdge::<OnComplete>::default(),
        ));

        // Root
        let commands = parent.commands_mut();
        commands.entity(entity).insert((
            Name::new("Firestorm Zone"),
            FirestormZoneMarker,
            Visibility::Inherited,
            StateMachine::new(),
            InitialState(repeating),
        ));
    });

    entity
}

// ---------------------------------------------------------------------------
// firestorm (ability) - spawns firestorm_zone above target
// ---------------------------------------------------------------------------

fn firestorm_ability_template(commands: &mut Commands, entity: Option<Entity>) -> Entity {
    let entity = entity.unwrap_or_else(|| commands.spawn_empty().id());

    commands.entity(entity).with_children(|parent| {
        let ready = parent.spawn((Name::new("Ready"), SubstateOf(entity))).id();

        let invoke = parent.spawn((Name::new("Invoke"), SubstateOf(entity))).id();

        // Spawn firestorm_zone at target, elevated
        parent.spawn((
            Name::new("SpawnZone"),
            SubstateOf(invoke),
            SubEffectOf(invoke),
            InvokedBy(entity),
            SpawnConfig::at_passed("firestorm_zone")
                .with_offset(Vec3Offset::Fixed(Vec3::new(0.0, 8.0, 0.0))),
        ));

        // Ready →[StartInvoke]→ Invoke
        parent.spawn((
            Name::new("Ready→Invoke"),
            Source(ready),
            bevy_gearbox::prelude::Target(invoke),
            EventEdge::<StartInvoke>::default(),
        ));

        // Invoke →[always]→ Ready
        parent.spawn((
            Name::new("Invoke→Ready"),
            Source(invoke),
            bevy_gearbox::prelude::Target(ready),
            AlwaysEdge,
        ));

        // Root
        let commands = parent.commands_mut();
        commands.entity(entity).insert((
            Name::new("Firestorm Ability"),
            Ability,
            StateMachine::new(),
            InitialState(ready),
        ));
    });

    entity
}

// ===========================================================================
// SCENE
// ===========================================================================

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground plane
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(50.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.35, 0.3),
            ..default()
        })),
        RigidBody::Static,
        Collider::cuboid(50.0, 0.002, 50.0),
        CollisionLayers::new([Layer::Terrain], [Layer::Character, Layer::Projectile]),
    ));

    // Player capsule
    let player = commands
        .spawn((
            Name::new("Player"),
            Player,
            Mesh3d(meshes.add(Capsule3d::new(0.4, 1.2))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.5, 0.8),
                ..default()
            })),
            Transform::from_xyz(0.0, 1.0, 0.0),
            PLAYER_TEAM,
            Invokes::new(),
            InvokerTarget::position(Vec3::ZERO),
        ))
        .id();

    // Spawn abilities as children of the player
    let mut registry_cmds = commands;

    let fireball = fireball_ability_template(&mut registry_cmds, None);
    registry_cmds.entity(fireball).insert(InvokedBy(player));

    let firestorm = firestorm_ability_template(&mut registry_cmds, None);
    registry_cmds.entity(firestorm).insert(InvokedBy(player));

    // Store ability entity references on the player
    registry_cmds.entity(player).insert(PlayerAbilities {
        fireball,
        firestorm,
    });

    // Camera - isometric-ish
    registry_cmds.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_xyz(15.0, 20.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    registry_cmds.spawn((
        Name::new("Light"),
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.3, 0.0)),
    ));
}

#[derive(Component)]
struct PlayerAbilities {
    fireball: Entity,
    firestorm: Entity,
}

// ===========================================================================
// INPUT - resolve cursor, invoke abilities
// ===========================================================================

fn update_cursor_target(
    mut q_invoker_target: Query<&mut InvokerTarget, With<Player>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
    ray_caster: SpatialQuery,
) {
    let Ok((camera, camera_gt)) = camera_query.single() else {
        return;
    };
    let Ok(window) = window.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_gt, cursor_pos) else {
        return;
    };

    let Some(hit) = ray_caster.cast_ray(
        ray.origin,
        ray.direction.into(),
        100.0,
        true,
        &SpatialQueryFilter::default(),
    ) else {
        return;
    };

    let target_pos = ray.origin + *ray.direction * hit.distance;

    for mut invoker_target in q_invoker_target.iter_mut() {
        *invoker_target = InvokerTarget::position(target_pos);
    }
}

fn invoke_abilities(
    mouse: Res<ButtonInput<MouseButton>>,
    q_player: Query<(&PlayerAbilities, &InvokerTarget)>,
    mut commands: Commands,
) {
    let Ok((abilities, invoker_target)) = q_player.single() else {
        return;
    };

    let target = Target::position(invoker_target.position);

    if mouse.just_pressed(MouseButton::Left) {
        commands.trigger(StartInvoke::new(abilities.fireball, vec![target]));
        info!("Fireball → {:.1}", invoker_target.position);
    }

    if mouse.just_pressed(MouseButton::Right) {
        commands.trigger(StartInvoke::new(abilities.firestorm, vec![target]));
        info!("Firestorm → {:.1}", invoker_target.position);
    }
}

// ===========================================================================
// VISUALS - attach meshes to spawned template entities
// ===========================================================================

#[derive(Resource)]
struct VisualAssets {
    projectile_mesh: Handle<Mesh>,
    projectile_material: Handle<StandardMaterial>,
    explosion_mesh: Handle<Mesh>,
    explosion_material: Handle<StandardMaterial>,
    zone_mesh: Handle<Mesh>,
    zone_material: Handle<StandardMaterial>,
}

fn setup_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(VisualAssets {
        projectile_mesh: meshes.add(Sphere::new(0.15)),
        projectile_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.4, 0.0),
            emissive: LinearRgba::new(5.0, 2.0, 0.0, 1.0),
            ..default()
        }),
        explosion_mesh: meshes.add(Sphere::new(0.8)),
        explosion_material: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.6, 0.0, 0.6),
            emissive: LinearRgba::new(8.0, 3.0, 0.0, 1.0),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        zone_mesh: meshes.add(Cylinder::new(4.0, 0.05)),
        zone_material: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.3, 0.0, 0.3),
            emissive: LinearRgba::new(2.0, 0.5, 0.0, 1.0),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
    });
}

fn attach_visuals(
    mut commands: Commands,
    q_projectiles: Query<Entity, Added<ProjectileMarker>>,
    q_explosions: Query<Entity, Added<ExplosionMarker>>,
    q_zones: Query<Entity, Added<FirestormZoneMarker>>,
    assets: Res<VisualAssets>,
) {
    for entity in q_projectiles.iter() {
        commands.entity(entity).insert((
            Mesh3d(assets.projectile_mesh.clone()),
            MeshMaterial3d(assets.projectile_material.clone()),
        ));
    }
    for entity in q_explosions.iter() {
        commands.entity(entity).insert((
            Mesh3d(assets.explosion_mesh.clone()),
            MeshMaterial3d(assets.explosion_material.clone()),
        ));
    }
    for entity in q_zones.iter() {
        commands.entity(entity).insert((
            Mesh3d(assets.zone_mesh.clone()),
            MeshMaterial3d(assets.zone_material.clone()),
        ));
    }
}

// ===========================================================================
// APP
// ===========================================================================

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            MeshPickingPlugin,
            AvianBackend::plugin(),
            CollisionFilterPlugin::<TeamFilter>::default(),
        ))
        .add_systems(Startup, (setup, setup_assets, register_templates))
        .add_systems(
            Update,
            (update_cursor_target, invoke_abilities, attach_visuals),
        )
        .run();
}
