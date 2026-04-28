use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::RngCore;

use bevy_diesel::prelude::*;
use bevy_diesel::bevy_gauge::AttributeResolvable;

// Re-exports

pub use bevy_diesel;

pub mod ballistics;
pub mod collision;
pub mod impulse;
pub mod projectile;
pub mod velocity;

pub mod prelude {
    pub use crate::ballistics::{
        calculate_high_angle_velocity_with_speed, calculate_low_angle_velocity_with_speed,
        calculate_velocity_with_speed, distance_lock,
    };
    pub use crate::collision::{CollisionFilter, CollisionFilterPlugin, CollisionLayerFilter, Collides};
    pub use crate::projectile::{
        LinearProjectile, LinearProjectileEffect, ProjectileEffect, ProjectilePlugin,
    };
    pub use crate::velocity::{Trajectory, VelocityEffect, VelocityEffectPlugin};
    pub use crate::impulse::ImpulseEffect;
    pub use crate::{AvianBackend, AvianFilter, AvianGatherer, NumberType, Vec3Offset};
    pub use bevy_diesel::prelude::*;

    // Vec3 type aliases
    pub type InvokerTarget = bevy_diesel::target::InvokerTarget<bevy::math::Vec3>;
    pub type Target = bevy_diesel::target::Target<bevy::math::Vec3>;
    pub type GoOff = bevy_diesel::effect::GoOff<bevy::math::Vec3>;
    pub type StartInvoke = bevy_diesel::events::StartInvoke<bevy::math::Vec3>;
    pub type StopInvoke = bevy_diesel::events::StopInvoke<bevy::math::Vec3>;
    pub type OnRepeat = bevy_diesel::events::OnRepeat<bevy::math::Vec3>;
    pub use crate::collision::{CollidedEntity, CollidedPosition};
    pub type OnSpawnOrigin = bevy_diesel::spawn::OnSpawnOrigin<bevy::math::Vec3>;
    pub type OnSpawnTarget = bevy_diesel::spawn::OnSpawnTarget<bevy::math::Vec3>;
    pub type OnSpawnInvoker = bevy_diesel::spawn::OnSpawnInvoker<bevy::math::Vec3>;
    pub type TargetType = bevy_diesel::target::TargetType<bevy::math::Vec3>;
    pub type TargetGenerator = bevy_diesel::target::TargetGenerator<crate::AvianBackend>;
    pub type TargetMutator = bevy_diesel::target::TargetMutator<crate::AvianBackend>;
    pub type SpawnConfig = bevy_diesel::spawn::SpawnConfig<crate::AvianBackend>;
    pub type GoOffConfig = bevy_diesel::effect::GoOffConfig<crate::AvianBackend>;

    // Vec3-concrete template wrappers (shadow the generic versions from bevy_diesel::prelude)

    /// Outermost ability wrapper: Ready → Invoking → Cooldown, with Vec3 positions.
    pub fn template_invoked<F>(
        commands: &mut bevy::prelude::Commands,
        entity: Option<bevy::prelude::Entity>,
        cooldown: std::time::Duration,
        configure_invoking: F,
    ) -> bevy::prelude::Entity
    where
        F: FnOnce(&mut bevy::prelude::EntityCommands),
    {
        bevy_diesel::gearbox::templates::template_invoked::<bevy::math::Vec3, F>(
            commands, entity, cooldown, configure_invoking,
        )
    }

    /// Single-shot fire → done state with the avian backend.
    pub fn template_single_shot<F>(
        on_fire: F,
    ) -> impl FnOnce(&mut bevy::prelude::EntityCommands)
    where
        F: FnOnce(&mut bevy::prelude::EntityCommands),
    {
        bevy_diesel::gearbox::templates::template_single_shot::<crate::AvianBackend, F>(on_fire)
    }

    /// Counted volley sub-machine with Vec3 positions.
    pub fn template_repeater<F>(
        count_expr: &str,
        delay_secs: f32,
        on_tick: F,
    ) -> impl FnOnce(&mut bevy::prelude::EntityCommands)
    where
        F: FnOnce(&mut bevy::prelude::EntityCommands),
    {
        bevy_diesel::gearbox::templates::template_repeater::<bevy::math::Vec3, F>(
            count_expr, delay_secs, on_tick,
        )
    }
}

// ---------------------------------------------------------------------------
// AvianContext - backend runtime queries + RNG bundled as a SystemParam
// ---------------------------------------------------------------------------

#[derive(SystemParam)]
pub struct AvianContext<'w, 's> {
    pub spatial_query: SpatialQuery<'w, 's>,
    pub transforms: Query<'w, 's, &'static Transform>,
    global_transforms: Query<'w, 's, &'static GlobalTransform>,
    rng: Local<'s, SplitMix64>,
}

