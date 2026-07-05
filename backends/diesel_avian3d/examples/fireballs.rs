//! Fireball & Firestorm example — **BSN scenes** edition.
//!
//! Demonstrates diesel's declarative authoring: every "template" is a
//! `fn() -> impl Scene` registered as a scene factory, composed from the core
//! scene helpers (`invoked` / `single_shot` / `repeater`) plus bare diesel
//! components. Visuals are part of the scene (cloned from a cached
//! `VisualAssets` resource in a `template(|ctx| …)` closure) — there is no
//! post-spawn marker/attach system.
//!
//! Scenes: explosion, explosive_projectile (shared), fireball, firestorm_zone,
//! firestorm.
//!
//! Left click: fireball at cursor | Right click: firestorm at cursor

use avian3d::prelude::*;
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use bevy_gauge::{attributes, instant};
use bevy_gearbox::prelude::*;
// `Target` (gearbox's transition target) is aliased to `DieselTarget` for
// diesel's position-target to avoid the name clash.
use diesel_avian3d::bevy_diesel::target::Target as DieselTarget;
use diesel_avian3d::prelude::*;
use diesel_avian3d::DirectionOffset;

// ---------------------------------------------------------------------------
// Team / collision filtering
// ---------------------------------------------------------------------------

/// Team marker. Same team = allies.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
struct Team(u32);

/// Filter on projectiles. Determines which teams they can hit.
#[derive(Component, Clone, Debug, Default, bevy::ecs::template::FromTemplate)]
enum TeamFilter {
    #[default]
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
// ScaleFadeVfx - scales entity down to zero over duration, then despawns
// ---------------------------------------------------------------------------

#[derive(Component, Clone, Default)]
struct ScaleFadeVfx {
    duration: f32,
    elapsed: f32,
    starting_scale: Option<f32>,
}

impl ScaleFadeVfx {
    fn new(duration: f32) -> Self {
        Self {
            duration,
            elapsed: 0.0,
            starting_scale: None,
        }
    }
}

fn scale_fade_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut ScaleFadeVfx)>,
) {
    for (entity, mut transform, mut fade) in query.iter_mut() {
        if fade.starting_scale.is_none() {
            fade.starting_scale = Some(transform.scale.x);
        }
        let Some(starting) = fade.starting_scale else {
            continue;
        };

        fade.elapsed += time.delta_secs();
        let remaining = fade.duration - fade.elapsed;
        if remaining > 0.0 {
            let factor = remaining / fade.duration;
            transform.scale = Vec3::ONE * starting * factor;
        } else {
            commands.entity(entity).try_despawn();
        }
    }
}

// ===========================================================================
// SCENES (registered as scene factories)
// ===========================================================================

fn register_templates(mut registry: ResMut<TemplateRegistry>) {
    registry.register("explosion", || Box::new(explosion()));
    registry.register("explosive_projectile", || Box::new(explosive_projectile()));
    registry.register("fireball", || Box::new(fireball()));
    registry.register("firestorm_zone", || Box::new(firestorm_zone()));
    registry.register("firestorm", || Box::new(firestorm()));
}

/// Mass + inertia for a collider shape, as one mergeable scene (a
/// `MassPropertiesBundle` can't go through a single `template(...)`).
fn mass_properties(shape: Collider, density: f32) -> impl Scene {
    let mp = MassPropertiesBundle::from_shape(&shape, density);
    let (mass, angular_inertia, center_of_mass) = (mp.mass, mp.angular_inertia, mp.center_of_mass);
    bsn! {
        template(move |_| Ok(mass.clone()))
        template(move |_| Ok(angular_inertia.clone()))
        template(move |_| Ok(center_of_mass.clone()))
    }
}

// ---------------------------------------------------------------------------
// explosion - expanding sphere VFX, scale-fades then despawns
// ---------------------------------------------------------------------------

fn explosion() -> impl Scene {
    bsn! {
        Name::new("Explosion")
        Visibility::Inherited
        ScaleFadeVfx::new(0.4)
        template(|ctx| Ok(Mesh3d(ctx.resource::<VisualAssets>().explosion_mesh.clone())))
        template(|ctx| Ok(MeshMaterial3d(ctx.resource::<VisualAssets>().explosion_material.clone())))
    }
}

// ---------------------------------------------------------------------------
// explosive_projectile - Flying → Hit → Done; on collision spawns an explosion
// and decrements ProjectileLife. With ProjectileLife == 1, one hit ends it.
// ---------------------------------------------------------------------------

