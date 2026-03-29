
## Your first statechart
Statecharts can be defined in code or they can be defined in a bevy scene file. The editor makes it easy to inspect, edit, and save statecharts to scene files. However there are a number of reasons that you would not want to define a statechart as a scene asset. The primary reasons are:
- Scene assets are spawned asynchronously. If you can tolerate a frame delay on spawning your statechart that's fine, but in many cases you don't want that delay.
- Gearbox statecharts work better if they are the top-level entity in the hierarchy. For example, if you have a `Player` entity with all of the components needed to make the `Player` work, you want the `StateMachine` component to be on that top-level `Player` entity. This pattern is very hard to achieve using bevy scene assets because scenes are spawned as separate entities. 

### Setting up your statechart in rust:
Defining statecharts in text is simple and the recommended approach for the time being. Lets get started with a player example. This is a somewhat complicated case. I will go through it step by step explaining my thought process and hopefully by the end you understand how you should use gearbox:

First, we'll spawn a player. The details of this entity are not important:
```rust
let player_entity = commands.spawn((
  Player,
  Collider::capsule(1.0, 0.5),
  Hitpoints { max: 100.0, current: 100.0 },
)).id();
```

I like to separate my statechart building logic out into a separate function, which I use to "patch" the first entity. This allows me to keep statechart stuff self contained. It is not strictly necessary.
```rust
// Take an entity and add a character state machine to it.
pub fn add_character_sm(commands: &mut Commands, entity: Entity) {
  // do everything in a `with_children` block. This ensures that all states and
  // edges are spawned as a child of the root entity, which lets them be automatically
  // cleaned up.
  // Use with_children to spawn all of our state entities and insert them into the entity.
  // This will make our state and edge entities children of the root, which means they will
  // be despawned when the root is despawned.
  commands.entity(entity).with_children(|c| {
    // First, spawn all of our state entities. We spawn these first because there are
    // some dependencies between states can be complicated, and it's better to have all
    // of our state entities up front. 
    let alive = c.spawn_empty().id();
    let dead = c.spawn_empty().id();
    let standing = c.spawn_empty().id();
    let jumping = c.spawn_empty().id();

    // Our character can be alive, dead, standing, or jumping. Importantly, they can only
    // jump or stand if they are alive. So standing and jumping are actually sub-states of
    // the alive state. Notice alive and dead are SubstateOf(entity), while standing and
    // jumping are SubstateOf(alive).

    // Note: The root entity (here called `entity`) is also a state entity. This isn't
    // important for this example, but it's worth noting.
    let commands = c.commands_mut();

    commands.entity(alive).insert((
      Name::new("Alive"),
      SubstateOf(entity),
      InitialState(alive),
    ));

    commands.entity(dead).insert((
      Name::new("Dead"),
      SubstateOf(entity),
    ));

    commands.entity(standing).insert((
      Name::new("Standing"),
      SubstateOf(alive),
    ));

    commands.entity(jumping).insert((
      Name::new("Jumping"),
      SubstateOf(alive),
    ));

    // It is extremely important to insert the StateMachine component after all of your
    // InitialState components. Learn more in the !!! FOOTGUN ALERT !!! section below.
    commands.entity(entity).insert((
      StateMachine::new(),
      InitialState(alive),
    ));

    // Next, we will set up our edges. Edges are how we get from one state to another.
    // Edges are simple. They have a source, a target, and a trigger event.
    // These events would require accompanying systems to fire them. You might have an
    // input system that fires `Jump` when the player presses the jump button. You might
    // have a physics system that fires `Land` when the player lands on the ground. You
    // might have a death system that fires `Die` when the players life reaches 0.
    c.spawn((
      Name::new("Standing -> Jumping"),
      Source(standing),
      Target(jumping),
      EventEdge::<Jump>::default(),
    ));

    c.spawn((
      Name::new("Jumping -> Standing"),
      Source(jumping),
      Target(standing),
      EventEdge::<Land>::default(),
    ));

    c.spawn((
      Name::new("Alive -> Dead"),
      Source(alive),
      Target(dead),
      EventEdge::<Die>::default(),
    ));
  });
}

...

// after spawning the player entity, we can do this:
add_character_sm(&mut commands, player_entity);
```

Here's a definition for one of our events:
```rust
#[derive(EntityEvent, Clone, SimpleTransition)]
pub struct Jump {
    #[event_target]
    pub target: Entity,
}
```

Transition events must implement `EntityEvent`, `Clone`, and either:

- derive `SimpleTransition` (auto-implements `TransitionEvent` and auto-registers via inventory), or
- manually implement `TransitionEvent` and decorate the type with `#[transition_event]` (registers it for auto-wiring).

With either approach, no manual app registration is required; `GearboxPlugin` discovers and installs transition handlers automatically.

### On using `StateComponent`s

We have our state machine, but right now it's not really usable. We can fire our events and the state machine will change state based on the states and edges we gave it, but we don't have a straightforward way to tell what state it is in from the outside. For instance, what if I want to find all jumping characters so my physics can act on them differently? Ideally, I could do something like this:

```rust
fn falling_system(
  mut q_jumping: Query<&mut Velocity, With<Jumping>>,
) {
  for mut velocity in q_jumping.iter() {
    // ...
  }
}
```

To accomplish this, we can use a `StateComponent`. `StateComponent`s basically take some component data and clone it to the root while a given state is active, removing it from the root when the state is no longer active. Lets change our `Jumping` state to this:
```rust
#[derive(Clone, Component)]
pub struct Jumping;

...

commands.entity(jumping).insert((
  Name::new("Jumping"),
  SubstateOf(alive),
  StateComponent(Jumping),
));
```
Now, while the `Jumping` state is active, the root will have a `Jumping` component added to it. Now our `With<Jumping>` query above can find our jumping characters!

