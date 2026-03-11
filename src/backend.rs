use std::fmt::Debug;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::target::Target;

/// A type-level configuration for spatial representation.
///
/// Backends implement this trait on a unit struct (e.g. `AvianBackend`) and define
/// the position, offset, gatherer, and filter types. Users interact with backends
/// through type aliases exported from the backend's prelude.
///
/// The `Context` GAT bundles the backend's runtime queries (transforms, spatial queries,
/// team lookups, RNG, etc.) into a single `SystemParam`. Diesel's generic propagation
/// observer extracts it automatically — backend authors never write observer boilerplate.
///
/// Methods receive `&mut Context` so backends can include mutable state (e.g. RNG)
/// in their context without requiring interior mutability.
pub trait SpatialBackend: Send + Sync + 'static {
    /// The position representation (Vec3, Vec2, IVec2, usize, etc.)
    type Pos: Clone + Copy + Send + Sync + Default + Debug + bevy::reflect::TypePath + 'static;

    /// Backend-specific offset configuration (e.g. `Vec3Offset`, `GridOffset`)
    type Offset: Clone + Send + Sync + Default + Debug + 'static;

    /// Backend-specific gatherer configuration (e.g. `AvianGatherer`, `GridGatherer`).
    /// Does NOT require Default — lives inside `Option<B::Gatherer>` on TargetGenerator.
    type Gatherer: Clone + Send + Sync + Debug + 'static;

    /// Backend-specific post-gather filter configuration (e.g. `AvianFilter`)
    type Filter: Clone + Send + Sync + Default + Debug + 'static;

    /// Backend-specific runtime context bundling all needed system params.
    /// Diesel's generic propagation observer extracts this automatically.
    ///
    /// Include any mutable state (RNG, caches, etc.) here — trait methods
    /// receive `&mut Context`.
    ///
    /// Example:
    /// ```ignore
    /// #[derive(SystemParam)]
    /// pub struct AvianContext<'w, 's> {
    ///     pub spatial_query: SpatialQuery<'w, 's>,
    ///     pub transforms: Query<'w, 's, &'static Transform>,
    ///     pub teams: Query<'w, 's, &'static Team>,
    ///     rng: Local<'s, MyRng>,
    /// }
    /// ```
    type Context<'w, 's>: SystemParam;

    /// Apply an offset to a position. Called during pipeline stage 2.
    fn apply_offset(ctx: &mut Self::Context<'_, '_>, pos: Self::Pos, offset: &Self::Offset) -> Self::Pos;

    /// Calculate the distance between two positions.
    fn distance(a: &Self::Pos, b: &Self::Pos) -> f32;

    /// Look up an entity's position. Called during pipeline stage 1 (resolve).
    fn position_of(ctx: &Self::Context<'_, '_>, entity: Entity) -> Option<Self::Pos>;

    /// Gather targets from a position. Called during pipeline stage 3 when
    /// `gatherer` is `Some`. Handles both position generation and entity querying —
    /// the core doesn't distinguish between them.
    fn gather(
        ctx: &mut Self::Context<'_, '_>,
        origin: Self::Pos,
        gatherer: &Self::Gatherer,
        exclude: Entity,
    ) -> Vec<Target<Self::Pos>>;

    /// Apply post-gather filtering. Called during pipeline stage 4.
    /// Backends compose diesel utility functions (filter_by_team, limit_count, etc.)
    /// with any custom filters (line-of-sight, priority scoring, etc.).
    fn apply_filter(
        ctx: &mut Self::Context<'_, '_>,
        targets: Vec<Target<Self::Pos>>,
        filter: &Self::Filter,
        invoker: Entity,
        origin: Self::Pos,
    ) -> Vec<Target<Self::Pos>>;

    /// Compute the `Transform` to use when spawning an entity at `world_pos`.
    /// If `parent` is provided, returns a local-space transform relative to that entity.
    fn spawn_transform(
        world_pos: Self::Pos,
        parent: Option<Entity>,
        q_global_transform: &Query<&GlobalTransform>,
    ) -> Transform;
}
