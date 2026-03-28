# bevy_diesel

A data-driven ability engine for Bevy. Analogous to UE5's Gameplay Ability System (GAS), but designed around Bevy's ECS.

Diesel lets you build abilities by composing reusable templates. A fireball ability spawns an explosive projectile, which spawns an explosion on hit, which deals damage in a radius - each piece is a small, self-contained template that references others by name. The same explosion template works whether it came from a fireball, a firestorm, or a landmine.

## Why diesel

Most games with abilities end up building the same infrastructure: state machines for ability lifecycles, target resolution, effect propagation, stat-driven guards, spawning, cleanup. Diesel provides all of this as a coherent framework so you can focus on designing abilities rather than building plumbing.

Diesel makes it easy to:

- **Compose abilities from reusable parts.** Define an explosion once, reference it from any ability that needs one. Templates are just functions that build entity hierarchies - wire them together by name.
- **Drive ability behavior with data.** A projectile's lifetime, a buff's stat requirements, a damage formula - these are attributes and expressions, not hardcoded logic. Change a number, change the behavior.
- **Define ability lifecycles declaratively.** Ready, invoking, cooldown, channeling - state machines with event-driven transitions. Wire edges between states, attach effects to state entry, let the framework handle the rest.
- **Resolve targets generically.** "Nearest 3 enemies within 5 meters" or "random point in a circle around the caster" - the target pipeline handles resolution, gathering, and filtering without coupling to a specific physics engine.

## How it works

Diesel builds on two companion crates:

- [bevy_gearbox](https://github.com/DEMIURGE-studio/bevy_gearbox) provides hierarchical state machines (statecharts) with event-driven transitions, guards, parallel regions, and history. Abilities use these for their lifecycle - ready, invoking, repeating, done.
- [bevy_gauge](https://github.com/DEMIURGE-studio/bevy_gauge) provides a dependency-graph attribute system with modifiers, expressions, and cross-entity references. Abilities use these for stat requirements, damage formulas, and resource tracking (like projectile life).

Diesel's core is generic over spatial representation - it doesn't know about `Vec3` or physics engines. Instead, you provide (or use) a **spatial backend** that implements the `SpatialBackend` trait, telling diesel how to resolve positions, gather targets, and filter results in your game's coordinate system.

`diesel_avian3d` is the reference backend for 3D games using the Avian physics engine. It provides projectile effects, ballistic math, collision-to-event bridging, and a concrete `Vec3` implementation of the spatial pipeline. Use it directly, or reference it when building your own backend for a different physics engine, a 2D game, or a grid-based system.

Existing backends may not be the right fit for your game. You're encouraged to fork and adapt them - they're intentionally thin wrappers over the generic core.

## Quick start

```rust
use diesel_avian3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(AvianBackend::plugin())
        .add_systems(Startup, register_templates)
        .run();
}

fn register_templates(mut registry: ResMut<TemplateRegistry>) {
    registry.register("fireball", fireball_template);
}
```

## Templates

The core authoring pattern is the **template** - a function that builds an entity hierarchy representing an ability, an effect, or a piece of one:

```rust
fn fireball_template(commands: &mut Commands, entity: Option<Entity>) -> Entity {
    let entity = entity.unwrap_or_else(|| commands.spawn_empty().id());

    commands.entity(entity).with_children(|parent| {
        let ready = parent.spawn((Name::new("Ready"), SubstateOf(entity))).id();
        let invoke = parent.spawn((Name::new("Invoke"), SubstateOf(entity))).id();

        // On invoke, spawn a projectile at the invoker aimed at their target
        parent.spawn((
            SubstateOf(invoke),
            SubEffectOf(invoke),
            InvokedBy(entity),
            SpawnConfig::at_invoker("explosive_projectile")
                .with_target_generator(TargetGenerator::at_invoker_target()),
        ));

        // State machine wiring
        parent.spawn((Source(ready), Target(invoke), EventEdge::<StartInvoke>::default()));
        parent.spawn((Source(invoke), Target(ready), AlwaysEdge));

        parent.commands_mut().entity(entity).insert((
            Ability,
            StateMachine::new(),
            InitialState(ready),
        ));
    });

    entity
}
```

Templates reference other templates by name via `SpawnConfig`. The explosive projectile template references an explosion template. The firestorm ability template references the explosive projectile template. Composition all the way down.

## Examples

See `diesel_avian3d/examples/fireballs.rs` for a complete working example with fireball and firestorm abilities built from shared templates - projectile physics, collision handling, gauge-driven projectile life, and team-based collision filtering.