// ---------------------------------------------------------------------------
// AvianBackend - SpatialBackend implementation
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
    ) -> Vec<(bevy_diesel::target::Target<Vec3>, bevy_diesel::target::Scope)> {
        match gatherer {
            // Position generators - read embedded count, produce N points
            AvianGatherer::Sphere { radius, count } => {
                let n = count.resolve_count(&mut ctx.rng);
                let total = n as f32;
                (0..n)
                    .map(|i| {
                        let offset = random_in_sphere(&mut ctx.rng, *radius);
                        let pos = origin + offset;
                        let scope = vec![
                            ("Distance@scope", offset.length()),
                            ("Radius@scope", *radius),
                            ("Rank@scope", i as f32),
                            ("GatherCount@scope", total),
                        ];
                        (bevy_diesel::target::Target::position(pos), scope)
                    })
                    .collect()
            }
            AvianGatherer::Circle { radius, count } => {
                let n = count.resolve_count(&mut ctx.rng);
                let total = n as f32;
                (0..n)
                    .map(|i| {
                        let v = random_in_circle(&mut ctx.rng, *radius);
                        let offset = Vec3::new(v.x, 0.0, v.y);
                        let pos = origin + offset;
                        let scope = vec![
                            ("Distance@scope", offset.length()),
                            ("Radius@scope", *radius),
                            ("Rank@scope", i as f32),
                            ("GatherCount@scope", total),
                        ];
                        (bevy_diesel::target::Target::position(pos), scope)
                    })
                    .collect()
            }
            AvianGatherer::Box {
                half_extents,
                count,
            } => {
                let n = count.resolve_count(&mut ctx.rng);
                let total = n as f32;
                (0..n)
                    .map(|i| {
                        let offset = Vec3::new(
                            rand_f32_range(&mut ctx.rng, -half_extents.x, half_extents.x),
                            rand_f32_range(&mut ctx.rng, -half_extents.y, half_extents.y),
                            rand_f32_range(&mut ctx.rng, -half_extents.z, half_extents.z),
                        );
                        let pos = origin + offset;
                        let scope = vec![
                            ("Distance@scope", offset.length()),
                            ("Rank@scope", i as f32),
                            ("GatherCount@scope", total),
                        ];
                        (bevy_diesel::target::Target::position(pos), scope)
                    })
                    .collect()
            }
            AvianGatherer::Line {
                direction,
                length,
                count,
            } => {
                let n = count.resolve_count(&mut ctx.rng);
                let total = n as f32;
                let dir = direction.normalize_or_zero();
                (0..n)
                    .map(|i| {
                        let dist = rand_f32_range(&mut ctx.rng, 0.0, *length);
                        let pos = origin + dir * dist;
                        let scope = vec![
                            ("Distance@scope", dist),
                            ("Length@scope", *length),
                            ("Rank@scope", i as f32),
                            ("GatherCount@scope", total),
                        ];
                        (bevy_diesel::target::Target::position(pos), scope)
                    })
                    .collect()
            }

            // Entity gatherers - query avian3d spatial index
            AvianGatherer::EntitiesInSphere(radius)
            | AvianGatherer::EntitiesInCircle(radius)
            | AvianGatherer::AllEntitiesInRadius(radius) => find_entities_in_radius(
                origin,
                *radius,
                &ctx.spatial_query,
                exclude,
                &ctx.transforms,
            ),
            AvianGatherer::NearestEntities(radius) => {
                let mut targets = find_entities_in_radius(
                    origin,
                    *radius,
                    &ctx.spatial_query,
                    exclude,
                    &ctx.transforms,
                );
                sort_by_distance(&mut targets, &origin);
                // Rewrite rank now that order is stable.
                let total = targets.len() as f32;
                for (i, (_, scope)) in targets.iter_mut().enumerate() {
                    for (key, val) in scope.iter_mut() {
                        match *key {
                            "Rank@scope" => *val = i as f32,
                            "GatherCount@scope" => *val = total,
                            _ => {}
                        }

                    }
                }
                targets
            }
        }
    }

    fn apply_filter(
        ctx: &mut AvianContext,
        targets: Vec<(bevy_diesel::target::Target<Vec3>, bevy_diesel::target::Scope)>,
        filter: &AvianFilter,
        _invoker: Entity,
        _origin: Vec3,
    ) -> Vec<(bevy_diesel::target::Target<Vec3>, bevy_diesel::target::Scope)> {
        // TODO: line_of_sight filtering using ctx.spatial_query

        // Count limiting
        limit_count(targets, &filter.count, &mut ctx.rng)
    }

    fn insert_position(
        commands: &mut EntityCommands,
        ctx: &AvianContext,
        world_pos: Vec3,
        parent: Option<Entity>,
    ) {
        let transform = if let Some(parent_entity) = parent {
            if let Ok(parent_gt) = ctx.global_transforms.get(parent_entity) {
                let local_pos = parent_gt.affine().inverse().transform_point3(world_pos);
                Transform::from_translation(local_pos)
            } else {
                Transform::from_translation(world_pos)
            }
        } else {
            Transform::from_translation(world_pos)
        };
        commands.insert(transform);
    }

    fn plugin() -> impl Plugin {
        AvianDieselPlugin
    }
}

