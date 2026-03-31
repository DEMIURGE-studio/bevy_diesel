use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::reflect::TypePath;

use crate::backend::SpatialBackend;
use crate::effect::{GoOff, GoOffOrigin};
use crate::invoker::InvokedBy;
use crate::pipeline::generate_targets;
use crate::target::{InvokerTarget, Target, TargetGenerator, TargetType};

// ---------------------------------------------------------------------------
// TemplateRegistry
// ---------------------------------------------------------------------------

/// Maps string IDs to template spawning functions.
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

// ---------------------------------------------------------------------------
// SpawnConfig
// ---------------------------------------------------------------------------

/// Configures entity spawning within the effect tree.
#[derive(Component, Clone, Debug)]
pub struct SpawnConfig<B: SpatialBackend> {
    pub template_id: String,
    pub spawn_position_generator: TargetGenerator<B>,
    pub spawn_target_generator: Option<TargetGenerator<B>>,
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

    pub fn invoker(template_id: &str) -> Self {
        Self::new(template_id, TargetType::Invoker)
    }
    pub fn target(template_id: &str) -> Self {
        Self::new(template_id, TargetType::InvokerTarget)
    }
    pub fn spawn(template_id: &str) -> Self {
        Self::new(template_id, TargetType::Spawn)
    }
    pub fn root(template_id: &str) -> Self {
        Self::new(template_id, TargetType::Root)
    }
    pub fn passed(template_id: &str) -> Self {
        Self::new(template_id, TargetType::Passed)
    }
    pub fn at_position(template_id: &str, position: B::Pos) -> Self {
        Self::new(template_id, TargetType::Position(position))
    }
    pub fn at_zero(template_id: &str) -> Self
    where
        B::Pos: Default,
    {
        Self::at_position(template_id, B::Pos::default())
    }

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
    pub fn with_target_generator(mut self, target_generator: TargetGenerator<B>) -> Self {
        self.spawn_target_generator = Some(target_generator);
        self
    }
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

// ---------------------------------------------------------------------------
// Generic spawn events
// ---------------------------------------------------------------------------

macro_rules! spawn_event {
    ($Name:ident) => {
        #[derive(Message, Clone, Debug)]
        pub struct $Name<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static> {
            pub entity: Entity,
            pub target: Target<P>,
        }

        impl<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static>
            bevy_gearbox::GearboxMessage for $Name<P>
        {
            type Validator = bevy_gearbox::AcceptAll;
            fn machine(&self) -> Entity {
                self.entity
            }
        }

        impl<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static> $Name<P> {
            pub fn new(entity: Entity, target: Target<P>) -> Self {
                Self { entity, target }
            }
        }

        impl<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static>
            bevy_gearbox::SideEffect<$Name<P>> for GoOffOrigin<P>
        {
            fn produce(matched: &bevy_gearbox::Matched<$Name<P>>) -> Option<Self> {
                Some(GoOffOrigin::new(
                    matched.target,
                    matched.message.target,
                ))
            }
        }
    };
}

spawn_event!(OnSpawnOrigin);
spawn_event!(OnSpawnTarget);
spawn_event!(OnSpawnInvoker);

// ---------------------------------------------------------------------------
// Generic spawn system
// ---------------------------------------------------------------------------

pub fn spawn_system<B: SpatialBackend>(
    mut reader: MessageReader<GoOff<B::Pos>>,
    q_effect: Query<&SpawnConfig<B>>,
    q_invoker: Query<&InvokedBy>,
    q_child_of: Query<&ChildOf>,
    q_invoker_target: Query<&InvokerTarget<B::Pos>>,
    template_registry: Res<TemplateRegistry>,
    mut ctx: B::Context<'_, '_>,
    mut commands: Commands,
    mut spawn_target_writer: MessageWriter<OnSpawnTarget<B::Pos>>,
    mut spawn_origin_writer: MessageWriter<OnSpawnOrigin<B::Pos>>,
    mut spawn_invoker_writer: MessageWriter<OnSpawnInvoker<B::Pos>>,
) {
    for go_off in reader.read() {
        let effect_entity = go_off.entity;
        let passed = go_off.target;

        let invoker = q_invoker.root_ancestor(effect_entity);
        let invoker_target: Target<B::Pos> = q_invoker_target
            .get(invoker)
            .copied()
            .map(Target::from)
            .unwrap_or_default();
        let Ok(spawn_config) = q_effect.get(effect_entity) else {
            continue;
        };
        let root = q_child_of.root_ancestor(effect_entity);

        let mut spawn_targets = generate_targets::<B>(
            &spawn_config.spawn_position_generator,
            &mut ctx,
            invoker,
            invoker_target,
            root,
            B::Pos::default(),
            passed,
        );

        if spawn_targets.is_empty() {
            continue;
        }

        let target_targets = if let Some(target_generator) = &spawn_config.spawn_target_generator {
            let targets = generate_targets_with_spawn_positions::<B>(
                target_generator,
                &spawn_targets,
                &[passed],
                invoker,
                invoker_target,
                root,
                &mut ctx,
            );
            if targets.is_empty() {
                continue;
            }
            Some(targets)
        } else {
            None
        };

        let parent_entity = if let Some(parent_target_type) = &spawn_config.as_child_of {
            let parent = match parent_target_type {
                TargetType::Invoker => Some(invoker),
                TargetType::InvokerTarget => invoker_target.entity,
                TargetType::Root => Some(root),
                TargetType::Passed => passed.entity,
                TargetType::Spawn => None,
                TargetType::Position(_) => None,
            };
            if parent.is_none() {
                continue;
            }
            parent
        } else {
            None
        };

        for (i, spawn_target) in spawn_targets.iter().enumerate() {
            let spawned_entity = commands.spawn(InvokedBy(invoker)).id();
            B::insert_position(
                &mut commands.entity(spawned_entity),
                &ctx,
                spawn_target.position,
                parent_entity,
            );
            template_registry.get(&spawn_config.template_id).unwrap()(
                &mut commands,
                Some(spawned_entity),
            );

            match (&target_targets, parent_entity) {
                (Some(targets), Some(parent)) => {
                    let target_target = targets
                        .get(i % targets.len())
                        .copied()
                        .unwrap_or(*spawn_target);
                    commands
                        .entity(spawned_entity)
                        .insert((target_target, ChildOf(parent)));
                    spawn_target_writer
                        .write(OnSpawnTarget::new(spawned_entity, target_target));
                }
                (Some(targets), None) => {
                    let target_target = targets
                        .get(i % targets.len())
                        .copied()
                        .unwrap_or(*spawn_target);
                    commands.entity(spawned_entity).insert(target_target);
                    spawn_target_writer
                        .write(OnSpawnTarget::new(spawned_entity, target_target));
                }
                (None, Some(parent)) => {
                    commands.entity(spawned_entity).insert(ChildOf(parent));
                }
                (None, None) => {}
            }

            spawn_origin_writer.write(OnSpawnOrigin::new(
                spawned_entity,
                Target::entity(spawned_entity, spawn_target.position),
            ));

            let invoker_position = B::position_of(&ctx, invoker).unwrap_or_default();
            spawn_invoker_writer.write(OnSpawnInvoker::new(
                spawned_entity,
                Target::entity(invoker, invoker_position),
            ));
        }
    }
}


// ---------------------------------------------------------------------------
// Internal pipeline helper
// ---------------------------------------------------------------------------

fn generate_targets_with_spawn_positions<B: SpatialBackend>(
    generator: &TargetGenerator<B>,
    spawn_targets: &[Target<B::Pos>],
    in_targets: &[Target<B::Pos>],
    invoker: Entity,
    invoker_target: Target<B::Pos>,
    root: Entity,
    ctx: &mut B::Context<'_, '_>,
) -> Vec<Target<B::Pos>> {
    if matches!(generator.target_type, TargetType::Spawn) {
        let mut all_targets = Vec::new();
        for (idx, spawn_target) in spawn_targets.iter().enumerate() {
            let passed_target = in_targets
                .get(idx % in_targets.len())
                .copied()
                .unwrap_or(*spawn_target);
            let mut targets = generate_targets::<B>(
                generator,
                ctx,
                invoker,
                invoker_target,
                root,
                spawn_target.position,
                passed_target,
            );
            all_targets.append(&mut targets);
        }
        all_targets
    } else {
        let mut all_targets = Vec::new();
        for passed_target in in_targets.iter() {
            let mut t = generate_targets::<B>(
                generator,
                ctx,
                invoker,
                invoker_target,
                root,
                B::Pos::default(),
                *passed_target,
            );
            all_targets.append(&mut t);
        }
        all_targets
    }
}
