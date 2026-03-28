use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_gearbox::RegistrationAppExt;

use crate::events::{CollidedEntity, CollidedPosition, OnRepeat, StartInvoke};
use crate::gearbox::repeater;
use crate::spawn::{OnSpawnInvoker, OnSpawnOrigin, OnSpawnTarget};
use crate::target::Target;

/// A type-level configuration for spatial representation.
///
/// Backends implement this trait on a unit struct (e.g. `AvianBackend`) and define
/// the position, offset, gatherer, and filter types. Users interact with backends
/// through type aliases exported from the backend's prelude.
///
/// The `Context` GAT bundles the backend's runtime queries (transforms, spatial queries,
/// faction lookups, RNG, etc.) into a single `SystemParam`. Diesel's generic propagation
/// observer extracts it automatically — backend authors never write observer boilerplate.
///
/// # Plugin methods
///
/// `plugin_core()` returns a `DieselCorePlugin` that registers all diesel infrastructure
/// for the backend's position type. `plugin()` defaults to calling `plugin_core()` but
/// can be overridden to add backend-specific systems while still including the core.
///
/// ```ignore
/// // Simple — just the core
/// app.add_plugins(MyBackend::plugin());
///
/// // Backend overrides plugin() to add custom systems
/// impl SpatialBackend for MyBackend {
///     fn plugin() -> impl Plugin {
///         MyBackendPlugin  // calls Self::plugin_core() internally
///     }
/// }
/// ```
pub trait SpatialBackend: Send + Sync + 'static {
    /// The position representation (Vec3, Vec2, IVec2, usize, etc.)
    type Pos: Clone + Copy + Send + Sync + Default + Debug + bevy::reflect::TypePath + bevy::reflect::Reflect + 'static;

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
    /// Backends compose diesel utility functions (limit_count, sort_by_distance, etc.)
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

    /// Returns the core diesel plugin that registers all generic infrastructure
    /// for this backend's position type: state machines, repeaters, despawn,
    /// propagation, spawning, and transition events.
    fn plugin_core() -> DieselCorePlugin<Self>
    where
        Self: Sized,
    {
        DieselCorePlugin {
            _marker: PhantomData,
        }
    }

    /// Returns the full plugin for this backend. Override to add backend-specific
    /// systems (physics, collision, projectiles, etc.).
    ///
    /// The default implementation just returns `plugin_core()`. Override it and
    /// call `Self::plugin_core()` within your custom plugin to get the defaults
    /// plus your additions.
    fn plugin() -> impl Plugin
    where
        Self: Sized,
    {
        Self::plugin_core()
    }
}

// ---------------------------------------------------------------------------
// DieselCorePlugin<B> — registers all generic diesel infrastructure
// ---------------------------------------------------------------------------

/// Core plugin that registers diesel infrastructure for a specific `SpatialBackend`.
///
/// Registered automatically by `SpatialBackend::plugin_core()`. Includes everything
/// that doesn't require the backend's `Context` GAT:
/// - `GearboxPlugin` (state machine core)
/// - `TemplateRegistry` resource
/// - Repeater system for `OnRepeat<B::Pos>`
/// - Despawn system
/// - Transition registration for all generic events
/// - Propagation plugin (inventory-based event forwarding)
///
/// **Backend-specific observers** (`propagate_observer`, `spawn_observer`, `print_effect`)
/// must be registered by the backend's `plugin()` override since they require the
/// backend's `Context` system param.
pub struct DieselCorePlugin<B: SpatialBackend> {
    _marker: PhantomData<B>,
}

impl<B: SpatialBackend> Plugin for DieselCorePlugin<B> {
    fn build(&self, app: &mut App) {
        // State machine core
        app.add_plugins(bevy_gearbox::GearboxPlugin);

        // Attribute system + PAE (persistent attribute effects with stat-gated guards)
        app.add_plugins(bevy_gauge::plugin::AttributesPlugin);
        app.add_plugins(crate::gauge::pae::DieselPaePlugin);

        // Template registry
        app.init_resource::<crate::spawn::TemplateRegistry>();

        // Repeater (hardcoded to OnRepeat<B::Pos>)
        app.add_observer(repeater::repeater_observer::<OnRepeat<B::Pos>>);
        app.add_observer(repeater::reset_repeater);
        app.register_type::<repeater::Repeater>();

        // Despawn
        app.add_observer(crate::despawn::queue_despawn_observer::<B::Pos>);
        app.add_systems(PostUpdate, crate::despawn::despawn_queue_system);
        app.register_state_component::<crate::despawn::DelayedDespawn>();

        // Register transition events
        app.register_transition::<StartInvoke<B::Pos>>();
        app.register_transition::<OnRepeat<B::Pos>>();
        app.register_transition::<CollidedEntity<B::Pos>>();
        app.register_transition::<CollidedPosition<B::Pos>>();
        app.register_transition::<OnSpawnOrigin<B::Pos>>();
        app.register_transition::<OnSpawnTarget<B::Pos>>();
        app.register_transition::<OnSpawnInvoker<B::Pos>>();

        // Propagation (inventory-based event forwarding)
        crate::propagation::plugin(app);
    }
}
