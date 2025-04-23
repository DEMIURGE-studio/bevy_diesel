# Test Cases
Test cases represent different ways a user would try to use the stat system, or put another way, what kinds of games they would make and how Diesel can be used to make that kind of game. As of writing Diesel is incomplete, but we have a working approximation of the stat system (bevy_gauge / bevy_diesel_core). To that end we should start putting together some examples, or at least conceptualizing how our stat system would work for some test cases. 

Eventually test cases will become submodules or examples, which will be used to showcase "blessed" ways to handle certain common types of mechanics. Here are some of the cases I would like to be able to handle:

### Simple Case (Balatro, Nubby's Number Factory)

#### Challenges:
1. Create a Balatro-like stack of effects that iteratively modify some final outcome.

### RTS (Age of Empires, Starcraft)

#### Challenges:
1. Lots of entities. An RTS should be able to handle between 1,000 and 2,000 stat entities at the top end, with live updates to life totals, etc. 
2. Global upgrades. When you get a "+1 defense for melee units" upgrade at your barracks, that buff is applied to all of your melee units. 

### FPS (Call of Duty, Destiny, Borderlands)

#### Challenges:
1. Guns may grant stats like normal equipment, but it may also make sense for it to have stats of its own that it does not grant to the wielder. In UE5 GAS parlance the wielder and gun would have separate AttributeSets and maybe even be separate AbilitySystemComponents. For example, it may make more sense for a gun to have a "fire bullet" ability as opposed to granting an ability to the owner.

### Path of Exile

#### Challenges:
Path of Exile has a number of different stats that do interesting things. Its innevitable that some of these things will fall outside of the stat system, or at least will not be directly supported features. With that in mind, it would be nice to handle as many of them as possible.
1. Chaos Innoculation - Makes you immune to chaos damage and sets your maximum life to 1. To me this means that Life and ChaosResistance would have an override, with chaos resistance being locked at 100% and life being locked at 1. 
2. Damage stats come in a number of flavors; Increased damage, increased damage with fire, increased damage with fire spells, increased damage with spells while wielding a staff. All of these different different stats are ultimately the same, and should be combined into a single number. Querying on these different types should be as intuitive as any other query type.
3. Bonus X from Y. For example, your character gets bonus life per strength.
4. Stat-dependent modifiers, like Pain Innoculation, which only applies while below 50% life.
5. Local vs Global modifiers (specifically for weapons)