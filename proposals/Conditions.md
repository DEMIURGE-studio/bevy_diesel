# Conditions Proposal

## Conditions
Conditions are how we decide if an effect entity can "go off" and if that entity propagates cue events to its child effect entities.

## Problem
Conditions are not a trivial problem to solve in ECS. Imagine you want to activate an effect with a condition. The most straightforward way to do this would be to have the condition in the effect's query like so:

```rust
Query<(&FooCondition, &BarEffect)>

if (foo_condition) { apply bar_effect }
```

The problem is that now your effects and conditions are tightly coupled. Adding new effects or conditions becomes more cumbersome. Ideally, we could define separate effects and conditions in different files with a common interface for deciding if something "passed" or not.

## Proposed Solutions
I will propose several possible solutions or Bevy features (some of which could be mixed and matched) to solve this class of problem. Some of these solutions come with problems of their own which I will try to cover. The order below is arbitrary and does not indicate any level of support on my end.

### StatRequirements Only
This "solution" suggests that a `StatRequirements` component is all that a user would need to decide whether a cue could fire or not. This would massively simplify things but leaves a lot on the table.

### State Proxy Component
You can have any number of conditions, but all of their collective states are aggregated to a single component. It would look something like this:

```rust
// foo_condition_system - writes its state to ActiveState
Query<(&FooCondition, &mut ActiveState)>

if foo_condition { active_state = Active } else { active_state = Inactive }

// bar_condition_system - writes its state to ActiveState
Query<(&BarCondition, &mut ActiveState)>

if bar_condition { active_state = Active } else { active_state = Inactive }

// my_effect_system - reads ActiveState to decide if it should fire
Query<(&MyEffect, &ActiveState)>

if active_state == Active { apply my_effect }
```

This approach decouples conditions and effects, making them easier to define and extend separately. However, it raises several questions.

- In the above model, when a condition is not met it sets the active state to 'inactive.' This effectively means that all of our conditions are forced into a single "and" block where all conditions must be met. Do we want to be able to represent "or" blocks?
    - Some "or" blocks can be handled per-component. For example StatRequirements could have a requirement like `"mana > 100 || life > 100"` 

### Direct World Access
In bevy you are able to [access the world directly](https://bevy-cheatbook.github.io/programming/world.html). This gives you access to all of your data at once and allows you to maniplulate or query it in arbitrary ways. You could define

You could make Condition components derive [`FromWorld`](https://docs.rs/bevy/latest/bevy/ecs/world/trait.FromWorld.html) to give users a formalized way to generate condition state from world state.

I'm skeptical of this approach because it basically abandons everything that makes ECS good in the name of expediency. That is acceptible when we're talking about operations that are very fast, or which happen very infrequently. I do not think that necessarily applies here. If we went this route we'd probably want to take inspiration from the way [seldom_state](https://github.com/Seldom-SE/seldom_state) lets you define arbitrary state transition conditions.

### EventMutator
Bevy events can be interacted through 3 main tools:
- `EventWriter<T>` allows you to generate new events
- `EventReader<T>` allows you to read events
- [`EventMutator<T>`](https://docs.rs/bevy/latest/bevy/prelude/struct.EventMutator.html) allows you to read and mutate events.

It's pretty easy to imagine a "TryGoOff" event, which is then caught by various condition-systems. These systems write state to the event based on their conditions. Then it gets to a final system which reads the event. If the state in the event is valid, a "GoOff" event is fired. This event is the one caught by effect systems.

### QueryBuilder
Honorable mention to [QueryBuilder](https://docs.rs/bevy/latest/bevy/ecs/prelude/struct.QueryBuilder.html), billed as a way for scripting languages to hook into bevy queries, it allows dynamic queries to be defined at runtime. This could be useful but it needs a `&mut World` to work so its not clear to me what (if any) advantage it has over regular world access besides maybe usability?

## Conclusion
Right now I think `EventMutator` is our best bet. Here's how I propose it would work. 

1. `ConditionComponent`s would have an id property. The id would be a unique `&'static str` that represents that component type, probably its fully qualified type path. 
2. `TryGoOff` would be an event that contained an expression. This expression would represent all of the conditions the effect depends on. This would take the form of a boolean expression. For instance `"stat_requirements | mana_cost"` would indicate that this effect can be activated if the `StatRequirement` or `ManaCost` conditions are met. 
3. If all conditions are met, the `GoOff` event will be fired. 

I believe this to be the best of all of the above systems. Here are some pros and cons:
- Pros
    1. Event systems do not need to be ordered (except for that they are after `TryGoOff` events are generated and `GoOff` events are consumed)
    2. You can express your conditions in data as arbitrary boolean conditions like you're writing rust. Or, more accurately, the same way that you define stat requirements or other expression types.
    3. Events are fast (compared to direct world access)
    4. Conditions are completely modular, with no external dependencies. 
- Cons
    1. In ECS, writable access is blocking. This means that the conditional systems (which all need write access to the `TryGoOff` events) cannot be parallelized. 