// ---------------------------------------------------------------------------
// Vec3Offset
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, AttributeResolvable)]
pub struct DirectionOffset {
    #[skip]
    pub direction: Dir3,
    pub magnitude: f32,
}

impl DirectionOffset {
    pub fn new(dir: Dir3, magnitude: f32) -> DirectionOffset {
        DirectionOffset {
            direction: dir,
            magnitude,
        }
    }
}

/// Offset configuration for 3D space.
#[derive(Clone, Debug, AttributeResolvable)]
pub enum Vec3Offset {
    None,
    Fixed(DirectionOffset),
    RandomBetween {
        min: DirectionOffset,
        max: DirectionOffset,
    },
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
        Vec3Offset::Fixed(offset) => *offset.direction * offset.magnitude,
        Vec3Offset::RandomBetween { min, max } => {
            let t = rand_f32(rng);
            let magnitude = min.magnitude + t * (max.magnitude - min.magnitude);
            let dir = min.direction.slerp(max.direction, t);
            *dir * magnitude
        }
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

/// Gatherer variants for 3D space.
/// Position generators embed count; entity gatherers defer count limiting to filters.
#[derive(Clone, Debug, AttributeResolvable)]
pub enum AvianGatherer {
    // Position generators - produce N random points around origin
    Sphere {
        radius: f32,
        count: NumberType,
    },
    Circle {
        radius: f32,
        count: NumberType,
    },
    Box {
        #[skip]
        half_extents: Vec3,
        count: NumberType,
    },
    Line {
        #[skip]
        direction: Vec3,
        length: f32,
        count: NumberType,
    },

