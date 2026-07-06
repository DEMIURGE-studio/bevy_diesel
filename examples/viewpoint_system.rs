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
//! This lets each entity's state machine react to combat events from its own
//! perspective — a "thorns" effect subscribes to HitDD (defender was hit) while
//! a "life steal" effect subscribes to HitAD (attacker hit defender).
//!
//! This is a PATTERN - copy and adapt it for your event types.

use bevy::prelude::*;
use bevy_diesel::prelude::*;
use bevy_gearbox::GearboxPlugin;

// ============================================================================
// Step 1: Define your base message with attacker/defender roles
// ============================================================================

#[derive(Message, Clone, Reflect)]
pub struct Hit {
    pub attacker: Entity,
    pub defender: Entity,
    pub amount: f32,
}

// ============================================================================
// Step 2: Define the 4 viewpoint variants (each addressed to a recipient)
// ============================================================================

macro_rules! viewpoint {
    ($Name:ident) => {
        #[derive(Message, Clone, Reflect, PropagatedMessage)]
        pub struct $Name {
            #[propagate(target)]
            pub target: Entity,
            pub base: Hit,
        }
    };
}

viewpoint!(HitAA); // delivered to attacker, GoOff targets attacker
viewpoint!(HitAD); // delivered to attacker, GoOff targets defender
viewpoint!(HitDA); // delivered to defender, GoOff targets attacker
viewpoint!(HitDD); // delivered to defender, GoOff targets defender

// ============================================================================
// Step 3: Forwarding system - splits the base Hit into the 4 variants
// ============================================================================

/// Reads `Hit` and emits the 4 viewpoint variants. The `CharacterMarker` filter
/// ensures we only deliver to actual participants (not projectiles, VFX, etc.).
fn forward_hit_viewpoints(
    mut reader: MessageReader<Hit>,
    q_character: Query<(), With<CharacterMarker>>,
    mut aa: MessageWriter<HitAA>,
    mut ad: MessageWriter<HitAD>,
    mut da: MessageWriter<HitDA>,
    mut dd: MessageWriter<HitDD>,
) {
    for base in reader.read() {
        let (attacker, defender) = (base.attacker, base.defender);

        if q_character.get(attacker).is_ok() {
            aa.write(HitAA { target: attacker, base: base.clone() });
            ad.write(HitAD { target: attacker, base: base.clone() });
        }
        if q_character.get(defender).is_ok() {
            da.write(HitDA { target: defender, base: base.clone() });
            dd.write(HitDD { target: defender, base: base.clone() });
        }
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

fn thorns_on_hit(mut reader: MessageReader<HitDD>, q_thorns: Query<&ThornsEffect>) {
    for hit in reader.read() {
        let Ok(thorns) = q_thorns.get(hit.target) else {
            continue;
        };
        // You would emit a Damage message here targeting the attacker.
        info!(
            "Thorns: reflecting {:.1} damage back to {:?}",
            thorns.reflect_damage, hit.base.attacker
        );
    }
}

// ============================================================================
// Step 5: Plugin registration
// ============================================================================

pub struct ViewpointPlugin;

impl Plugin for ViewpointPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Hit>();
        // Registers each variant's buffer + `propagate_message::<T>` (in the
        // gearbox schedule) from the `#[derive(PropagatedMessage)]` submissions.
        bevy_diesel::propagation::plugin(app);
        app.add_systems(Update, (forward_hit_viewpoints, thorns_on_hit));
    }
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        // Propagation runs inside GearboxSchedule (see damage_pipeline).
        .add_plugins(GearboxPlugin::default())
        .add_plugins(ViewpointPlugin)
        .run();
}
