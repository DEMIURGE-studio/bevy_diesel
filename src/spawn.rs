use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::reflect::TypePath;

use crate::backend::SpatialBackend;
use crate::effect::GoOff;
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

    pub fn invoker(template_id: &str) -> Self { Self::new(template_id, TargetType::Invoker) }
    pub fn target(template_id: &str) -> Self { Self::new(template_id, TargetType::InvokerTarget) }
    pub fn spawn(template_id: &str) -> Self { Self::new(template_id, TargetType::Spawn) }
    pub fn root(template_id: &str) -> Self { Self::new(template_id, TargetType::Root) }
    pub fn passed(template_id: &str) -> Self { Self::new(template_id, TargetType::Passed) }
    pub fn at_position(template_id: &str, position: B::Pos) -> Self { Self::new(template_id, TargetType::Position(position)) }
    pub fn at_invoker(template_id: &str) -> Self { Self::invoker(template_id) }
    pub fn at_target(template_id: &str) -> Self { Self::target(template_id) }
    pub fn at_root(template_id: &str) -> Self { Self::root(template_id) }
    pub fn at_passed(template_id: &str) -> Self { Self::passed(template_id) }
    pub fn at_zero(template_id: &str) -> Self where B::Pos: Default { Self::at_position(template_id, B::Pos::default()) }

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
    pub fn as_child_of_invoker(mut self) -> Self { self.as_child_of = Some(TargetType::Invoker); self }
    pub fn as_child_of_target(mut self) -> Self { self.as_child_of = Some(TargetType::InvokerTarget); self }
    pub fn as_child_of_root(mut self) -> Self { self.as_child_of = Some(TargetType::Root); self }
    pub fn as_child_of_passed(mut self) -> Self { self.as_child_of = Some(TargetType::Passed); self }
}

// ---------------------------------------------------------------------------
// Generic spawn events
// ---------------------------------------------------------------------------

macro_rules! spawn_event {
    ($Name:ident) => {
        #[derive(EntityEvent, Clone, Debug, Reflect)]
        #[reflect(no_field_bounds)]
        pub struct $Name<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static> {
            #[event_target]
            pub entity: Entity,
            #[reflect(ignore)]
            pub targets: Vec<Target<P>>,
        }

        impl<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static> $Name<P> {
            pub fn new(entity: Entity, targets: Vec<Target<P>>) -> Self {
                Self { entity, targets }
            }
        }

        impl<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static>
            bevy_gearbox::transitions::TransitionEvent for $Name<P>
        where
            $Name<P>: TypePath,
        {
            type ExitEvent = bevy_gearbox::NoEvent;
            type EdgeEvent = bevy_gearbox::NoEvent;
            type EntryEvent = GoOff<P>;
            type Validator = bevy_gearbox::AcceptAll;

            fn to_entry_event(
                &self,
                entering: Entity,
                _exiting: Entity,
                _edge: Entity,
            ) -> Option<GoOff<P>> {
                Some(GoOff::new(entering, self.targets.clone()))
            }
        }

        impl<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static>
            From<Vec<Target<P>>> for $Name<P>
        {
            fn from(targets: Vec<Target<P>>) -> Self {
                Self { entity: Entity::PLACEHOLDER, targets }
            }
        }

        impl<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static>
            crate::gearbox::repeater::Repeatable for $Name<P>
        where
            $Name<P>: bevy_gearbox::transitions::TransitionEvent,
            for<'a> <$Name<P> as Event>::Trigger<'a>: Default,
        {
            fn repeat_tick(entity: Entity) -> Self {
                Self { entity, targets: Vec::new() }
            }
        }
    };
}

spawn_event!(OnSpawnOrigin);
spawn_event!(OnSpawnTarget);
spawn_event!(OnSpawnInvoker);

// ---------------------------------------------------------------------------
// Spawn event observer helpers
// ---------------------------------------------------------------------------

/// Forward OnSpawnOrigin to GoOff.
pub fn on_spawn_origin<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static>(
    ev: On<OnSpawnOrigin<P>>,
    mut commands: Commands,
) {
    commands.trigger(GoOff::new(ev.entity, ev.targets.clone()));
}

/// Forward OnSpawnTarget to GoOff.
pub fn on_spawn_target<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static>(
    ev: On<OnSpawnTarget<P>>,
    mut commands: Commands,
) {
    commands.trigger(GoOff::new(ev.entity, ev.targets.clone()));
}

