# Targetting Proposal

## Targetting
Targetting refers to information passed into an effect about an entity, location, or set of entities/locations that effect applies to. An example is damage. You might have an explosion that gathers all the enemies in an area and then deals damage to them. That explosion effect should actually be broken into parts.
1. The part that gathers targets (using physics or whatever)
2. The VFX (explosion animation)
3. The damage

Targetting refers to the first part: Gathering the targets.