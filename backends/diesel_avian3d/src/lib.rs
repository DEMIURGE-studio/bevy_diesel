use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use avian3d::prelude::*;
use rand::RngCore;

use bevy_diesel::prelude::*;

// ---------------------------------------------------------------------------
// Re-export the diesel ecosystem — users only need this crate
// ---------------------------------------------------------------------------

pub use bevy_diesel;

pub mod ballistics;
pub mod collision;
pub mod projectile;
pub mod velocity;

pub mod prelude {
    // Re-export everything from diesel core (includes gauge, gearbox, bevy_gauge, bevy_gearbox)
    pub use bevy_diesel::prelude::*;

    // Backend-specific types
    pub use crate::{
        AvianBackend, AvianFilter, AvianGatherer, Vec3Offset,
    };

    // Collision
    pub use crate::collision::CollisionFilterPlugin;

    // Ballistics
    pub use crate::ballistics::{
        calculate_low_angle_velocity_with_speed, calculate_high_angle_velocity_with_speed,
        calculate_velocity_with_speed, distance_lock,
    };

    // Projectile effects
    pub use crate::projectile::{
        ProjectileEffect, LinearProjectileEffect, LinearProjectile, ProjectilePlugin,
    };

    // Velocity effect
    pub use crate::velocity::{VelocityEffect, Trajectory, VelocityEffectPlugin};

    // Concrete Vec3 spawn observer helpers — use with .observe(on_spawn_origin) etc.
    pub use crate::{on_spawn_origin, on_spawn_target, on_spawn_invoker};

    // Backend type aliases — users import these instead of the generic types
    pub type InvokerTarget = bevy_diesel::target::InvokerTarget<bevy::math::Vec3>;
    pub type Target = bevy_diesel::target::Target<bevy::math::Vec3>;
    pub type GoOff = bevy_diesel::effect::GoOff<bevy::math::Vec3>;
    pub type StartInvoke = bevy_diesel::events::StartInvoke<bevy::math::Vec3>;
    pub type OnRepeat = bevy_diesel::events::OnRepeat<bevy::math::Vec3>;
    pub type CollidedEntity = bevy_diesel::events::CollidedEntity<bevy::math::Vec3>;
    pub type CollidedPosition = bevy_diesel::events::CollidedPosition<bevy::math::Vec3>;
    pub type OnSpawnOrigin = bevy_diesel::spawn::OnSpawnOrigin<bevy::math::Vec3>;
    pub type OnSpawnTarget = bevy_diesel::spawn::OnSpawnTarget<bevy::math::Vec3>;
    pub type OnSpawnInvoker = bevy_diesel::spawn::OnSpawnInvoker<bevy::math::Vec3>;
    pub type TargetType = bevy_diesel::target::TargetType<bevy::math::Vec3>;
    pub type TargetGenerator = bevy_diesel::target::TargetGenerator<crate::AvianBackend>;
    pub type TargetMutator = bevy_diesel::target::TargetMutator<crate::AvianBackend>;
    pub type SpawnConfig = bevy_diesel::spawn::SpawnConfig<crate::AvianBackend>;
}

// ---------------------------------------------------------------------------
// AvianContext — backend runtime queries + RNG bundled as a SystemParam
// ---------------------------------------------------------------------------

#[derive(SystemParam)]
pub struct AvianContext<'w, 's> {
    pub spatial_query: SpatialQuery<'w, 's>,
    pub transforms: Query<'w, 's, &'static Transform>,
    rng: Local<'s, SplitMix64>,
}

// ---------------------------------------------------------------------------
// AvianBackend — SpatialBackend implementation
// ---------------------------------------------------------------------------

pub struct AvianBackend;

impl SpatialBackend for AvianBackend {
    type Pos = Vec3;
    type Offset = Vec3Offset;
    type Gatherer = AvianGatherer;
    type Filter = AvianFilter;
    type Context<'w, 's> = AvianContext<'w, 's>;

    fn apply_offset(ctx: &mut AvianContext, pos: Vec3, offset: &Vec3Offset) -> Vec3 {
        pos + apply_vec3_offset(offset, &mut ctx.rng)
    }

    fn distance(a: &Vec3, b: &Vec3) -> f32 {
        a.distance(*b)
    }

