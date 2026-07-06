# bevy_diesel

A data-driven ability engine for Bevy. Inspired by UE5's Gameplay Ability System (GAS), designed around Bevy's ECS.

Diesel lets you build abilities by composing reusable templates. A fireball ability spawns an explosive projectile, which spawns an explosion on hit, which deals damage in a radius - each piece is a small, self-contained template that references others by name. The same explosion template works whether it came from a fireball, a firestorm, or a landmine.

## Why diesel

Abilities are a hard problem. Without a framework, you end up with a mess of bespoke components and one-off systems for every ability - each with its own targeting logic, lifecycle management, and effect propagation. Diesel gives you a coherent framework for all of it so you can focus on designing abilities rather than reinventing plumbing.

Diesel makes it easy to:

- **Compose abilities from reusable parts.** Define an explosion once, reference it from any ability that needs one. Templates are just functions that build entity hierarchies - wire them together by name.
- **Drive ability behavior with data.** A projectile's lifetime, a buff's stat requirements, a damage formula - these are attributes and expressions, not hardcoded logic. Change a number, change the behavior.
- **Define ability lifecycles declaratively.** Ready, invoking, cooldown, channeling - state machines with message-driven transitions. Wire edges between states, attach effects to state entry, let the framework handle the rest.
- **Resolve targets generically.** "Nearest 3 enemies within 5 meters" or "random point in a circle around the caster" - the target pipeline handles resolution, gathering, and filtering without coupling to a specific physics engine.

## How it works

Diesel builds on two companion crates:

- **bevy_gearbox** provides hierarchical state machines (statecharts) with message-driven transitions, guards, parallel regions, and history. Diesel uses this to manage ability lifecycle. "on cooldown" is a gearbox state.
- **bevy_gauge** provides a dependency-graph attribute system with modifiers, expressions, and cross-entity references. Abilities use these for stat requirements, damage formulas, and resource tracking (like projectile life).

Diesel's core is generic over spatial representation - it doesn't know about `Vec3` or physics engines. Instead, you use a **spatial backend** that implements the `SpatialBackend` trait, telling diesel how to resolve positions, gather targets, and filter results in your game's coordinate system.

`diesel_avian3d` is the reference backend for 3D games using the Avian physics engine. It provides projectile effects, ballistic math, collision-to-event bridging, and a concrete `Vec3` implementation of the spatial pipeline. Use it directly, or reference it when building your own backend for a different physics engine, a 2D game, or a grid-based system.

## What diesel adds over gearbox and gauge

`bevy_gearbox` gives you state machines; `bevy_gauge` gives you an attribute graph. Neither knows anything about *abilities* - targeting, space, effects, or how the two fit together. Diesel is that layer:

- **A targeting pipeline.** Resolve "the invoker's target", "everything within 5m", or "a random point in a circle" as `TargetType -> offset -> gather -> filter`, generic over a `SpatialBackend` so it isn't tied to `Vec3` or any one physics engine.
- **An effect pipeline.** Attach effects to a state with `SubEffectOf`; when the state activates, diesel walks the effect tree and delivers each effect - spawn, damage, despawn, impulse - to its resolved target(s). Gearbox transitions *happen*; diesel turns them into *effects on things*.
- **Composition by name.** Templates are scene factories in a registry, and abilities spawn each other by id (`fireball` -> `explosive_projectile` -> `explosion`), so one explosion is reused everywhere.
- **The wiring between the two.** Diesel registers the ability hierarchy as gauge sources (`@invoker`, `@ability`, `@root`) so a projectile can read `Damage@ability`; drives gearbox edge delays from gauge (`Cooldown@ability`); gates transitions on gauge requirements; and sequences effect stat-changes *before* guard evaluation, so a hit is visible to an always-edge the same frame.

On top of that come the ability-shaped primitives: cooldowns, repeater volleys, persistent stat effects with requirement-gated activation, and combat-event propagation.

## Quick start

