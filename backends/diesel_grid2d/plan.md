---
title: diesel_grid2d — Grid-Based Roguelike Backend + Example
status: planned
---

# diesel_grid2d

A discrete grid backend for bevy_diesel. No physics engine — positions are
`IVec2` tile coordinates, distances are Manhattan/Chebyshev, and spatial queries
are simple entity iteration. Designed for roguelikes, tactics games, and any
tile-based system.

## Backend: `GridBackend`

```rust
impl SpatialBackend for GridBackend {
    type Pos     = IVec2;
    type Offset  = GridOffset;
    type Gatherer = GridGatherer;
    type Filter  = GridFilter;
    type Context<'w, 's> = GridContext<'w, 's>;
}
```

### Pos = `IVec2`

Tile coordinates. `spawn_transform` maps to world space via a tile size constant:
`Transform::from_xyz(pos.x as f32 * TILE_SIZE, pos.y as f32 * TILE_SIZE, 0.0)`.

`distance` uses Manhattan distance: `(a.x - b.x).abs() + (a.y - b.y).abs()`.

### GridOffset

```rust
enum GridOffset {
    None,
    Fixed(IVec2),
    RandomInManhattan(i32),   // random tile within Manhattan radius
    RandomInChebyshev(i32),   // random tile within Chebyshev radius
}
```

### GridGatherer

```rust
enum GridGatherer {
    // Position generators (produce tile coordinates)
    ManhattanRadius { radius: i32, count: NumberType },
    ChebyshevRadius { radius: i32, count: NumberType },
    Line { direction: IVec2, length: i32 },          // all tiles along a line
    Cross { radius: i32 },                            // + shape
    Diamond { radius: i32 },                          // filled Manhattan diamond

    // Entity gatherers (find entities on tiles)
    EntitiesInManhattan(i32),
    EntitiesInChebyshev(i32),
    EntitiesOnLine { direction: IVec2, length: i32 }, // entities along a ray
    NearestEntity(i32),                               // closest entity within radius
}
```

Position generators produce tile coordinates (useful for AoE ground effects).
Entity gatherers iterate `GridPosition` entities and filter by distance.

### GridFilter

```rust
struct GridFilter {
    count: Option<NumberType>,
    exclude_occupied: bool,  // filter to empty tiles (for position generators)
}
```

### GridContext

```rust
#[derive(SystemParam)]
struct GridContext<'w, 's> {
    positions: Query<'w, 's, (Entity, &'static GridPosition)>,
    rng: Local<'s, SplitMix64>,
}
```

No spatial index — just iterates entities. O(n) per gather call, which is fine
for roguelike scale (tens to low hundreds of entities).

### Components

```rust
#[derive(Component)]
struct GridPosition(pub IVec2);
```

This is the single spatial component. The backend reads it for `position_of` and
entity gathering.

### Plugin

Same core pattern. No collision system — turn-based games don't have physics
collisions. Abilities hit via gatherers directly.

```rust
struct GridDieselPlugin;

impl Plugin for GridDieselPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GridBackend::plugin_core());
        app.add_systems(Update,
            propagate_observer::<GridBackend>
                .in_set(DieselSet::Propagation));
        app.add_systems(Update, (
            spawn_observer::<GridBackend>,
            print_effect::<IVec2>,
            modifier_set_system::<IVec2>,
            instant_set_system::<IVec2>,
        ).in_set(DieselSet::Effects));
    }
}
```

### Dependencies

```toml
[dependencies]
bevy_diesel = { path = "../../crates/diesel" }
bevy = "0.18.0"
rand = "0.9"
bevy_gearbox = { path = "../../crates/gearbox" }
```

No physics crate needed.

---

# Roguelike Example

A turn-based dungeon crawler on a grid. Player moves with arrow keys, each move
is a turn. Enemies act after the player. Abilities are selected and targeted
with keyboard.

## Scene

- 20x15 tile grid rendered as colored squares (sprites or meshes)
- Walls (impassable), floor tiles, player, enemies, items
- Top-down 2D camera

## Tile Rendering

Each `GridPosition` entity gets a `Transform` synced via:
```rust
fn sync_grid_transforms(mut q: Query<(&GridPosition, &mut Transform)>) {
    for (gp, mut t) in q.iter_mut() {
        t.translation = Vec3::new(gp.0.x as f32, gp.0.y as f32, 0.0);
    }
}
```

Visual layers via Z-ordering: floor=0, items=1, characters=2, effects=3.

## Characters