    fn position_of(ctx: &AvianContext, entity: Entity) -> Option<Vec3> {
        ctx.transforms.get(entity).ok().map(|t| t.translation)
    }

    fn gather(
        ctx: &mut AvianContext,
        origin: Vec3,
        gatherer: &AvianGatherer,
        exclude: Entity,
    ) -> Vec<bevy_diesel::target::Target<Vec3>> {
        match gatherer {
            // Position generators — read embedded count, produce N points
            AvianGatherer::Sphere { radius, count } => {
                let n = count.resolve(&mut ctx.rng);
                (0..n)
                    .map(|_| {
                        let pos = origin + random_in_sphere(&mut ctx.rng, *radius);
                        bevy_diesel::target::Target::position(pos)
                    })
                    .collect()
            }
            AvianGatherer::Circle { radius, count } => {
                let n = count.resolve(&mut ctx.rng);
                (0..n)
                    .map(|_| {
                        let v = random_in_circle(&mut ctx.rng, *radius);
                        let pos = origin + Vec3::new(v.x, 0.0, v.y);
                        bevy_diesel::target::Target::position(pos)
                    })
                    .collect()
            }
            AvianGatherer::Box {
                half_extents,
                count,
            } => {
                let n = count.resolve(&mut ctx.rng);
                (0..n)
                    .map(|_| {
                        let pos = origin
                            + Vec3::new(
                                rand_f32_range(&mut ctx.rng, -half_extents.x, half_extents.x),
                                rand_f32_range(&mut ctx.rng, -half_extents.y, half_extents.y),
                                rand_f32_range(&mut ctx.rng, -half_extents.z, half_extents.z),
                            );
                        bevy_diesel::target::Target::position(pos)
                    })
                    .collect()
            }
            AvianGatherer::Line {
                direction,
                length,
                count,
            } => {
                let n = count.resolve(&mut ctx.rng);
                let dir = direction.normalize_or_zero();
                (0..n)
                    .map(|_| {
                        let dist = rand_f32_range(&mut ctx.rng, 0.0, *length);
                        let pos = origin + dir * dist;
                        bevy_diesel::target::Target::position(pos)
                    })
                    .collect()
            }

            // Entity gatherers — query avian3d spatial index
            AvianGatherer::EntitiesInSphere(radius)
            | AvianGatherer::EntitiesInCircle(radius)
            | AvianGatherer::AllEntitiesInRadius(radius) => {
                find_entities_in_radius(origin, *radius, &ctx.spatial_query, exclude, &ctx.transforms)
            }
            AvianGatherer::NearestEntities(radius) => {
                let mut targets = find_entities_in_radius(
                    origin,
                    *radius,
                    &ctx.spatial_query,
                    exclude,
                    &ctx.transforms,
                );
                sort_by_distance::<AvianBackend>(&mut targets, &origin);
                targets
            }
        }
    }

    fn apply_filter(
        ctx: &mut AvianContext,
        mut targets: Vec<bevy_diesel::target::Target<Vec3>>,
        filter: &AvianFilter,
        _invoker: Entity,
        _origin: Vec3,
    ) -> Vec<bevy_diesel::target::Target<Vec3>> {
        // TODO: line_of_sight filtering using ctx.spatial_query

        // Count limiting
        if let Some(count) = &filter.count {
            targets = limit_count(targets, count, &mut ctx.rng);
        }

        targets
    }

    fn spawn_transform(
        world_pos: Vec3,
        parent: Option<Entity>,
        q_global_transform: &Query<&GlobalTransform>,
    ) -> Transform {
        if let Some(parent_entity) = parent {
            if let Ok(parent_gt) = q_global_transform.get(parent_entity) {
                let local_pos = parent_gt.affine().inverse().transform_point3(world_pos);
                return Transform::from_translation(local_pos);
            }
        }
        Transform::from_translation(world_pos)
    }

    fn plugin() -> impl Plugin {
        AvianDieselPlugin
    }
}

// ---------------------------------------------------------------------------
// Vec3Offset
// ---------------------------------------------------------------------------

