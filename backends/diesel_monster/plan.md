---
title: diesel_abstract — Abstract Space Backend + Pokemon Example
status: planned
---

# diesel_abstract

A backend with no spatial dimension at all. Positions are `()`. There is no
distance, no offset, no physics. Targeting is purely logical: "all allies",
"all enemies", "self", "random enemy". This is the simplest possible backend
and proves that bevy_diesel's spatial abstraction scales down to zero dimensions.

## Backend: `AbstractBackend`

```rust
impl SpatialBackend for AbstractBackend {
    type Pos     = ();
    type Offset  = NoOffset;
    type Gatherer = SlotGatherer;
    type Filter  = SlotFilter;
    type Context<'w, 's> = BattleContext<'w, 's>;
}
```

### Pos = `()`

No position. `distance` returns 0. `apply_offset` returns `()`.
`spawn_transform` returns `Transform::IDENTITY`.
`position_of` returns `Some(())` if the entity exists, `None` otherwise.

### NoOffset

```rust
struct NoOffset;
```

Single unit struct. Default, Clone, Debug. `apply_offset` ignores it.

### SlotGatherer

```rust
enum SlotGatherer {
    AllAllies,
    AllEnemies,
    RandomAlly(NumberType),
    RandomEnemy(NumberType),
    Self_,
}
```

The `exclude` parameter in `gather()` is the invoker entity. The context
queries the invoker's `Team` to determine allies vs enemies.

### SlotFilter

```rust
struct SlotFilter {
    count: Option<NumberType>,
    alive_only: bool,
}
```

`alive_only` checks `"HP" > 0` via the gauge `Attributes` query.

### BattleContext

```rust
#[derive(SystemParam)]
struct BattleContext<'w, 's> {
    combatants: Query<'w, 's, (Entity, &'static Team)>,
    rng: Local<'s, SplitMix64>,
}
```

Minimal. Just needs team membership to resolve ally/enemy. The `alive_only`
filter reads `Attributes` but that can be a separate query in `apply_filter`.

### Plugin

Bare minimum — no collision, no projectile, no velocity.

```rust
struct AbstractDieselPlugin;

impl Plugin for AbstractDieselPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AbstractBackend::plugin_core());
        app.add_systems(Update,
            propagate_observer::<AbstractBackend>
                .in_set(DieselSet::Propagation));
        app.add_systems(Update, (
            spawn_observer::<AbstractBackend>,
            print_effect::<()>,
            modifier_set_system::<()>,
            instant_set_system::<()>,
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

Lightest possible dependency set. No physics, no spatial crate.

---

# Pokemon Example

A turn-based battle between two teams of 3 creatures. Menu-driven move
selection. No spatial positioning — creatures exist in "battle slots" and
targeting is purely team-relative.

This example is deliberately simple. The point is to show diesel working in a
completely non-spatial context.

## Scene

- UI-only: no 3D/2D world to navigate
- Left side: player's 3 creatures (sprites + HP bars)
- Right side: enemy's 3 creatures (sprites + HP bars)
- Bottom: move selection menu
- Center: battle log text

## Creature Setup

```
Creature entity:
  - Team(0 or 1)
  - BattleSlot(0..2) — position in party
  - Attributes:
      "HP"      => species-dependent (e.g. 45, 60, 35)
      "MaxHP"   => same
      "Attack"  => species-dependent
      "Defense" => species-dependent
      "Speed"   => species-dependent (determines turn order)
  - CreatureName("Flamander" / "Aquadon" / "Thornyx" etc.)
  - Invokes, known abilities
```

Player team: 3 creatures with 2 moves each.
Enemy team: 3 creatures, AI-controlled.

## Turn System

```rust
enum BattlePhase { SelectMove, Execute, CheckFaints, NextTurn }
```

Turn order determined by `"Speed"` attribute. Faster creatures go first.
Each creature acts once per round.

## Moves / Templates

### 1. Tackle (single-target damage)

```
State machine: Ready -[StartInvoke]-> Hit -> Ready
  Hit subeffects:
    - Gatherer: AllEnemies, Filter: count=1  (target picked by input/AI)
    - instant!{ "HP" -= "Attack@attacker - Defense@target" }
```

The simplest possible move. Demonstrates single-target damage.

### 2. Heal (single-ally restore)

```
State machine: Ready -[StartInvoke]-> Heal -> Ready
  Heal subeffects:
    - Gatherer: Self_
    - instant!{ "HP" += 20.0 }  (capped at MaxHP by game logic)
```

### 3. Earthquake (all-enemies AoE)

```
State machine: Ready -[StartInvoke]-> Quake -> Ready
  Quake subeffects:
    - Gatherer: AllEnemies (no count filter — hits everyone)
    - instant!{ "HP" -= "Attack@attacker * 0.6" }
```

Lower per-target damage but hits all enemies. Demonstrates AoE in abstract space.

### 4. Protect (self-buff)

```
State machine: Ready -[StartInvoke]-> Shield -> Ready
  Shield subeffects:
    - Gatherer: Self_
    - Spawn a PAE "protect_effect" on self
```

PAE `protect_effect`:
  - `ActivatedModifiers`: `"Defense" += 15.0`
  - Duration: lasts 2 turns (repeater counting turns, then OnComplete -> unapply)

Demonstrates PAEs in abstract context.

### 5. Poison Sting (damage + status)

```
State machine: Ready -[StartInvoke]-> Sting -> Ready
  Sting subeffects:
    - Gatherer: AllEnemies, Filter: count=1
    - instant!{ "HP" -= 5.0 }
    - Spawn a PAE "poison_effect" on target
```

PAE `poison_effect`:
  - Repeater: 3 ticks (one per turn)
  - Each tick: instant!{ "HP" -= 3.0 }
  - After 3 ticks: OnComplete -> unapply

Demonstrates damage-over-time as a repeating PAE.

## Enemy AI

Simple priority:
1. If any ally below 30% HP and has Heal: heal
2. If all enemies alive and has AoE: use AoE
3. Otherwise: Tackle a random enemy

## Input

Menu-driven:
```
> Select move for Flamander:
  [1] Tackle
  [2] Earthquake

> Select target:
  [1] Enemy Aquadon (HP: 40/60)
  [2] Enemy Thornyx (HP: 35/35)
```

Keyboard number keys to select. Could use bevy_egui or just map to key presses.

## Systems

1. `turn_order_system` — sort creatures by Speed, determine who goes next
2. `player_select_system` — menu input for move + target selection
3. `ai_select_system` — enemy AI picks move + target
4. `execute_move_system` — fire StartInvoke for the selected ability
5. `faint_check_system` — creatures with HP <= 0 are "fainted" (removed from battle)
6. `battle_end_system` — check if one team is fully fainted -> victory/defeat
7. `ui_system` — render creature sprites, HP bars, move menu, battle log

## What This Demonstrates

- **Zero-dimensional backend** (`Pos = ()`, no space at all)
- **Logical targeting** (AllEnemies, AllAllies, Self — no distance)
- **Turn-based combat** without any physics
- **Status effects as PAEs** (Poison with repeating damage)
- **Buff/debuff** (Protect as a timed PAE)
- **Stat-driven damage** (Attack vs Defense via gauge expressions)
- **Simplest possible backend** (proves the abstraction doesn't *require* space)