```rust
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use diesel_avian3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(AvianBackend::plugin())
        .add_systems(Startup, register_templates)
        .run();
}

// Templates are scene factories - `fn() -> impl Scene` - registered by name.
// Abilities reference the templates they spawn by that same name.
fn register_templates(mut registry: ResMut<TemplateRegistry>) {
    registry.register("fireball", || Box::new(fireball()));
    registry.register("explosive_projectile", || Box::new(explosive_projectile()));
    registry.register("explosion", || Box::new(explosion()));
}
```

## Templates

The core authoring pattern is the **template** - a `fn() -> impl Scene` that builds one piece of an ability as a [`bsn!`](https://docs.rs/bevy/latest/bevy/scene/) scene: an ability shell, a projectile, an effect. Templates reference each other by name, so the same explosion works whether it came from a fireball, a firestorm, or a landmine.

An **ability** is an `invoked` shell (Ready -> Invoking -> Cooldown) whose Invoking phase fires an effect - here a `single_shot` that spawns a projectile at the invoker, aimed at their target:

```rust
fn fireball() -> impl Scene {
    invoked("Fireball", 0.8, |root| {
        single_shot(root, bsn! {
            SpawnConfig::invoker_offset_target(
                "explosive_projectile",
                Vec3Offset::Fixed(DirectionOffset::new(Dir3::Y, 1.5)),
                TargetGenerator::at_invoker_target(),
            )
        })
    })
}
```

The **projectile** is its own small state chart. It flies until it collides, spawns an `explosion` at the impact point, then despawns. Effects attach to a state with `SubEffectOf(#State) InvokedBy(#Root)`; a terminal `#Done` state carrying `GoOffConfig::root() DespawnEffect` tears the projectile down - `DespawnEffect` is just another effect, despawning whatever the pipeline resolves as its target (here the root):

```rust
fn explosive_projectile() -> impl Scene {
    bsn! {
        #Root
            Name::new("ExplosiveProjectile")
            ProjectileEffect::new(20.0)
            StateMachine InitialState(#Flying)
        Substates [
            #Flying Transitions [
                (Target(#Hit) MessageEdge::<CollidedEntity>)
            ],
            #Hit Substates [
                (SubEffectOf(#Hit) InvokedBy(#Root) SpawnConfig::passed("explosion"))
            ] Transitions [
                (Target(#Done) AlwaysEdge)
            ],
            #Done GoOffConfig::root() DespawnEffect,
        ]
    }
}
```

The **explosion** gathers targets and applies an effect. `GoOffConfig::default()` fires the effect pipeline when the state activates; `TargetMutator::root_gathering` rewrites the single entry-target into "every entity within radius". The gather radius is gauge-driven, so upgrading `Area` scales every explosion the ability ever spawns:

```rust
fn explosion() -> impl Scene {
    bsn! {
        #Root
            Name::new("Explosion")
            StateMachine InitialState(#Active)
        Substates [
            #Active GoOffConfig::default() Substates [
                #AoE SubEffectOf(#Active) InvokedBy(#Root)
                    TargetMutator::root_gathering(AvianGatherer::AllEntitiesInRadius(3.0))
                    template(|_| Ok(attributes! { "TargetMutator.gatherer" => "Area@ability" }))
                Substates [
                    (SubEffectOf(#AoE) InvokedBy(#Root)
                        template(|_| Ok(instant! { "Health.current" -= "Damage@ability" })))
                ],
            ],
        ]
    }
}
```

Composition all the way down: `fireball` references `explosive_projectile`, which references `explosion`. Swap the ability's spawn leaf for a `repeater` volley and you have a `firestorm` that drops the same projectiles in waves - the explosion never changes.

## Examples

See `backends/diesel_avian3d/examples/fireballs.rs` for a complete working example with fireball and firestorm abilities built from shared templates - projectile physics, collision handling, gauge-driven projectile life, and team-based collision filtering.

## Dependencies


```toml
[dependencies]
bevy_diesel = "0.4"
```

## Version Table

| Bevy | Diesel | bundled bevy_gauge | bundled bevy_gearbox |
| ---- | ------ | ------------------ | -------------------- |
| 0.19 | 0.4    | 0.5                | 0.8                  |

The gauge/gearbox versions are what `bevy_diesel 0.4` pulls in transitively -
listed for reference only; you don't declare them.

## License

Bevy diesel is free and open source. All code in this repository is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.