/// Forward OnSpawnInvoker to GoOff.
pub fn on_spawn_invoker<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + 'static>(
    ev: On<OnSpawnInvoker<P>>,
    mut commands: Commands,
) {
    commands.trigger(GoOff::new(ev.entity, ev.targets.clone()));
}

// ---------------------------------------------------------------------------
// Generic spawn observer
// ---------------------------------------------------------------------------

pub fn spawn_observer<B: SpatialBackend>(
    go_off: On<GoOff<B::Pos>>,
    q_effect: Query<&SpawnConfig<B>>,
    q_invoker: Query<&InvokedBy>,
    q_child_of: Query<&ChildOf>,
    q_invoker_target: Query<&InvokerTarget<B::Pos>>,
    q_global_transform: Query<&GlobalTransform>,
    template_registry: Res<TemplateRegistry>,
    mut ctx: B::Context<'_, '_>,
    mut commands: Commands,
) {
    let effect_entity = go_off.entity;
    let in_targets = go_off.targets.clone();

    let invoker = q_invoker.root_ancestor(effect_entity);
    let invoker_target: Target<B::Pos> = q_invoker_target.get(invoker).copied().map(Target::from).unwrap_or_default();
    let Ok(spawn_config) = q_effect.get(effect_entity) else {
        return;
    };
    let root = q_child_of.root_ancestor(effect_entity);

    let passed_targets: &[Target<B::Pos>] = if in_targets.is_empty() {
        &[Target::default()]
    } else {
        &in_targets
    };

    let mut spawn_targets = Vec::new();
    for passed_target in passed_targets.iter() {
        let mut chunk = generate_targets::<B>(
            &spawn_config.spawn_position_generator,
            &mut ctx,
            invoker,
            invoker_target,
            root,
            B::Pos::default(),
            *passed_target,
        );
        spawn_targets.append(&mut chunk);
    }

    if spawn_targets.is_empty() {
        return;
    }

    let target_targets = if let Some(target_generator) = &spawn_config.spawn_target_generator {
        let targets = generate_targets_with_spawn_positions::<B>(
            target_generator,
            &spawn_targets,
            passed_targets,
            invoker,
            invoker_target,
            root,
            &mut ctx,
        );
        if targets.is_empty() {
            return;
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
            TargetType::Passed => passed_targets.first().and_then(|t| t.entity),
            TargetType::Spawn => None,
            TargetType::Position(_) => None,
        };
        if parent.is_none() {
            return;
        }
        parent
    } else {
        None
    };

    for (i, spawn_target) in spawn_targets.iter().enumerate() {
        let final_transform = B::spawn_transform(spawn_target.position, parent_entity, &q_global_transform);

        let spawned_entity = commands.spawn((final_transform, InvokedBy(invoker))).id();
        template_registry
            .get(&spawn_config.template_id)
            .unwrap()(&mut commands, Some(spawned_entity));

        match (&target_targets, parent_entity) {
            (Some(targets), Some(parent)) => {
                let target_target = targets.get(i % targets.len()).copied().unwrap_or(*spawn_target);
                commands.entity(spawned_entity).insert((target_target, ChildOf(parent)));
                commands.trigger(OnSpawnTarget::new(spawned_entity, vec![target_target]));
            }
            (Some(targets), None) => {
                let target_target = targets.get(i % targets.len()).copied().unwrap_or(*spawn_target);
                commands.entity(spawned_entity).insert(target_target);
                commands.trigger(OnSpawnTarget::new(spawned_entity, vec![target_target]));
            }
            (None, Some(parent)) => {
                commands.entity(spawned_entity).insert(ChildOf(parent));
            }
            (None, None) => {}
        }

        commands.trigger(OnSpawnOrigin::new(
            spawned_entity,
            vec![Target::entity(spawned_entity, spawn_target.position)],
        ));

        let invoker_position = B::position_of(&ctx, invoker).unwrap_or_default();
        commands.trigger(OnSpawnInvoker::new(
            spawned_entity,
            vec![Target::entity(invoker, invoker_position)],
        ));
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
                generator, ctx, invoker, invoker_target, root, spawn_target.position, passed_target,
            );
            all_targets.append(&mut targets);
        }
        all_targets
    } else {
        let mut all_targets = Vec::new();
        for passed_target in in_targets.iter() {
            let mut t = generate_targets::<B>(
                generator, ctx, invoker, invoker_target, root, B::Pos::default(), *passed_target,
            );
            all_targets.append(&mut t);
        }
        all_targets
    }
}