/// Backend-specific offset configuration for 3D space.
#[derive(Clone, Debug)]
pub enum Vec3Offset {
    None,
    Fixed(Vec3),
    RandomBetween(Vec3, Vec3),
    RandomInSphere(f32),
    RandomInCircle(f32),
}

impl Default for Vec3Offset {
    fn default() -> Self {
        Self::None
    }
}

fn apply_vec3_offset(offset: &Vec3Offset, rng: &mut dyn RngCore) -> Vec3 {
    match offset {
        Vec3Offset::None => Vec3::ZERO,
        Vec3Offset::Fixed(v) => *v,
        Vec3Offset::RandomBetween(min, max) => Vec3::new(
            rand_f32_range(rng, min.x, max.x),
            rand_f32_range(rng, min.y, max.y),
            rand_f32_range(rng, min.z, max.z),
        ),
        Vec3Offset::RandomInSphere(radius) => random_in_sphere(rng, *radius),
        Vec3Offset::RandomInCircle(radius) => {
            let v = random_in_circle(rng, *radius);
            Vec3::new(v.x, 0.0, v.y)
        }
    }
}

// ---------------------------------------------------------------------------
// AvianGatherer
// ---------------------------------------------------------------------------

/// Backend-specific gatherer configuration.
///
/// Position generators embed `NumberType` because count is a generation-time parameter.
/// Entity gatherers do not — count limiting is a post-gather filter concern.
#[derive(Clone, Debug)]
pub enum AvianGatherer {
    // Position generators — produce N random points around origin
    Sphere { radius: f32, count: NumberType },
    Circle { radius: f32, count: NumberType },
    Box { half_extents: Vec3, count: NumberType },
    Line {
        direction: Vec3,
        length: f32,
        count: NumberType,
    },

    // Entity gatherers — query avian3d spatial index
    /// All entities in a 3D sphere, unordered.
    EntitiesInSphere(f32),
    /// All entities in an XZ circle, unordered.
    EntitiesInCircle(f32),
    /// Entities in radius, sorted by distance (nearest first). Count cap via filter.
    NearestEntities(f32),
    /// Entities in radius, unsorted. Count cap via reservoir sampling in filter.
    AllEntitiesInRadius(f32),
}

// ---------------------------------------------------------------------------
// AvianFilter
// ---------------------------------------------------------------------------

/// Backend-specific post-gather filter configuration.
#[derive(Clone, Debug)]
pub struct AvianFilter {
    /// Count limit for entity gatherers (diesel utility type).
    pub count: Option<NumberType>,
    /// Backend-specific: require line-of-sight to target.
    pub line_of_sight: bool,
}