```
Player:
  - GridPosition, Team(0)
  - Attributes: "Health" => 50, "MaxHealth" => 50, "Attack" => 8, "Defense" => 2
  - Inventory of ability entities

Enemy (Skeleton):
  - GridPosition, Team(1)
  - Attributes: "Health" => 20, "Attack" => 5, "Defense" => 1

Enemy (Mage):
  - GridPosition, Team(1)
  - Attributes: "Health" => 15, "Attack" => 10, "Defense" => 0
```

## Turn System

```rust
enum TurnPhase { PlayerInput, PlayerAction, EnemyAction, Resolve }
```

Resource-based state machine. Player moves/acts -> all enemies act -> resolve
deaths -> back to player input.

## Abilities / Templates

### 1. Melee Attack (basic)

Automatic when player bumps into an enemy (move into occupied tile).

```
State machine: Ready -[StartInvoke]-> Strike -> Ready
  Strike subeffects:
    - Gatherer: EntitiesInManhattan(1), Filter: count=1
    - instant!{ "Health" -= "Attack@attacker - Defense@target" }
```

### 2. Heal Potion (consumable)

```
State machine: Ready -[StartInvoke]-> Use -> Ready
  Use subeffects:
    - TargetMutator::root (targets self)
    - instant!{ "Health" += 20.0 }
```

Limited uses: gauge attribute `"Potions" => 3`, guarded by `requires!{ "Potions > 0" }`,
decremented on use.

### 3. Equip Armor (PAE)

A persistent attribute effect that stays applied while equipped.

```
PAE on player:
  AppliedModifiers: "Defense" => +5.0
```

Demonstrates the PAE lifecycle: equip = PAETryApply, unequip = PAEUnapplyApproved.
Could be triggered by walking over an armor item on the grid.

### 4. Fireball (AoE)

```
State machine: Ready -[StartInvoke]-> Cast -> Ready
  Cast subeffects:
    - SpawnConfig::at_passed("fire_explosion")
        .with_gatherer(GridGatherer::ChebyshevRadius { radius: 1, count: Fixed(9) })
    - On each gathered position: spawn fire VFX
    - On each gathered entity: instant!{ "Health" -= "Attack@attacker * 2" }
```

Targets a 3x3 area (Chebyshev radius 1) around a chosen tile. Player picks a
target tile, then all entities in the 3x3 area take damage.

### 5. Lightning Bolt (line)

```
State machine: Ready -[StartInvoke]-> Cast -> Ready
  Cast subeffects:
    - Gatherer: EntitiesOnLine { direction: chosen_dir, length: 5 }
    - instant!{ "Health" -= "Attack@attacker * 3" }
```

Hits the first enemy along a cardinal direction, up to 5 tiles away.

## Enemy AI

Simple chase-and-attack:
1. If adjacent to player: melee attack
2. Else: move one tile toward player (Manhattan pathfinding / greedy step)

Mage variant:
1. If within 4 tiles: cast a fireball-like AoE at player
2. Else: move toward player

## Items on the Grid

- Health potion (pickup -> adds to `"Potions"` count)
- Armor (pickup -> applies armor PAE)
- Scroll of Fireball (pickup -> grants fireball ability)

Walking onto an item tile triggers pickup.

## Input

| Input          | Action                          |
|----------------|---------------------------------|
| Arrow keys     | Move / bump-attack              |
| 1              | Use heal potion                 |
| 2              | Cast fireball (then pick tile)  |
| 3              | Cast lightning bolt (then pick dir) |
| Space          | Wait (skip turn)                |

## Systems

1. `player_input_system` — reads keys, sets action for the turn
2. `player_action_system` — executes movement or ability invocation
3. `enemy_ai_system` — each enemy picks and executes an action
4. `sync_grid_transforms` — GridPosition -> Transform
5. `death_system` — remove entities with Health <= 0
6. `pickup_system` — detect player stepping on item tiles
7. `turn_advance_system` — cycles TurnPhase
8. `ui_system` — text overlay: health, potions, ability cooldowns

## What This Demonstrates

- **Discrete grid backend** (IVec2, Manhattan distance, no physics)
- **Turn-based gameplay** (resource-driven turn phases)
- **AoE on a grid** (3x3 fireball, line lightning bolt)
- **Consumables** (potion with limited uses via gauge)
- **Equipment as PAE** (armor that persistently modifies Defense)
- **Bump-to-attack** (melee as a side effect of movement)
- **Simple AI** (chase + attack pattern)
