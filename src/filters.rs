use std::fmt::Debug;

use bevy::prelude::*;
use rand::RngCore;

use crate::backend::SpatialBackend;
use crate::target::Target;

// ---------------------------------------------------------------------------
// CollisionFilter - generic trait for collision/target filtering
// ---------------------------------------------------------------------------

/// Determines whether an ability can affect a target entity.
///
/// `Self` goes on the ability entity, `Self::Lookup` is queried on invoker/target.
///
/// ```ignore
/// impl CollisionFilter for Faction {
///     type Lookup = Alliance;
///     fn can_target(&self, invoker: Option<&Alliance>, target: Option<&Alliance>) -> bool {
///         match (self, invoker, target) {
///             (Faction::Enemies, Some(i), Some(t)) => i.0 != t.0,
///             _ => true,
///         }
///     }
/// }
/// ```
pub trait CollisionFilter: Component + Clone + Debug + Send + Sync + 'static {
    /// Component queried on invoker and target entities.
    type Lookup: Component;

    /// Return `true` if the ability should affect this target.
    fn can_target(
        &self,
        invoker_data: Option<&Self::Lookup>,
        target_data: Option<&Self::Lookup>,
    ) -> bool;
}

/// Marker: every collision fires an event, no filtering.
#[derive(Component, Clone, Debug, Default)]
pub struct Collides;

// ---------------------------------------------------------------------------
// Utility types
// ---------------------------------------------------------------------------

/// Count: fixed or random range.
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
// Utility functions - backends call these in their filter logic
// ---------------------------------------------------------------------------

/// Limit targets via reservoir sampling.
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

/// Sort targets by distance (nearest first).
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
