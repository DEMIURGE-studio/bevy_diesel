use bevy::prelude::*;

pub trait SpatialBackend {
    /// The target type for this backend (e.g., contains an Entity and/or a Vec3).
    type Target;

    /// The configuration structure for generating targets.
    type Config;

    /// Generates targets using the provided configuration.
    /// All the complex logic lives here, where it has access to the queries.
    fn generate_targets(
        &mut self,
        config: &Self::Config,
        origin: &Self::Target,
        cue_target: &Self::Target,
        effect_entity: Entity,
    ) -> Vec<Self::Target>;
}