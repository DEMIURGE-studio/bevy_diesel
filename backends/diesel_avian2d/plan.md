---
title: diesel_avian2d — 2D Fighting Game Backend + Example
status: planned
---

# diesel_avian2d

A 2D physics backend for bevy_diesel using Avian2d. Mirrors the structure of
`diesel_avian3d` but for `Vec2` positions, 2D colliders, and 2D physics.

## Backend: `Avian2dBackend`

```rust
impl SpatialBackend for Avian2dBackend {
    type Pos     = Vec2;
    type Offset  = Vec2Offset;
    type Gatherer = Gatherer2d;
    type Filter  = Filter2d;
    type Context<'w, 's> = Avian2dContext<'w, 's>;
}
```

### Pos = `Vec2`

Positions are 2D. `spawn_transform` maps `Vec2` to `Transform::from_xyz(x, y, 0.0)`.
`position_of` reads `Transform.translation.truncate()`.

### Vec2Offset

```rust
enum Vec2Offset {
    None,
    Fixed(Vec2),
    RandomBetween(Vec2, Vec2),
    RandomInCircle(f32),
}
```

### Gatherer2d

```rust
enum Gatherer2d {
    // Position generators (embed count)
    Circle { radius: f32, count: NumberType },
    Box { half_extents: Vec2, count: NumberType },
    Line { direction: Vec2, length: f32, count: NumberType },

    // Entity gatherers (query avian2d spatial index)
    EntitiesInCircle(f32),
    NearestEntities(f32),
    AllEntitiesInRadius(f32),
}
```

Same split as the 3D backend: position generators produce points, entity gatherers
query the Avian2d `SpatialQuery` with a circle collider.

### Filter2d

```rust
struct Filter2d {
    count: Option<NumberType>,
    facing_only: bool,  // filter to entities in front of invoker
}
```

`facing_only` checks `dot(direction_to_target, invoker_facing) > 0`. Requires a
`Facing(Vec2)` component on invokers, queried via the context.

### Avian2dContext

```rust
#[derive(SystemParam)]
struct Avian2dContext<'w, 's> {
    spatial_query: SpatialQuery<'w, 's>,  // avian2d 2D spatial queries
    transforms: Query<'w, 's, &'static Transform>,
    facings: Query<'w, 's, &'static Facing>,
    rng: Local<'s, SplitMix64>,
}
```

### Modules

Mirror the 3D backend where applicable:

| Module         | Contents                                              |
|----------------|-------------------------------------------------------|
| `lib.rs`       | Backend impl, offset/gatherer/filter, plugin, RNG     |
| `collision.rs` | 2D collision -> CollidedEntity/CollidedPosition        |
| `projectile.rs`| LinearProjectile2d (constant speed, no gravity)        |

No `ballistics.rs` or `velocity.rs` initially — fighting games don't need
parabolic arcs. Can be added later for a platformer example.

### Plugin

```rust
struct Avian2dDieselPlugin;

impl Plugin for Avian2dDieselPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Avian2dBackend::plugin_core());
        app.add_systems(Update,
            propagate_observer::<Avian2dBackend>
                .in_set(DieselSet::Propagation));
        app.add_systems(Update, (
            spawn_observer::<Avian2dBackend>,
            print_effect::<Vec2>,
            modifier_set_system::<Vec2>,
            instant_set_system::<Vec2>,
        ).in_set(DieselSet::Effects));
        // 2D-specific
        app.add_plugins(Projectile2dPlugin);
        collision2d::plugin(app);
    }
}
```

### Dependencies

```toml
[dependencies]
bevy_diesel = { path = "../../crates/diesel" }
bevy = "0.18.0"
avian2d = "0.5"
rand = "0.9"
bevy_gearbox = { path = "../../crates/gearbox" }
```

---

# Fighter Example

A 2-player (or player vs AI) side-view fighting game. Two characters face each
other on a flat stage. Real-time combat with punches, kicks, a projectile
special, blocking, and a combo chain.

## Scene

- Flat stage (static collider, ~20 units wide)
- Two fighters: player 1 (left, blue) and player 2 (right, red/AI)
- 2D camera centered on the midpoint between fighters
- Health bars above each fighter

## Fighter Setup

