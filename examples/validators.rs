//! Example: Composable Event Validators
//!
//! Demonstrates how to build a composable condition system for filtering
//! when effects should trigger. For example, a "thorns" effect might only
//! trigger when the defender is within 5 units of the attacker and the
//! damage type is physical.
//!
//! This uses bevy_gearbox's `EventValidator` trait to gate state machine
//! transitions on runtime conditions.
//!
//! This is a PATTERN — define your own conditions and compose them.

use bevy::prelude::*;
use bevy_gearbox::transitions::EventValidator;

// ============================================================================
// Step 1: Define traits for events that carry role/damage information
// ============================================================================

/// Events that have spatial information about two roles.
pub trait ProvidesRolePositions {
    fn attacker_pos(&self) -> Option<Vec3>;
    fn defender_pos(&self) -> Option<Vec3>;
}

/// Events that carry damage metadata.
pub trait ProvidesDamageInfo {
    fn damage_amount(&self) -> f32;
    fn damage_element(&self) -> &str;
}

// ============================================================================
// Step 2: Define individual condition types
// ============================================================================

/// Validates that attacker and defender are within a maximum distance.
#[derive(Component, Clone, Reflect)]
pub struct DistanceWithin {
    pub max: f32,
}

impl<E: Event + ProvidesRolePositions> EventValidator<E> for DistanceWithin {
    fn matches(&self, event: &E) -> bool {
        let (Some(a), Some(d)) = (event.attacker_pos(), event.defender_pos()) else {
            return false;
        };
        a.distance(d) <= self.max
    }
}

/// Validates that the damage element matches one of the allowed types.
#[derive(Component, Clone, Reflect)]
pub struct DamageElementAnyOf {
    pub elements: Vec<String>,
}

impl<E: Event + ProvidesDamageInfo> EventValidator<E> for DamageElementAnyOf {
    fn matches(&self, event: &E) -> bool {
        self.elements.iter().any(|e| e == event.damage_element())
    }
}

/// Validates that the damage amount meets a threshold.
#[derive(Component, Clone, Reflect)]
pub struct DamageAmountAtLeast {
    pub minimum: f32,
}

impl<E: Event + ProvidesDamageInfo> EventValidator<E> for DamageAmountAtLeast {
    fn matches(&self, event: &E) -> bool {
        event.damage_amount() >= self.minimum
    }
}

// ============================================================================
// Step 3: Composable condition set
// ============================================================================

/// A composable set of conditions. All must pass for the set to validate.
#[derive(Component, Clone, Reflect)]
pub struct ConditionSet {
    #[reflect(ignore)]
    pub conditions: Vec<Condition>,
}

#[derive(Clone)]
pub enum Condition {
    DistanceWithin { max: f32 },
    DamageElementAnyOf { elements: Vec<String> },
    DamageAmountAtLeast { minimum: f32 },
}

/// Implement EventValidator for ConditionSet against events that provide both
/// role positions and damage info. Each condition is checked independently.
impl<E: Event + ProvidesRolePositions + ProvidesDamageInfo> EventValidator<E> for ConditionSet {
    fn matches(&self, event: &E) -> bool {
        self.conditions.iter().all(|cond| match cond {
            Condition::DistanceWithin { max } => {
                let (Some(a), Some(d)) = (event.attacker_pos(), event.defender_pos()) else {
                    return false;
                };
                a.distance(d) <= *max
            }
            Condition::DamageElementAnyOf { elements } => {
                elements.iter().any(|e| e == event.damage_element())
            }
            Condition::DamageAmountAtLeast { minimum } => event.damage_amount() >= *minimum,
        })
    }
}

// ============================================================================
// Step 4: Example usage with a concrete event type
// ============================================================================

#[derive(Event, Clone)]
pub struct DamageEvent {
    pub attacker: Entity,
    pub defender: Entity,
    pub attacker_position: Vec3,
    pub defender_position: Vec3,
    pub amount: f32,
    pub element: String,
}

impl ProvidesRolePositions for DamageEvent {
    fn attacker_pos(&self) -> Option<Vec3> {
        Some(self.attacker_position)
    }
    fn defender_pos(&self) -> Option<Vec3> {
        Some(self.defender_position)
    }
}

impl ProvidesDamageInfo for DamageEvent {
    fn damage_amount(&self) -> f32 {
        self.amount
    }
    fn damage_element(&self) -> &str {
        &self.element
    }
}

// ============================================================================
// Step 5: Using validators in state machine edges
// ============================================================================
//
// In practice, you'd attach a ConditionSet (or individual validators) to
// a gearbox EventEdge to gate when a transition fires:
//
// ```rust
// // Only trigger thorns when:
// // - Defender is within 5 units of attacker
// // - Damage is physical
// // - Damage is at least 10
// let edge = commands.spawn((
//     Source(idle_state),
//     bevy_gearbox::prelude::Target(thorns_state),
//     EventEdge::<HitDD>::default(),
//     ConditionSet {
//         conditions: vec![
//             Condition::DistanceWithin { max: 5.0 },
//             Condition::DamageElementAnyOf {
//                 elements: vec!["Physical".into(), "Blunt".into()],
//             },
//             Condition::DamageAmountAtLeast { minimum: 10.0 },
//         ],
//     },
// )).id();
// ```
//
// The gearbox state machine will call `ConditionSet::matches()` before
// allowing the transition, providing a clean, data-driven way to configure
// when effects activate.

fn main() {
    // This example is primarily a code pattern reference.
    // See the inline usage example above for how to integrate with gearbox.
    println!("Validators example — see source code for patterns");
}
