//! Example: Damage Pipeline using diesel's message propagation
//!
//! Demonstrates a multi-stage combat resolution chain:
//!   Attack -> Hit -> (defense checks) -> Damage -> Killed
//!
//! Each stage is a `Message` that carries the entity it is addressed to
//! (`PropagatedMessage`). A stage system reads one message type and writes the
//! next; `.chain()` ordering resolves the whole pipeline in a single frame -
//! no fixpoint loop, because each stage is a distinct type.
//!
//! Deriving `PropagatedMessage` registers each type's buffer and its
//! subscription graph, so a parent state machine can subscribe to a defender's
//! combat events (forwarded copies arrive the following frame - the one
//! ergonomic cost versus the old immediate-observer model).
//!
//! This is a PATTERN - copy and adapt it. Diesel provides the message plumbing;
//! you define the events, defense formulas, and resolution logic.

use bevy::prelude::*;
use bevy_diesel::prelude::*;
use bevy_gearbox::{GearboxPlugin, GearboxSet};

// ============================================================================
// Step 1: Define your combat messages
// ============================================================================

/// Initial attack. Written by abilities that strike a defender.
#[derive(Message, Clone, Reflect, PropagatedMessage)]
pub struct Attack {
    #[propagate(target)]
    pub defender: Entity,
    pub attacker: Entity,
    pub ability: Entity,
    pub element: String,
}

/// Post-defense hit. Written after the attack is evaluated.
#[derive(Message, Clone, Reflect, PropagatedMessage)]
pub struct Hit {
    #[propagate(target)]
    pub defender: Entity,
    pub attacker: Entity,
    pub ability: Entity,
    pub element: String,
    pub hit_value: f32,
}

/// Final damage applied to health.
#[derive(Message, Clone, Reflect, PropagatedMessage)]
pub struct Damage {
    #[propagate(target)]
    pub defender: Entity,
    pub attacker: Entity,
    pub ability: Entity,
    pub element: String,
    pub amount: f32,
}

/// Entity was killed.
#[derive(Message, Clone, Reflect, PropagatedMessage)]
pub struct Killed {
    #[propagate(target)]
    pub defender: Entity,
    pub attacker: Entity,
}

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
// Step 3: Resolution systems (ordered - the chain resolves in one frame)
// ============================================================================

/// Attack -> Hit: evaluate base hit value from ability stats.
fn resolve_attack(mut reader: MessageReader<Attack>, mut writer: MessageWriter<Hit>) {
    for attack in reader.read() {
        // In a real game you'd read ability damage expressions here.
        let base_hit = 50.0;
        writer.write(Hit {
            defender: attack.defender,
            attacker: attack.attacker,
            ability: attack.ability,
            element: attack.element.clone(),
            hit_value: base_hit,
        });
    }
}

/// Hit -> Damage: apply defense (armor reduction), absorbing if it hits zero.
fn resolve_hit(
    mut reader: MessageReader<Hit>,
    q_armor: Query<&Armor>,
    mut writer: MessageWriter<Damage>,
) {
    for hit in reader.read() {
        let mut remaining = hit.hit_value;
        if let Ok(armor) = q_armor.get(hit.defender) {
            remaining -= armor.0;
        }
        if remaining <= 0.0 {
            info!("Attack absorbed by armor");
            continue;
        }
        writer.write(Damage {
            defender: hit.defender,
            attacker: hit.attacker,
            ability: hit.ability,
            element: hit.element.clone(),
            amount: remaining,
        });
    }
}

/// Damage -> apply to health, emit `Killed` if it drops to zero.
fn resolve_damage(
    mut reader: MessageReader<Damage>,
    mut q_health: Query<&mut Health>,
    mut writer: MessageWriter<Killed>,
    frame: Res<FrameCount>,
) {
    for damage in reader.read() {
        let Ok(mut health) = q_health.get_mut(damage.defender) else {
            continue;
        };
        health.current -= damage.amount;
        info!(
            "[frame {}] dealt {:.1} {} damage to {:?} (health: {:.1}/{:.1})",
            frame.0, damage.amount, damage.element, damage.defender, health.current, health.max
        );
        if health.current <= 0.0 {
            info!("[frame {}] emitting Killed for {:?}", frame.0, damage.defender);
            writer.write(Killed {
                defender: damage.defender,
                attacker: damage.attacker,
            });
        }
    }
}

