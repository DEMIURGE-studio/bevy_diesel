use std::fmt::Debug;

use bevy::prelude::*;
use rand::RngCore;

use crate::backend::SpatialBackend;
use crate::target::Target;

// ---------------------------------------------------------------------------
// CollisionFilter — generic trait for collision/target filtering
// ---------------------------------------------------------------------------

/// A generic filter for determining whether an ability can affect a target entity.
///
/// Backends use this to filter collisions and target resolution. Users implement
/// this trait for their own faction/alliance/team system.
///
/// - `Self` is the filter component placed on ability entities (e.g. your `TeamFilter`)
/// - `Self::Lookup` is the component queried on invoker/target entities (e.g. your `Team`)
///
/// # Example
///
/// ```ignore
/// #[derive(Component, Clone, Debug)]
/// pub enum Faction { Allies, Enemies, Both }
///
/// #[derive(Component, Clone, Copy)]
/// pub struct Alliance(pub u32);
///
/// impl CollisionFilter for Faction {
///     type Lookup = Alliance;
///     fn can_target(&self, invoker: Option<&Alliance>, target: Option<&Alliance>) -> bool {
///         match (self, invoker, target) {
///             (Faction::Both, _, _) => true,
///             (Faction::Allies, Some(i), Some(t)) => i.0 == t.0,
///             (Faction::Enemies, Some(i), Some(t)) => i.0 != t.0,
///             _ => true,
///         }
///     }
/// }
/// ```
pub trait CollisionFilter: Component + Clone + Debug + Send + Sync + 'static {
    /// The component to look up on invoker and target entities.
    type Lookup: Component;

    /// Return `true` if the ability should affect this target.
    ///
    /// `invoker_data` is the `Lookup` component on the root invoker entity.
    /// `target_data` is the `Lookup` component on the potential target entity.
    /// Either may be `None` if the entity doesn't have the component.
    fn can_target(
        &self,
        invoker_data: Option<&Self::Lookup>,
        target_data: Option<&Self::Lookup>,
    ) -> bool;
}

/// Simple marker component that opts an entity into collision events
/// without any filtering. Every collision fires an event.
///
/// For filtered collisions (faction/team-based), implement the `CollisionFilter`
/// trait and register `CollisionFilterPlugin<F>` in your app.
#[derive(Component, Clone, Debug, Default)]
pub struct Collides;

// ---------------------------------------------------------------------------
// Utility types
// ---------------------------------------------------------------------------

/// Count specification: fixed or random range.
/// Used by position-generator Gatherer variants (embedded count) and
/// by filter logic (post-gather capping).
#[derive(Clone, Debug)]
pub enum NumberType {
    Fixed(usize),
    Random(usize, usize),
}

impl NumberType {
    /// Resolve to a concrete count.
    pub fn resolve(&self, rng: &mut dyn RngCore) -> usize {
        match self {
            NumberType::Fixed(n) => *n,
            NumberType::Random(min, max) => {
                if min >= max {
                    return *min;
                }
                let range = max - min + 1;
                let r = (rng.next_u64() as usize) % range;
                min + r
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Utility functions — backends call these in their filter logic
// ---------------------------------------------------------------------------

/// Limit the number of targets using reservoir sampling for random selection.
pub fn limit_count<P: Clone + Copy + Send + Sync + Default + Debug + 'static>(
    targets: Vec<Target<P>>,
    number: &NumberType,
    rng: &mut dyn RngCore,
) -> Vec<Target<P>> {
    let max_count = number.resolve(rng);

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

/// Sort targets by distance to an origin point (nearest first).
pub fn sort_by_distance<B: SpatialBackend>(
    targets: &mut [Target<B::Pos>],
    origin: &B::Pos,
) {
    targets.sort_by(|a, b| {
        let dist_a = B::distance(&a.position, origin);
        let dist_b = B::distance(&b.position, origin);
        dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
    });
}
