//! Example: Damage Pipeline using diesel's propagation system
//!
//! Demonstrates how to build a multi-stage combat resolution chain:
//!   Attack → Hit → (defense checks) → Damage → Killed
//!
//! Each stage is an EntityEvent that propagates upward through the entity hierarchy
//! using `submit_propagation_for!`. Defense observers intercept Hit events and
//! may absorb them (preventing Damage) or let them through.
//!
//! This is a PATTERN — copy and adapt it. Diesel provides the propagation infrastructure;
//! you define the events, defense formulas, and resolution logic.

use bevy::{ecs::event::SetEntityEventTarget, prelude::*};
use bevy_diesel::prelude::*;
use bevy_diesel::submit_propagation_for;

// ============================================================================
// Step 1: Define your combat events
// ============================================================================

/// Initial attack event. Triggered by abilities with AttackEffect.
#[derive(EntityEvent, Clone, Reflect)]
pub struct Attack {
    #[event_target]
    pub defender: Entity,
    pub attacker: Entity,
    pub ability: Entity,
    pub element: String,
}
impl SetEntityEventTarget for Attack {
    fn set_event_target(&mut self, target: Entity) { self.defender = target; }
}

/// Post-defense hit. Triggered after defense checks pass.
#[derive(EntityEvent, Clone, Reflect)]
pub struct Hit {
    #[event_target]
    pub defender: Entity,
    pub attacker: Entity,
    pub ability: Entity,
    pub element: String,
    pub hit_value: f32,
}
impl SetEntityEventTarget for Hit {
    fn set_event_target(&mut self, target: Entity) { self.defender = target; }
}

/// Final damage applied to health.
#[derive(EntityEvent, Clone, Reflect)]
pub struct Damage {
    #[event_target]
    pub defender: Entity,
    pub attacker: Entity,
    pub ability: Entity,
    pub element: String,
    pub amount: f32,
}
impl SetEntityEventTarget for Damage {
    fn set_event_target(&mut self, target: Entity) { self.defender = target; }
}

/// Entity was killed.
#[derive(EntityEvent, Clone, Reflect)]
pub struct Killed {
    #[event_target]
    pub defender: Entity,
    pub attacker: Entity,
}
impl SetEntityEventTarget for Killed {
    fn set_event_target(&mut self, target: Entity) { self.defender = target; }
}

// Register propagation so parent state machines can subscribe to these events
submit_propagation_for!(Attack);
submit_propagation_for!(Hit);
submit_propagation_for!(Damage);
submit_propagation_for!(Killed);

// ============================================================================
// Step 2: Defense components (user-defined)
// ============================================================================

#[derive(Component)]
pub struct Armor(pub f32);

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

// ============================================================================
// Step 3: Resolution observers
// ============================================================================

/// Attack → Hit: Evaluate base hit value from ability stats
fn resolve_attack(
    attack: On<Attack>,
    mut commands: Commands,
) {
    // In a real game, you'd read AbilityDamage expressions here
    let base_hit = 50.0; // placeholder

    commands.trigger(Hit {
        defender: attack.defender,
        attacker: attack.attacker,
        ability: attack.ability,
        element: attack.element.clone(),
        hit_value: base_hit,
    });
}

/// Hit → Damage: Apply defense (armor reduction)
fn resolve_hit(
    hit: On<Hit>,
    q_armor: Query<&Armor>,
    mut commands: Commands,
) {
    let mut remaining = hit.hit_value;

    // Armor check
    if let Ok(armor) = q_armor.get(hit.defender) {
        remaining -= armor.0;
    }

    if remaining <= 0.0 {
        // Attack was fully absorbed — don't emit Damage
        info!("Attack absorbed by armor");
        return;
    }

    commands.trigger(Damage {
        defender: hit.defender,
        attacker: hit.attacker,
        ability: hit.ability,
        element: hit.element.clone(),
        amount: remaining,
    });
}

/// Damage → apply to health, check for kill
fn resolve_damage(
    damage: On<Damage>,
    mut q_health: Query<&mut Health>,
    mut commands: Commands,
) {
    let Ok(mut health) = q_health.get_mut(damage.defender) else {
        return;
    };

    health.current -= damage.amount;
    info!(
        "Dealt {:.1} {} damage to {:?} (health: {:.1}/{:.1})",
        damage.amount, damage.element, damage.defender, health.current, health.max
    );

    if health.current <= 0.0 {
        commands.trigger(Killed {
            defender: damage.defender,
            attacker: damage.attacker,
        });
    }
}

fn on_killed(killed: On<Killed>) {
    info!("{:?} was killed by {:?}", killed.defender, killed.attacker);
}

// ============================================================================
// Step 4: Plugin registration
// ============================================================================

pub struct DamagePipelinePlugin;

impl Plugin for DamagePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(resolve_attack)
            .add_observer(resolve_hit)
            .add_observer(resolve_damage)
            .add_observer(on_killed);

        // Register propagation for all combat events (from inventory submissions)
        bevy_diesel::propagation::plugin(app);
    }
}

fn main() {
    // This example demonstrates the pattern — in a real app you'd add AvianDieselPlugin,
    // DamagePipelinePlugin, spawn entities with Health/Armor, and trigger Attack events.
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(DamagePipelinePlugin)
        .run();
}
