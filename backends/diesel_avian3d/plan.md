---
title: diesel_avian3d — FPS Example Extension
status: planned
---

# FPS Example

Extend the existing `diesel_avian3d` backend with a first-person shooter example
that showcases hitscan weapons, physics projectiles, ammo management, power-ups,
and destructible targets. No backend changes needed — everything builds on the
existing `AvianBackend`, `ProjectileEffect`, `CollisionFilterPlugin`, and gauge
integration.

## Scene

- Ground plane + a few walls/cover (static rigid bodies)
- 8-10 target dummies scattered around (team 1, `"Health"` attribute)
- 3 power-up pickups floating above pedestals (rotating, `Sensor` collider)
- First-person camera locked to player capsule

## Key Design: Weapons Own Their Attributes

Abilities and stats live on **weapon entities**, not the player. The player
`Invokes` weapons, and each weapon carries its own gauge attributes. This
models the real pattern: ammo, damage, fire rate, reload speed are properties
of the gun, not the character.

```
Player entity:
  - RigidBody::Kinematic, Capsule collider
  - Team(0), Invokes, InvokerTarget (camera ray hit point)
  - Attributes:
      "Health"    => 100.0
      "MoveSpeed" => 8.0
  - EquippedWeapon(entity) — points to currently held weapon

Rifle entity (child of player):
  - InvokedBy(player)
  - Ability
  - Attributes:
      "Ammo"       => 30.0
      "MaxAmmo"    => 30.0
      "Damage"     => 10.0
      "FireRate"   => 0.1     (seconds between shots)

Rocket Launcher entity (child of player):
  - InvokedBy(player)
  - Ability
  - Attributes:
      "Ammo"       => 5.0
      "MaxAmmo"    => 5.0
      "Damage"     => 40.0
      "SplashRadius" => 5.0
```

Power-ups that modify damage target the *weapon's* attributes, not the player's.

## Collision Layers

```rust
enum Layer { Terrain, Player, Enemy, Projectile, Pickup }
```

Reuse the `TeamFilter::Enemies` pattern from fireballs for projectile filtering.

## Abilities / Templates

### 1. Hitscan Rifle

The rifle entity IS the ability. Its state machine:

```
State machine: Ready -[StartInvoke]-> Fire -[always]-> Ready
  Ready: guarded by requires!{ "Ammo > 0" }
         (RequiresStatsOf points to self — the weapon entity)
  Fire subeffects:
    - instant!{ "Ammo" -= 1.0 } targeting self (TargetMutator::root)
    - Hitscan damage delivered via a *system*, not a spawned projectile
```

The hitscan system:
- Reads `GoOff` for entities with a `HitscanMarker` component
- Casts a ray from the camera using `SpatialQuery::cast_ray`
- On hit: reads `"Damage"` from the weapon's attributes, applies to target's `"Health"`

This demonstrates a **system-driven effect** rather than a template-spawned one.

### 2. Rocket Launcher

Also an ability entity with its own attributes:

```
State machine: Ready -[StartInvoke]-> Fire -[always]-> Ready
  Ready: guarded by requires!{ "Ammo > 0" }
  Fire subeffects:
    - instant!{ "Ammo" -= 1.0 } targeting self
    - SpawnConfig::at_invoker("rocket_projectile")
        .with_offset(Vec3Offset::Fixed(Vec3::Y * 1.5))
        .with_target_generator(TargetGenerator::at_invoker_target())
```

Template: `rocket_projectile` — reuses the `explosive_projectile_template` pattern
from fireballs (ProjectileEffect, explosion on collision, life gauge).

Template: `rocket_explosion` — AoE damage via gatherer. Damage reads from the
weapon's attributes via the InvokedBy chain.

### 3. Reload

Sub-state within each weapon's state machine:

```
Ready -[ReloadInvoke]-> Reloading -[after delay]-> Ready
  Reloading subeffects:
    - instant!{ "Ammo" = "MaxAmmo" } targeting self
```

Reload time could be a weapon attribute, so different guns reload at different speeds.

### 4. Weapon Switching

`EquippedWeapon(Entity)` component on the player. Number keys or scroll wheel
switch. The invoke system reads the equipped weapon and sends `StartInvoke` to
that specific entity.

## Power-Ups (PAEs)

Each pickup is an entity with `Sensor` + `Collides` marker. On collision with
player, apply a PAE to the player.

### Speed Boost (targets player)
- `ActivatedModifiers`: `"MoveSpeed" => +4.0` (flat)
- Duration: 10 seconds
- Visual: blue glow

### Damage Boost (targets equipped weapon)
- `ActivatedModifiers`: `"Damage" => +5.0` (flat)
- Applied to the *weapon entity*, not the player
- Duration: 8 seconds
- Visual: red glow

### Health Pack (instant, targets player)
- Not a PAE — just an instant `"Health" += 50.0` on pickup
- Visual: green cross

## Target Dummies

```
Target entity:
  - RigidBody::Static or Dynamic, Box collider
  - Team(1)
  - Attributes: "Health" => 50.0, "MaxHealth" => 50.0
  - CollisionLayers: [Enemy], [Projectile]
```

Visual feedback: lerp material color red -> gray based on `Health / MaxHealth`.
Despawn (collapse animation) when health <= 0.

## Input Mapping

| Input              | Action                   |
|--------------------|--------------------------|
| WASD               | Movement                 |
| Mouse              | Look                     |
| Left click         | Fire equipped weapon     |
| R                  | Reload equipped weapon   |
| 1 / 2              | Switch weapon            |
| Esc                | Release cursor           |

## Systems (in the example file)

1. `fps_camera_system` — mouse look via `AccumulatedMouseMotion`, cursor grab
2. `player_movement_system` — WASD relative to camera, reads `"MoveSpeed"` from player
3. `hitscan_system` — raycast + damage, reads `"Damage"` from weapon attributes
4. `invoke_abilities_system` — input -> StartInvoke to equipped weapon entity
5. `weapon_switch_system` — 1/2 keys change `EquippedWeapon`
6. `pickup_collision_system` — detect overlap with pickups, apply PAE to player or weapon
7. `dummy_feedback_system` — color lerp based on health ratio
8. `dummy_death_system` — despawn on health <= 0
9. `hud_system` — text overlay: weapon name, ammo/max, health, active boosts

## What This Demonstrates

- **Weapon-owned attributes** (ammo, damage, reload speed live on the gun, not the player)
- **Hitscan** (system-driven effect, not projectile-spawned)
- **Physics projectiles** (rocket reuses existing ProjectileEffect)
- **Ammo management** (gauge guards on weapon entity prevent fire when empty)
- **Reload** (delayed transition, duration from weapon attribute)
- **Weapon switching** (invoke different ability entities)
- **Power-ups targeting weapons** (damage boost modifies the gun's stats, not the player's)
- **AoE explosion** (rocket splash damage via gatherer)
- **First-person camera** (common game pattern)