fn explosive_projectile() -> impl Scene {
    bsn! {
        #Root
            Name::new("ExplosiveProjectile")
            ProjectileEffect::new(20.0)
            TeamFilter::Enemies
            CollisionLayers::new([Layer::Projectile], [Layer::Terrain, Layer::Character])
            Visibility::Inherited
            template(|_| Ok(attributes! { "ProjectileLife" => 1.0 }))
            { mass_properties(Collider::sphere(0.5), 2.0) }
            template(|ctx| Ok(Mesh3d(ctx.resource::<VisualAssets>().projectile_mesh.clone())))
            template(|ctx| Ok(MeshMaterial3d(ctx.resource::<VisualAssets>().projectile_material.clone())))
            StateMachine InitialState(#Flying)
        Substates [
            #Flying Transitions [
                (Target(#Hit) MessageEdge::<CollidedEntity>::default())
            ],

            #Hit Substates [
                // Spawn the explosion at the projectile's position.
                (SubEffectOf(#Hit) InvokedBy(#Root)
                    Name::new("SpawnExplosion")
                    SpawnConfig::passed("explosion")),
                // Decrement projectile life (targets the projectile root).
                #LifeTargeting SubEffectOf(#Hit) InvokedBy(#Root)
                    TargetMutator::root()
                Substates [
                    (SubEffectOf(#LifeTargeting) InvokedBy(#Root)
                        Name::new("DecrementLifeInstant")
                        template(|_| Ok(instant! { "ProjectileLife" -= 1.0 })))
                ],
            ]
            Transitions [
                (Target(#Done) AlwaysEdge)
            ],

            #Done GoOffConfig::root() DespawnEffect,
        ]
    }
}

// ---------------------------------------------------------------------------
// fireball (ability) - single-shot: spawn one explosive_projectile at invoker,
// aimed at the invoker's target.
// ---------------------------------------------------------------------------

fn fireball() -> impl Scene {
    invoked("Fireball Ability", 0.8, |root| {
        single_shot(root, bsn! {
            SpawnConfig::invoker_offset_target(
                "explosive_projectile",
                Vec3Offset::Fixed(DirectionOffset::new(Dir3::Y, 1.5)),
                TargetGenerator::at_invoker_target(),
            )
        })
    })
}

// ---------------------------------------------------------------------------
// firestorm_zone - a spawned zone whose repeater drops 3 volleys of
// explosive_projectiles in a circle, then despawns.
// ---------------------------------------------------------------------------

/// The firestorm volley: 3 waves of 30 explosive_projectiles in a circle around
/// the zone root, 500ms apart. Wraps the generic `repeater` at `Vec3` so it can
/// be called bare in `bsn!` (no turbofish in scene-function position).
fn firestorm_volley(root: bevy::ecs::template::EntityTemplate) -> impl Scene {
    repeater(
        root,
        "3",
        "0.5",
        bsn! {
            template(|_| Ok(SpawnConfig::root("explosive_projectile").with_gatherer(
                AvianGatherer::Circle { radius: 4.0, count: NumberType::Fixed(30) },
            )))
        },
    )
}

fn firestorm_zone() -> impl Scene {
    bsn! {
        #Root
            Name::new("Firestorm Zone")
            Visibility::Inherited
            template(|ctx| Ok(Mesh3d(ctx.resource::<VisualAssets>().zone_mesh.clone())))
            template(|ctx| Ok(MeshMaterial3d(ctx.resource::<VisualAssets>().zone_material.clone())))
            StateMachine InitialState(#RepeaterSlot)
            Transitions [
                (Target(#Done) MessageEdge::<Done>::default())
            ]
        Substates [
            // The repeater sub-chart merges onto this slot (its root carries
            // InitialState/Repeater/Substates). On exhaustion it emits `Done` to
            // its parent (#Root), which transitions to #Done above.
            #RepeaterSlot firestorm_volley(#Root),
            #Done GoOffConfig::root() DespawnEffect,
        ]
    }
}

// ---------------------------------------------------------------------------
// firestorm (ability) - single-shot: spawn a firestorm_zone above the target.
// ---------------------------------------------------------------------------

fn firestorm() -> impl Scene {
    invoked("Firestorm Ability", 1.2, |root| {
        single_shot(root, bsn! {
            template(|_| Ok(SpawnConfig::passed("firestorm_zone")
                .with_offset(Vec3Offset::Fixed(DirectionOffset::new(Dir3::Y, 8.0)))))
        })
    })
}

// ===========================================================================
// SCENE SETUP
// ===========================================================================

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    registry: Res<TemplateRegistry>,
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

    // Spawn abilities as scenes, parented (via InvokedBy) to the player.
    let fireball = registry.spawn("fireball", &mut commands).expect("fireball registered");
    commands.entity(fireball).insert(InvokedBy(player));

    let firestorm = registry.spawn("firestorm", &mut commands).expect("firestorm registered");
    commands.entity(firestorm).insert(InvokedBy(player));

    commands.entity(player).insert(PlayerAbilities { fireball, firestorm });

    // Camera - isometric-ish
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_xyz(15.0, 20.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        Name::new("Light"),
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.3, 0.0)),
    ));
}

#[derive(Component)]
struct Player;

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
    mut writer: MessageWriter<StartInvoke>,
) {
    let Ok((abilities, invoker_target)) = q_player.single() else {
        return;
    };

    let target = DieselTarget::position(invoker_target.position);

    if mouse.just_pressed(MouseButton::Left) {
        writer.write(StartInvoke::new(abilities.fireball, target));
        info!("Fireball → {:.1}", invoker_target.position);
    }

    if mouse.just_pressed(MouseButton::Right) {
        writer.write(StartInvoke::new(abilities.firestorm, target));
        info!("Firestorm → {:.1}", invoker_target.position);
    }
}

// ===========================================================================
// VISUALS - cached handles read by the scene closures above
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
        // setup_assets + register_templates before setup (which spawns scenes).
        .add_systems(Startup, (setup_assets, register_templates, setup).chain())
        .add_systems(
            Update,
            (update_cursor_target, invoke_abilities, scale_fade_system),
        )
        .run();
}