    // Entity gatherers - query avian3d spatial index
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

/// Post-gather filter config.
#[derive(Clone, Debug, AttributeResolvable)]
pub struct AvianFilter {
    /// Max target count. `NumberType::All` passes everything through.
    pub count: NumberType,
    /// Require line-of-sight (TODO).
    #[skip]
    pub line_of_sight: bool,
}

impl Default for AvianFilter {
    fn default() -> Self {
        Self {
            count: NumberType::All,
            line_of_sight: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------
struct AvianDieselPlugin;

impl Plugin for AvianDieselPlugin {
    fn build(&self, app: &mut App) {
        // Core diesel infrastructure (gearbox, repeater, despawn, transitions, etc.)
        app.add_plugins(AvianBackend::plugin_core());

        // Register AttributeDerived for concrete AvianBackend types
        use bevy_diesel::bevy_gauge::prelude::AttributesAppExt;
        app.register_attribute_derived::<bevy_diesel::spawn::SpawnConfig<AvianBackend>>();
        app.register_attribute_derived::<bevy_diesel::target::TargetMutator<AvianBackend>>();

        // Propagation: reads GoOffOrigin, writes GoOff
        app.add_systems(bevy_diesel::bevy_gearbox::GearboxSchedule,
            (
                bevy_diesel::effect::go_off_on_entry::<AvianBackend>,
                propagate_observer::<AvianBackend>,
            )
                .chain()
                .in_set(bevy_diesel::DieselSet::Propagation),
        );

        // Leaf effect systems: read GoOff
        app.add_systems(bevy_diesel::bevy_gearbox::GearboxSchedule, (
            bevy_diesel::spawn::spawn_system::<AvianBackend>,
            bevy_diesel::print::print_effect::<Vec3>,
            impulse::impulse_effect_system,
        ).in_set(bevy_diesel::DieselSet::Effects));

        // Sustained modifier apply — monomorphized here because the generic
        // fn needs B::Context which can only resolve with a concrete backend.
        app.add_systems(
            Update,
            bevy_diesel::gauge::modifiers::sustained_modifier_apply::<AvianBackend>
                .in_set(bevy_diesel::gauge::SustainedModifierSet),
        );

        // Avian3d-specific actions
        app.add_plugins((projectile::ProjectilePlugin, velocity::VelocityEffectPlugin));

        // Collision types + system (unfiltered - entities with Collides marker)
        app.register_transition::<collision::CollidedEntity>();
        app.register_transition::<collision::CollidedPosition>();
        app.add_systems(bevy_diesel::bevy_gearbox::GearboxSchedule, (
            bevy_diesel::events::go_off_side_effect::<collision::CollidedEntity, Vec3>
                .in_set(bevy_diesel::bevy_gearbox::GearboxPhase::SideEffectPhase),
            bevy_diesel::events::go_off_side_effect::<collision::CollidedPosition, Vec3>
                .in_set(bevy_diesel::bevy_gearbox::GearboxPhase::SideEffectPhase),
        ));
        collision::plugin(app);
    }
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
) -> Vec<(bevy_diesel::target::Target<Vec3>, bevy_diesel::target::Scope)> {
    let hits = spatial_query.shape_hits(
        &Collider::sphere(radius),
        origin,
        Quat::IDENTITY,
        Dir3::Y,
        100,
        &ShapeCastConfig::default(),
        &SpatialQueryFilter::default(),
    );

    let mut out: Vec<(bevy_diesel::target::Target<Vec3>, bevy_diesel::target::Scope)> = hits
        .iter()
        .filter_map(|hit| {
            if hit.entity == exclude {
                return None;
            }
            let position = q_transform
                .get(hit.entity)
                .map(|t| t.translation)
                .unwrap_or(origin);
            let distance = position.distance(origin);
            Some((
                bevy_diesel::target::Target::entity(hit.entity, position),
                vec![
                    ("Distance@scope", distance),
                    ("Radius@scope", radius),
                    ("Rank@scope", 0.0),
                    ("GatherCount@scope", 0.0),
                ],
            ))
        })
        .collect();

    // Fill rank + count now that the final set is known.
    let total = out.len() as f32;
    for (i, (_, scope)) in out.iter_mut().enumerate() {
        for (key, val) in scope.iter_mut() {
            match *key {
                "Rank@scope" => *val = i as f32,
                "GatherCount@scope" => *val = total,
                _ => {}
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// RNG - SplitMix64 for fast, lightweight randomness
// ---------------------------------------------------------------------------

/// SplitMix64 RNG, stored as a `Local` in `AvianContext`.
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

// ---------------------------------------------------------------------------
// NumberType - count: fixed, random range, or unlimited
// ---------------------------------------------------------------------------

/// Count specification: fixed, random range, or all (no limit).
#[derive(Clone, Debug, AttributeResolvable)]
pub enum NumberType {
    All,
    Fixed(usize),
    Random { min: usize, max: usize },
}

impl Default for NumberType {
    fn default() -> Self {
        Self::All
    }
}

impl NumberType {
    /// Resolve to a concrete count. Panics on `All`. Use only for gatherers
    /// where a count is always required.
    pub fn resolve_count(&self, rng: &mut dyn RngCore) -> usize {
        match self {
            NumberType::All => panic!("NumberType::All has no concrete count"),
            NumberType::Fixed(n) => *n,
            NumberType::Random { min, max } => {
                if min >= max {
                    return *min;
                }
                let range = max - min + 1;
                let r = (rng.next_u64() as usize) % range;
                min + r
            }
        }
    }

    /// Resolve to a concrete count, or `None` for unlimited.
    fn resolve_limit(&self, rng: &mut dyn RngCore) -> Option<usize> {
        match self {
            NumberType::All => None,
            _ => Some(self.resolve_count(rng)),
        }
    }
}

// ---------------------------------------------------------------------------
// Filter utilities
// ---------------------------------------------------------------------------

/// Limit targets via reservoir sampling.
fn limit_count(
    targets: Vec<(bevy_diesel::target::Target<Vec3>, bevy_diesel::target::Scope)>,
    number: &NumberType,
    rng: &mut dyn RngCore,
) -> Vec<(bevy_diesel::target::Target<Vec3>, bevy_diesel::target::Scope)> {
    let max_count = match number.resolve_limit(rng) {
        Some(n) => n,
        None => return targets,
    };

    if targets.len() <= max_count {
        return targets;
    }

    // Reservoir sampling
    let mut selected = Vec::with_capacity(max_count);
    for (i, target) in targets.into_iter().enumerate() {
        if selected.len() < max_count {
            selected.push(target);
        } else {
            let r = (rng.next_u64() as usize) % (i + 1);
            if r < max_count {
                selected[r] = target;
            }
        }
    }
    selected
}

/// Sort targets by distance (nearest first).
fn sort_by_distance(
    targets: &mut [(bevy_diesel::target::Target<Vec3>, bevy_diesel::target::Scope)],
    origin: &Vec3,
) {
    targets.sort_by(|(a, _), (b, _)| {
        let dist_a = a.position.distance(*origin);
        let dist_b = b.position.distance(*origin);
        dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
    });
}