impl Default for AvianFilter {
    fn default() -> Self {
        Self {
            count: None,
            line_of_sight: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin — one line to register the generic observer
// ---------------------------------------------------------------------------

/// Avian3d-specific plugin. Adds physics-based actions (projectile, kinematic projectile,
/// velocity effect) and unfiltered collision handling on top of the core diesel infrastructure.
///
/// For faction/team-based collision filtering, also register `CollisionFilterPlugin<F>`.
///
/// # Usage
///
/// ```ignore
/// app.add_plugins(AvianBackend::plugin());
///
/// // With additional filtered collisions:
/// app.add_plugins((
///     AvianBackend::plugin(),
///     CollisionFilterPlugin::<MyTeamFilter>::default(),
/// ));
/// ```
struct AvianDieselPlugin;

impl Plugin for AvianDieselPlugin {
    fn build(&self, app: &mut App) {
        // Core diesel infrastructure (gearbox, repeater, despawn, transitions, etc.)
        app.add_plugins(AvianBackend::plugin_core());

        // Backend-specific observers (require concrete position type or AvianContext)
        app.add_observer(propagate_observer::<AvianBackend>);
        app.add_observer(bevy_diesel::spawn::spawn_observer::<AvianBackend>);
        app.add_observer(bevy_diesel::print::print_effect::<Vec3>);
        app.add_observer(bevy_diesel::gauge::modifiers::modifier_set_observer::<Vec3>);
        app.add_observer(bevy_diesel::gauge::instant::instant_set_observer::<Vec3>);

        // Avian3d-specific actions
        app.add_plugins((
            projectile::ProjectilePlugin,
            velocity::VelocityEffectPlugin,
        ));

        // Collision system (unfiltered — entities with Collides marker)
        collision::plugin(app);
    }
}

// ---------------------------------------------------------------------------
// Concrete Vec3 spawn observer helpers
// ---------------------------------------------------------------------------

/// Forward OnSpawnOrigin<Vec3> to GoOff. Use with `.observe(on_spawn_origin)`.
pub fn on_spawn_origin(ev: On<bevy_diesel::spawn::OnSpawnOrigin<Vec3>>, mut commands: Commands) {
    bevy_diesel::spawn::on_spawn_origin::<Vec3>(ev, commands);
}

/// Forward OnSpawnTarget<Vec3> to GoOff. Use with `.observe(on_spawn_target)`.
pub fn on_spawn_target(ev: On<bevy_diesel::spawn::OnSpawnTarget<Vec3>>, mut commands: Commands) {
    bevy_diesel::spawn::on_spawn_target::<Vec3>(ev, commands);
}

/// Forward OnSpawnInvoker<Vec3> to GoOff. Use with `.observe(on_spawn_invoker)`.
pub fn on_spawn_invoker(ev: On<bevy_diesel::spawn::OnSpawnInvoker<Vec3>>, mut commands: Commands) {
    bevy_diesel::spawn::on_spawn_invoker::<Vec3>(ev, commands);
}

// ---------------------------------------------------------------------------
// Spatial query helpers
// ---------------------------------------------------------------------------

fn find_entities_in_radius(
    origin: Vec3,
    radius: f32,
    spatial_query: &SpatialQuery,
    exclude: Entity,
    q_transform: &Query<&Transform>,
) -> Vec<bevy_diesel::target::Target<Vec3>> {
    let hits = spatial_query.shape_hits(
        &Collider::sphere(radius),
        origin,
        Quat::IDENTITY,
        Dir3::Y,
        100,
        &ShapeCastConfig::default(),
        &SpatialQueryFilter::default(),
    );

    hits.iter()
        .filter_map(|hit| {
            if hit.entity == exclude {
                return None;
            }
            let position = q_transform
                .get(hit.entity)
                .map(|t| t.translation)
                .unwrap_or(origin);
            Some(bevy_diesel::target::Target::entity(hit.entity, position))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// RNG — SplitMix64 for fast, lightweight randomness
// ---------------------------------------------------------------------------

/// Minimal SplitMix64 RNG. Lives in `AvianContext` as a `Local` so that
/// randomness is an implementation detail of this backend, not a core concern.
struct SplitMix64(u64);

impl Default for SplitMix64 {
    fn default() -> Self {
        // Seed from a non-zero constant; Local persists across observer calls
        Self(0xdeadbeefcafe1234)
    }
}

impl RngCore for SplitMix64 {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for chunk in dest.chunks_mut(8) {
            let val = self.next_u64().to_le_bytes();
            let len = chunk.len().min(8);
            chunk[..len].copy_from_slice(&val[..len]);
        }
    }
}

// ---------------------------------------------------------------------------
// RNG helpers
// ---------------------------------------------------------------------------

fn rand_f32(rng: &mut dyn RngCore) -> f32 {
    (rng.next_u32() as f32) / (u32::MAX as f32)
}

fn rand_f32_range(rng: &mut dyn RngCore, min: f32, max: f32) -> f32 {
    min + rand_f32(rng) * (max - min)
}

fn random_in_sphere(rng: &mut dyn RngCore, radius: f32) -> Vec3 {
    loop {
        let x = rand_f32_range(rng, -1.0, 1.0);
        let y = rand_f32_range(rng, -1.0, 1.0);
        let z = rand_f32_range(rng, -1.0, 1.0);
        let v = Vec3::new(x, y, z);
        if v.length_squared() <= 1.0 {
            return v * radius;
        }
    }
}

fn random_in_circle(rng: &mut dyn RngCore, radius: f32) -> Vec2 {
    loop {
        let x = rand_f32_range(rng, -1.0, 1.0);
        let y = rand_f32_range(rng, -1.0, 1.0);
        let v = Vec2::new(x, y);
        if v.length_squared() <= 1.0 {
            return v * radius;
        }
    }
}