Note: There is also a `StateInactiveComponent` which is the opposite of the `StateComponent`. While the state is inactive, it will attach its component data to the root, removing it once the state becomes active. 

### On using `EnterState` / `ExitState`

Another way to hook logic into your state machine is via the `EnterState` and `ExitState` events. For example:
```rust
fn on_enter_jumping(
  on_enter: On<EnterState>,
  mut q_velocity: Query<&mut Velocity>,
) {
  // `EnterState and `ExitState` have a .state_machine property which is the character in
  // this case.
  let character_entity = on_enter.state_machine;
  let Ok(mut velocity) = q_velocity.get_mut(character_entity) else {
    return;
  };

  // apply a y impulse
}

...

// our modified jumping state:

commands.entity(jumping).insert((
  Name::new("Jumping"),
  SubstateOf(alive),
)).observe(on_enter_jumping);
```

### On using parameter edges

Right now we have a system somewhere that checks our character's `Hitpoints` and fires a `Die` event when `current <= 0`. However, there is a better way using parameters. Parameters let edges be driven by component data without you manually firing events.

We'll derive a boolean parameter from `Hitpoints` and let an Always edge transition when that parameter matches a condition.

1) Define a marker type and bind it to your source component so the parameter can be synced automatically. Add the attribute to auto-register guards and sync:

```rust
#[derive(Clone, Component)]
pub struct Hitpoints { pub max: f32, pub current: f32 }

// Marker type for our boolean parameter
#[derive(Clone)]
#[gearbox_param(kind = "bool", source = Hitpoints)]
pub struct IsDead;

// Bind the parameter to Hitpoints so it evaluates to (hp.current <= 0)
impl BoolParamBinding<Hitpoints> for IsDead {
  fn extract(hp: &Hitpoints) -> bool { hp.current <= 0.0 }
}
```

2) Ensure the root (player) entity has the parameter component so it can be synced:

```rust
let player_entity = commands.spawn((
  Player,
  Collider::capsule(1.0, 0.5),
  Hitpoints { max: 100.0, current: 100.0 },
  BoolParam::<IsDead>::default(), // <- parameter lives on the root
)).id();
```

3) Change the `Alive -> Dead` edge to be parameter-driven. Instead of `EventEdge<Die>`, make it an `AlwaysEdge` guarded by `BoolEquals::<IsDead>::new(true)`. The guard is automatically maintained by a system and will unblock the edge when `IsDead == true`.

```rust
c.spawn((
  Name::new("Alive -> Dead"),
  Source(alive),
  Target(dead),
  AlwaysEdge,                       // fires when guards pass
  BoolEquals::<IsDead>::new(true),  // guard maintained from BoolParam<IsDead>
));
```

With this setup:
- While `Alive` is active, the `AlwaysEdge` is checked every time guards change.
- When `Hitpoints.current <= 0`, the sync makes `BoolParam<IsDead> == true` and the guard system removes the blocking guard from the edge.
- The edge becomes eligible and immediately transitions to `Dead` without you firing any events.


### On using event payloads

`EnterState` and `ExitState` events are powerful but they lack context. Custom transition events with payloads let us add whatever context we want to an event. Imagine a damage system where a target can only be damaged in certain states. One way you could accomplish this is via payloads. Payloads are secondary events fired if an event is successful. Here is an `Attacked` event, which will fire a secondary `DoDamage` event if the target can be damaged (i.e., they are `Alive`).

1) Define a trigger event and map it to an Entry payload via the `TransitionEvent` trait.

```rust
#[derive(EntityEvent, Clone)]
#[transition_event]
pub struct Attacked { #[event_target] pub target: Entity, pub amount: f32 }

#[derive(EntityEvent, Clone)]
pub struct DoDamage { #[event_target] pub target: Entity, pub amount: f32 }

impl TransitionEvent for Attacked {
  type EntryEvent = DoDamage;
  fn to_entry_event(&self) -> Option<Self::EntryEvent> {
    Some(DoDamage { target: self.target, amount: self.amount })
  }
}
```

2) Apply the damage only if the edge actually fires.:

```rust
fn do_damage_on_entry(
  damage: On<DoDamage>,
  mut q_hp: Query<&mut Hitpoints>,
) {
  let target = damage.target;
  let amount = damage.amount;
  if let Ok(mut hp) = q_hp.get_mut(target) {
    hp.current -= amount;
  }
}
```

3) Prefer an internal self-transition on `Alive` so child states (e.g., `Standing`/`Jumping`) remain undisturbed. When Alive, it will accept damage; when Dead, there is no such edge:

```rust
// Add an internal self-loop on Alive: consumes Attacked without re-entering Alive/children
c.spawn((
  Name::new("Attacked"),
  Source(alive),
  Target(alive),
  EventEdge::<Attacked>::default(),
  EdgeKind::Internal, // Internal edges keep current substate (Standing/Jumping) intact. 
  // Note: Edges are external by default.
));
```

Notes:
- Damage updates `Hitpoints.current`; the existing `BoolParam<IsDead>` sync plus `apply_bool_param_guards::<IsDead>` will automatically enable the Alive -> Dead `AlwaysEdge` when `current <= 0`.
- Sending `Attacked { target: defender_root, amount }` is safe: if the defender is `Dead`, there’s no `EventEdge::<Attacked>`, so no Entry payload is emitted and no damage is applied.
