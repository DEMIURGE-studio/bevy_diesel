//! Example: Viewpoint System (AA/AD/DA/DD pattern)
//!
//! In a two-role combat system (attacker/defender), a single base event like "Hit"
//! needs to be delivered to BOTH participants with different perspectives:
//!
//!   AA = Attacker sees themselves attacking    (recipient: attacker, GoOff target: attacker)
//!   AD = Attacker sees effect on defender      (recipient: attacker, GoOff target: defender)
//!   DA = Defender sees attacker's action       (recipient: defender, GoOff target: attacker)
//!   DD = Defender sees themselves being hit     (recipient: defender, GoOff target: defender)
//!
//! This allows each entity's state machine to independently react to combat events
//! from their own perspective. For example, a "thorns" PAE subscribes to HitDD
//! (defender was hit) while a "life steal" PAE subscribes to HitAD (attacker hit defender).
//!
//! This is a PATTERN - copy and adapt it for your event types.

use bevy::{ecs::event::SetEntityEventTarget, prelude::*};
use bevy_diesel::submit_propagation_for;

// ============================================================================
// Step 1: Define your base event with attacker/defender roles
// ============================================================================

#[derive(Event, Clone, Reflect)]
pub struct Hit {
    pub attacker: Entity,
    pub defender: Entity,
    pub amount: f32,
}

// ============================================================================
// Step 2: Define the 4 viewpoint variants
// ============================================================================

/// Delivered to attacker, GoOff targets attacker entity
#[derive(EntityEvent, Clone, Reflect)]
pub struct HitAA {
    #[event_target]
    pub target: Entity,
    pub base: Hit,
}
impl SetEntityEventTarget for HitAA {
    fn set_event_target(&mut self, target: Entity) { self.target = target; }
}

/// Delivered to attacker, GoOff targets defender entity
#[derive(EntityEvent, Clone, Reflect)]
pub struct HitAD {
    #[event_target]
    pub target: Entity,
    pub base: Hit,
}
impl SetEntityEventTarget for HitAD {
    fn set_event_target(&mut self, target: Entity) { self.target = target; }
}

/// Delivered to defender, GoOff targets attacker entity
#[derive(EntityEvent, Clone, Reflect)]
pub struct HitDA {
    #[event_target]
    pub target: Entity,
    pub base: Hit,
}
impl SetEntityEventTarget for HitDA {
    fn set_event_target(&mut self, target: Entity) { self.target = target; }
}

/// Delivered to defender, GoOff targets defender entity
#[derive(EntityEvent, Clone, Reflect)]
pub struct HitDD {
    #[event_target]
    pub target: Entity,
    pub base: Hit,
}
impl SetEntityEventTarget for HitDD {
    fn set_event_target(&mut self, target: Entity) { self.target = target; }
}

// Register propagation so parent state machines can subscribe
submit_propagation_for!(HitAA);
submit_propagation_for!(HitAD);
submit_propagation_for!(HitDA);
submit_propagation_for!(HitDD);

// ============================================================================
// Step 3: Forwarding observer - splits base event into 4 variants
// ============================================================================

/// Generic forwarder: observes a base event and emits the 4 viewpoint variants.
/// The `character_filter` query ensures we only deliver to entities that are
/// actual participants (not projectiles, VFX, etc.).
fn forward_hit_viewpoints(
    base: On<Hit>,
    q_character: Query<(), With<CharacterMarker>>,
    mut commands: Commands,
) {
    let attacker = base.attacker;
    let defender = base.defender;
    let attacker_is_char = q_character.get(attacker).is_ok();
    let defender_is_char = q_character.get(defender).is_ok();

    if attacker_is_char {
        commands.trigger(HitAA {
            target: attacker,
            base: (*base).clone(),
        });
        commands.trigger(HitAD {
            target: attacker,
            base: (*base).clone(),
        });
    }

    if defender_is_char {
        commands.trigger(HitDA {
            target: defender,
            base: (*base).clone(),
        });
        commands.trigger(HitDD {
            target: defender,
            base: (*base).clone(),
        });
    }
}

// ============================================================================
// Step 4: Example subscriber - "thorns" effect reacts to being hit
// ============================================================================

#[derive(Component)]
pub struct ThornsEffect {
    pub reflect_damage: f32,
}

#[derive(Component)]
pub struct CharacterMarker;

fn thorns_on_hit(
    hit: On<HitDD>,
    q_thorns: Query<&ThornsEffect>,
    mut commands: Commands,
) {
    let defender = hit.target;
    let Ok(thorns) = q_thorns.get(defender) else {
        return;
    };

    // Reflect damage back to attacker
    info!(
        "Thorns: reflecting {:.1} damage back to {:?}",
        thorns.reflect_damage, hit.base.attacker
    );

    // You would trigger a Damage event here targeting the attacker
    let _ = commands; // placeholder
}

// ============================================================================
// Step 5: Plugin registration
// ============================================================================

pub struct ViewpointPlugin;

impl Plugin for ViewpointPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(forward_hit_viewpoints)
            .add_observer(thorns_on_hit);

        bevy_diesel::propagation::plugin(app);
    }
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(ViewpointPlugin)
        .run();
}
