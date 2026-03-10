use std::collections::HashMap;
use std::marker::PhantomData;

use bevy::prelude::*;

use crate::backend::SpatialBackend;
use crate::target::{TargetGenerator, TargetType};

// ---------------------------------------------------------------------------
// TemplateRegistry
// ---------------------------------------------------------------------------

/// Maps string IDs to spawning functions. Users register their entity templates
/// here; the spawn observer looks them up by `SpawnConfig::template_id`.
///
/// When Bevy scene notation lands this abstraction can be replaced or augmented.
#[derive(Resource, Default)]
pub struct TemplateRegistry {
    templates: HashMap<String, Box<dyn Fn(&mut Commands, Option<Entity>) -> Entity + Send + Sync>>,
}

impl TemplateRegistry {
    pub fn register<F>(&mut self, id: impl Into<String>, template: F)
    where
        F: Fn(&mut Commands, Option<Entity>) -> Entity + Send + Sync + 'static,
    {
        self.templates.insert(id.into(), Box::new(template));
    }

    pub fn get(
        &self,
        id: &str,
    ) -> Option<&(dyn Fn(&mut Commands, Option<Entity>) -> Entity + Send + Sync)> {
        self.templates.get(id).map(|f| f.as_ref())
    }
}

/// Component for configuring entity spawning within the effect tree.
///
/// Contains two generators with different purposes:
/// - `spawn_position_generator` — WHERE to spawn entities
/// - `spawn_target_generator` — WHAT the spawned entities target (optional)
///
/// The actual spawn observer is backend-specific. This type provides the
/// data structure and builder API.
#[derive(Component, Clone, Debug)]
pub struct SpawnConfig<B: SpatialBackend> {
    /// Which template/prefab to spawn (e.g. "fireball", "heal_zone").
    pub template_id: String,
    /// Where to spawn: generates positions from the pipeline.
    pub spawn_position_generator: TargetGenerator<B>,
    /// What the spawned entity targets (optional, separate from spawn position).
    pub spawn_target_generator: Option<TargetGenerator<B>>,
    /// Optionally parent the spawned entity under a resolved entity.
    pub as_child_of: Option<TargetType<B::Pos>>,
    #[allow(dead_code)]
    _phantom: PhantomData<B>,
}

impl<B: SpatialBackend> SpawnConfig<B> {
    fn new(template_id: &str, target_type: TargetType<B::Pos>) -> Self {
        Self {
            template_id: template_id.to_string(),
            spawn_position_generator: TargetGenerator {
                target_type,
                ..Default::default()
            },
            spawn_target_generator: None,
            as_child_of: None,
            _phantom: PhantomData,
        }
    }

    // -- Constructors: spawn at a resolved position (identity gather) --

    /// Spawn at the invoker's position.
    pub fn invoker(template_id: &str) -> Self {
        Self::new(template_id, TargetType::Invoker)
    }

    /// Spawn at the invoker's target position.
    pub fn target(template_id: &str) -> Self {
        Self::new(template_id, TargetType::InvokerTarget)
    }

    /// Spawn at the spawn context position.
    pub fn spawn(template_id: &str) -> Self {
        Self::new(template_id, TargetType::Spawn)
    }

    /// Spawn at the root entity's position.
    pub fn root(template_id: &str) -> Self {
        Self::new(template_id, TargetType::Root)
    }

    /// Spawn at each passed target's position.
    pub fn passed(template_id: &str) -> Self {
        Self::new(template_id, TargetType::Passed)
    }

    /// Spawn at a fixed position.
    pub fn at_position(template_id: &str, position: B::Pos) -> Self {
        Self::new(template_id, TargetType::Position(position))
    }

    // -- Builder methods for spawn position generator --

    pub fn with_offset(mut self, offset: B::Offset) -> Self {
        self.spawn_position_generator.offset = offset;
        self
    }

    pub fn with_gatherer(mut self, gatherer: B::Gatherer) -> Self {
        self.spawn_position_generator.gatherer = Some(gatherer);
        self
    }

    pub fn with_filter(mut self, filter: B::Filter) -> Self {
        self.spawn_position_generator.filter = filter;
        self
    }

    // -- Builder methods for spawn target generator --

    /// Set an optional target generator for the spawned entity's initial targets.
    pub fn with_target_generator(mut self, target_generator: TargetGenerator<B>) -> Self {
        self.spawn_target_generator = Some(target_generator);
        self
    }

    // -- Convenience aliases --

    pub fn at_invoker(template_id: &str) -> Self {
        Self::invoker(template_id)
    }

    pub fn at_target(template_id: &str) -> Self {
        Self::target(template_id)
    }

    pub fn at_root(template_id: &str) -> Self {
        Self::root(template_id)
    }

    pub fn at_passed(template_id: &str) -> Self {
        Self::passed(template_id)
    }

    pub fn at_zero(template_id: &str) -> Self
    where
        B::Pos: Default,
    {
        Self::at_position(template_id, B::Pos::default())
    }

    // -- Parenting --

    pub fn as_child_of_invoker(mut self) -> Self {
        self.as_child_of = Some(TargetType::Invoker);
        self
    }

    pub fn as_child_of_target(mut self) -> Self {
        self.as_child_of = Some(TargetType::InvokerTarget);
        self
    }

    pub fn as_child_of_root(mut self) -> Self {
        self.as_child_of = Some(TargetType::Root);
        self
    }

    pub fn as_child_of_passed(mut self) -> Self {
        self.as_child_of = Some(TargetType::Passed);
        self
    }
}
