use std::fmt::Debug;

use bevy::prelude::*;
use rand::RngCore;

use crate::backend::SpatialBackend;
use crate::target::Target;

// ---------------------------------------------------------------------------
// Utility types — opt-in building blocks for backends
// ---------------------------------------------------------------------------

/// Team filtering mode. Backends can embed this in their Filter type.
#[derive(Component, Clone, Debug)]
pub enum TeamFilter {
    /// Target any team.
    Both,
    /// Target entities on the same team as the invoker.
    Allies,
    /// Target entities on a different team from the invoker.
    Enemies,
    /// Target entities on a specific team.
    Specific(u32),
}

/// Team marker component. Entities with the same `Team(n)` are allies.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Team(pub u32);

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

/// Filter targets by team affiliation relative to the invoker.
///
/// Position-only targets (entity: None) always pass through.
/// Entities without a team are filtered out.
pub fn filter_by_team<P: Clone + Copy + Send + Sync + Default + Debug + 'static>(
    targets: Vec<Target<P>>,
    invoker_team: u32,
    filter: &TeamFilter,
    team_of: &dyn Fn(Entity) -> Option<u32>,
) -> Vec<Target<P>> {
    targets
        .into_iter()
        .filter(|target| {
            let Some(entity) = target.entity else {
                return true; // position-only targets pass through
            };

            let Some(entity_team) = team_of(entity) else {
                return false; // entities without teams are filtered out
            };

            match filter {
                TeamFilter::Both => true,
                TeamFilter::Allies => invoker_team == entity_team,
                TeamFilter::Enemies => invoker_team != entity_team,
                TeamFilter::Specific(team_id) => entity_team == *team_id,
            }
        })
        .collect()
}

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
