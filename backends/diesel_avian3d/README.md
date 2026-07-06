# diesel_avian3d

The reference [Avian](https://github.com/Jondolf/avian) 3D spatial backend for
[bevy_diesel](https://github.com/DEMIURGE-studio/bevy_diesel).

Diesel's core is generic over spatial representation. This crate implements the
`SpatialBackend` trait for `Vec3` + the Avian physics engine, and adds the pieces
a 3D action game needs: ballistic and linear projectile effects, ballistic math,
and a collision-to-message bridge that turns Avian contacts into diesel ability
events.

```rust
use bevy::prelude::*;
use diesel_avian3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(AvianBackend::plugin())
        .run();
}
```

See `examples/fireballs.rs` for fireball and firestorm abilities built from
shared templates — projectile physics, collision handling, gauge-driven
projectile life, and team-based collision filtering.

## Dependencies


```toml
[dependencies]
diesel_avian3d = "0.2"
bevy_diesel    = "0.4"
avian3d        = "0.7"
```

## Version Table

| Bevy | diesel_avian3d | bevy_diesel | Avian |
| ---- | -------------- | ----------- | ----- |
| 0.19 | 0.2            | 0.4         | 0.7   |

## License

Dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.