```
Fighter entity:
  - RigidBody::Dynamic, Capsule2d collider
  - Team(id), Invokes, Facing(Vec2)
  - Attributes:
      "Health"    => 100.0
      "MaxHealth" => 100.0
      "DmgReduce" => 0.0  (blocking adds to this)
  - FighterAbilities { punch, kick, hadouken, block, combo }
```

## Collision Layers

```rust
enum Layer { Stage, Fighter, Projectile, Hitbox }
```

## Abilities / Templates

### 1. Punch (fast, short range, low damage)

```
State machine: Ready -[StartInvoke]-> Windup(0.05s) -> Active -> Recovery(0.15s) -> Ready
  Active subeffects:
    - Gatherer: EntitiesInCircle(1.5), Filter: count=1 + facing_only
    - instant!{ "Health" -= 5.0 }
```

Short windup, fast recovery. The gatherer finds the closest enemy in front.

### 2. Kick (medium range, medium damage, slower)

```
State machine: Ready -[StartInvoke]-> Windup(0.1s) -> Active -> Recovery(0.3s) -> Ready
  Active subeffects:
    - Gatherer: EntitiesInCircle(2.5), Filter: count=1 + facing_only
    - instant!{ "Health" -= 12.0 }
```

### 3. Hadouken (projectile special)

```
State machine: Ready -[StartInvoke]-> Cast -> Recovery(0.5s) -> Ready
  Cast subeffects:
    - SpawnConfig::at_invoker("hadouken_projectile")
        .with_offset(Vec2Offset::Fixed(facing * 1.0))
        .with_target_generator(TargetGenerator::at_invoker_target())
```

Template: `hadouken_projectile`
- LinearProjectile2d, speed 12.0, travels in invoker's facing direction
- On collision: instant `"Health" -= 15.0`, despawn self
- CollisionLayers: [Projectile], [Fighter]
- Visual: glowing circle sprite

### 4. Block (hold-to-activate PAE)

```
State machine: Ready -[StartInvoke]-> Blocking -[StopInvoke]-> Ready
  Blocking: StateComponent(BlockActive) inserted on fighter
  PAE-style: while Blocking state is active, "DmgReduce" += 0.6
```

Damage formula in all attacks becomes `damage * (1.0 - "DmgReduce@target")`.
Blocking reduces incoming damage by 60%.

Uses `StateComponent` to insert/remove a marker while the block state is active,
paired with a gauge modifier that's conditionally applied.

### 5. Combo (punch -> punch -> kick chain)

```
State machine:
  Ready -[StartInvoke]-> Punch1 -[after 0.25s]-> Punch2 -[after 0.25s]-> Kick -[always]-> Ready

  Punch1 subeffects: same as punch (5 dmg, 1.5 range)
  Punch2 subeffects: same as punch (5 dmg, 1.5 range)
  Kick subeffects:   enhanced kick (18 dmg, 2.5 range, "finisher")
```

Multi-state ability (not a Repeater — each hit is different). Demonstrates
that complex sequenced attacks are just state machines with timed transitions.

## Input

| Input (P1)     | Action     | Input (P2/AI)   |
|----------------|------------|------------------|
| A/D            | Move L/R   | Arrow L/R        |
| W              | Jump       | Arrow Up         |
| J              | Punch      | Numpad 1         |
| K              | Kick       | Numpad 2         |
| L              | Hadouken   | Numpad 3         |
| S (hold)       | Block      | Arrow Down (hold) |
| Space          | Combo      | Numpad 0         |

P2 could also be a simple AI that approaches and attacks randomly.

## Systems

1. `movement_system` — horizontal movement + jump, update `Facing` based on direction
2. `ai_system` — simple enemy AI: approach player, attack when in range
3. `invoke_abilities_system` — input -> StartInvoke/StopInvoke
4. `camera_follow_system` — track midpoint between fighters
5. `health_bar_system` — UI health bars
6. `ko_system` — detect `"Health" <= 0`, show KO screen
7. `facing_sync_system` — flip sprite based on `Facing` direction

## What This Demonstrates

- **2D physics backend** (Avian2d spatial queries, 2D colliders)
- **Melee attacks** (short-range gatherer + facing filter)
- **2D projectiles** (LinearProjectile2d, hadouken)
- **Blocking** (StateComponent + conditional damage reduction)
- **Multi-hit combos** (sequenced state machine, not repeater)
- **Hold-to-activate** (StartInvoke / StopInvoke for block)
- **Two-player or player-vs-AI** input model
