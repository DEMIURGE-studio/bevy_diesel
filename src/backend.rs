use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_gearbox::RegistrationAppExt;

use crate::events::{CollidedEntity, CollidedPosition, OnRepeat, StartInvoke};
use crate::gearbox::repeater;
use crate::spawn::{OnSpawnInvoker, OnSpawnOrigin, OnSpawnTarget};
use crate::target::Target;

/// Defines how diesel interacts with a game's spatial representation.
///
/// Implement on a unit struct (e.g. `AvianBackend`) to provide position, offset,
/// gatherer, and filter types. The `Context` GAT bundles runtime queries
/// (transforms, spatial index, RNG, etc.) into a single `SystemParam`.
///
/// ```ignore
/// app.add_plugins(MyBackend::plugin());
/// ```
pub trait SpatialBackend: Send + Sync + 'static {
    /// Position type (Vec3, Vec2, IVec2, etc.)
    type Pos: Clone
        + Copy
        + Send
        + Sync
        + Default
        + Debug
        + bevy::reflect::TypePath
        + bevy::reflect::Reflect
        + 'static;

    /// Offset type (e.g. `Vec3Offset`, `GridOffset`)
    type Offset: Clone + Send + Sync + Default + Debug + 'static;

    /// Gatherer type (e.g. `AvianGatherer`). No Default bound - wrapped in `Option`.
    type Gatherer: Clone + Send + Sync + Debug + 'static;

    /// Post-gather filter type (e.g. `AvianFilter`)
    type Filter: Clone + Send + Sync + Default + Debug + 'static;

    /// Runtime context bundled as a `SystemParam` (spatial queries, transforms, RNG, etc.).
    type Context<'w, 's>: SystemParam;

    /// Apply an offset to a position.
    fn apply_offset(
        ctx: &mut Self::Context<'_, '_>,
        pos: Self::Pos,
        offset: &Self::Offset,
    ) -> Self::Pos;

    /// Distance between two positions.
    fn distance(a: &Self::Pos, b: &Self::Pos) -> f32;

    /// Look up an entity's position.
    fn position_of(ctx: &Self::Context<'_, '_>, entity: Entity) -> Option<Self::Pos>;

    /// Gather targets around a position.
    fn gather(
        ctx: &mut Self::Context<'_, '_>,
        origin: Self::Pos,
        gatherer: &Self::Gatherer,
        exclude: Entity,
    ) -> Vec<Target<Self::Pos>>;

    /// Filter gathered targets (count limits, line-of-sight, etc.).
    fn apply_filter(
        ctx: &mut Self::Context<'_, '_>,
        targets: Vec<Target<Self::Pos>>,
        filter: &Self::Filter,
        invoker: Entity,
        origin: Self::Pos,
    ) -> Vec<Target<Self::Pos>>;

    /// Compute a `Transform` for spawning at `world_pos`, local to `parent` if given.
    fn spawn_transform(
        world_pos: Self::Pos,
        parent: Option<Entity>,
        q_global_transform: &Query<&GlobalTransform>,
    ) -> Transform;

    /// Core plugin: state machines, repeaters, despawn, transitions, propagation.
    fn plugin_core() -> DieselCorePlugin<Self>
    where
        Self: Sized,
    {
        DieselCorePlugin {
            _marker: PhantomData,
        }
    }

    /// Full backend plugin. Override to add backend-specific systems on top of `plugin_core()`.
    fn plugin() -> impl Plugin
    where
        Self: Sized,
    {
        Self::plugin_core()
    }
}

// ---------------------------------------------------------------------------
// DieselCorePlugin<B> - registers all generic diesel infrastructure
// ---------------------------------------------------------------------------

/// Registers generic diesel infrastructure for a `SpatialBackend`.
///
/// Backend-specific observers (`propagate_observer`, `spawn_observer`, etc.)
/// must be registered by the backend's `plugin()` override due to the Context GAT.
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