/// The real death (Killed addressed to the entity that died, not a subscriber).
fn on_killed(
    mut reader: MessageReader<Killed>,
    q_machine: Query<(), With<Machine>>,
    frame: Res<FrameCount>,
) {
    for killed in reader.read() {
        if q_machine.get(killed.defender).is_ok() {
            continue; // that's a forwarded copy, handled by machine_on_killed
        }
        info!("[frame {}] {:?} was killed by {:?}", frame.0, killed.defender, killed.attacker);
    }
}

/// A subscribing "state machine": it receives `Killed` events *forwarded* from
/// the defender it subscribed to. This is the propagation fan-out in action.
fn machine_on_killed(
    mut reader: MessageReader<Killed>,
    q_machine: Query<(), With<Machine>>,
    frame: Res<FrameCount>,
) {
    for killed in reader.read() {
        if q_machine.get(killed.defender).is_ok() {
            info!(
                "[frame {}] MACHINE {:?} received forwarded Killed",
                frame.0, killed.defender
            );
        }
    }
}

// ============================================================================
// Step 4: Plugin registration
// ============================================================================

pub struct DamagePipelinePlugin;

impl Plugin for DamagePipelinePlugin {
    fn build(&self, app: &mut App) {
        // Registers the message buffers, subscription graphs, and
        // `propagate_message::<T>` systems for every `#[derive(PropagatedMessage)]`.
        bevy_diesel::propagation::plugin(app);

        // Stage systems, ordered so a fired Attack resolves all the way to
        // Killed before the gearbox schedule runs (which forwards copies to
        // subscribers inside its fixpoint loop).
        app.add_systems(
            Update,
            (resolve_attack, resolve_hit, resolve_damage, on_killed)
                .chain()
                .before(GearboxSet),
        );
        // The subscriber reads after the gearbox loop has forwarded the copy.
        app.add_systems(Update, machine_on_killed.after(GearboxSet));
    }
}

// ============================================================================
// Demo: strike one defender, and a "machine" subscribed to it observes the kill
// ============================================================================

/// Stand-in for an ability state machine that reacts to a target's combat
/// outcomes (e.g. "on kill, refund cooldown").
#[derive(Component)]
struct Machine;

#[derive(Resource, Default)]
struct FrameCount(u64);

fn tick_frame(mut f: ResMut<FrameCount>) {
    f.0 += 1;
}

fn spawn_demo(mut commands: Commands) {
    // 40 hp behind 10 armor: a 50-base hit lands 40 damage and kills it.
    let defender = commands
        .spawn((Health { current: 40.0, max: 40.0 }, Armor(10.0)))
        .id();

    // A machine that subscribes to the defender's Killed events.
    let machine = commands.spawn(Machine).id();
    commands.trigger(RegisterPropagationTarget::<Killed>::new(machine, defender));
}

fn fire_once(
    mut done: Local<bool>,
    q_defender: Query<Entity, (With<Health>, Without<Machine>)>,
    mut writer: MessageWriter<Attack>,
    frame: Res<FrameCount>,
) {
    if *done {
        return;
    }
    let Ok(defender) = q_defender.single() else {
        return;
    };
    info!("[frame {}] firing Attack at {:?}", frame.0, defender);
    writer.write(Attack {
        defender,
        attacker: defender, // no separate attacker entity in this minimal demo
        ability: defender,
        element: "fire".to_string(),
    });
    *done = true;
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin::default())
        // Propagation now runs inside GearboxSchedule, so the gearbox plugin
        // must be present. (A real diesel app gets this from DieselCorePlugin.)
        .add_plugins(GearboxPlugin::default())
        .add_plugins(DamagePipelinePlugin)
        .init_resource::<FrameCount>()
        .add_systems(First, tick_frame)
        .add_systems(Startup, spawn_demo)
        .add_systems(Update, fire_once)
        .run();
}
