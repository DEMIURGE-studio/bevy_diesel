use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::reflect::TypePath;

use bevy_gauge::prelude::{AttributeDerived, AttributesMut};
use bevy_gauge::resolvable::AttributeResolvable;

use crate::backend::SpatialBackend;
use crate::diagnostics::diesel_debug;
use crate::effect::{GoOff, GoOffOrigin};
use crate::invoke::Ability;
use crate::invoker::InvokedBy;
use crate::pipeline::generate_targets;
use crate::target::{InvokerTarget, Target, TargetGenerator, TargetType};

/// Walk the `InvokedBy` chain and return the first ancestor with `Ability`.
fn find_ability(
    entity: Entity,
    q_invoker: &Query<&InvokedBy>,
    q_ability: &Query<(), With<Ability>>,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        if q_ability.get(current).is_ok() {
            return Some(current);
        }
        let Ok(invoked_by) = q_invoker.get(current) else {
            return None;
        };
        current = invoked_by.0;
    }
}

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

    pub fn keys(&self) -> Vec<&str> {
        self.templates.keys().map(|s| s.as_str()).collect()
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
    /// Copy gather scope onto spawned entity as base attributes
    /// (suffix stripped).
    pub inherit_scope: bool,
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
            inherit_scope: false,
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

    /// Inject gather scope as base attributes on the spawned entity.
    pub fn with_scope_inheritance(mut self) -> Self {
        self.inherit_scope = true;
        self
    }
}

// ---------------------------------------------------------------------------
// AttributeResolvable for SpawnConfig
// ---------------------------------------------------------------------------

impl<B: SpatialBackend> AttributeResolvable for SpawnConfig<B>
where
    B::Offset: AttributeResolvable,
    B::Gatherer: AttributeResolvable,
    B::Filter: AttributeResolvable,
{
    fn should_resolve(
        &self,
        prefix: &str,
        attrs: &bevy_gauge::attributes::Attributes,
    ) -> bool {
        self.spawn_position_generator
            .should_resolve(&format!("{prefix}.position"), attrs)
            || self
                .spawn_target_generator
                .should_resolve(&format!("{prefix}.target"), attrs)
    }

    fn resolve(&mut self, prefix: &str, attrs: &bevy_gauge::attributes::Attributes) {
        self.spawn_position_generator
            .resolve(&format!("{prefix}.position"), attrs);
        self.spawn_target_generator
            .resolve(&format!("{prefix}.target"), attrs);
    }
}

// ---------------------------------------------------------------------------
// AttributeDerived for SpawnConfig (bridges to AttributeResolvable)
// ---------------------------------------------------------------------------

impl<B: SpatialBackend> AttributeDerived for SpawnConfig<B>
where
    B::Offset: AttributeResolvable,
    B::Gatherer: AttributeResolvable,
    B::Filter: AttributeResolvable,
{
    fn should_update(
        &self,
        attrs: &bevy_gauge::attributes::Attributes,
    ) -> bool {
        self.should_resolve("SpawnConfig", attrs)
    }

    fn update_from_attributes(
        &mut self,
        attrs: &bevy_gauge::attributes::Attributes,
    ) {
        self.resolve("SpawnConfig", attrs);
    }
}

// ---------------------------------------------------------------------------
// Generic spawn events
// ---------------------------------------------------------------------------

macro_rules! spawn_event {
    ($Name:ident) => {
        #[derive(Message, Clone, Debug, Reflect)]
        pub struct $Name<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + Reflect + 'static> {
            pub entity: Entity,
            pub target: Target<P>,
        }

        impl<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + Reflect + 'static>
            bevy_gearbox::GearboxMessage for $Name<P>
        {
            type Validator = bevy_gearbox::AcceptAll;
            fn target(&self) -> Entity {
                self.entity
            }
        }

        impl<P: Clone + Copy + Send + Sync + Default + Debug + TypePath + Reflect + 'static> $Name<P> {
            pub fn new(entity: Entity, target: Target<P>) -> Self {
                Self { entity, target }
            }
        }

        impl<P: crate::events::PosBound> crate::events::HasDieselTarget<P> for $Name<P> {
            fn diesel_target(&self) -> crate::target::Target<P> { self.target }
        }

        impl<P: crate::events::PosBound> crate::effect::MessageScope for $Name<P> {}
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
    q_ability: Query<(), With<Ability>>,
    template_registry: Res<TemplateRegistry>,
    mut ctx: B::Context<'_, '_>,
    mut commands: Commands,
    mut attributes: AttributesMut,
    mut spawn_target_writer: MessageWriter<OnSpawnTarget<B::Pos>>,
    mut spawn_origin_writer: MessageWriter<OnSpawnOrigin<B::Pos>>,
    mut spawn_invoker_writer: MessageWriter<OnSpawnInvoker<B::Pos>>,
) {
    for go_off in reader.read() {
        let effect_entity = go_off.entity;
        let passed = go_off.target;

        let invoker = q_invoker.root_ancestor(effect_entity);
        let invoker_target: Target<B::Pos> = match q_invoker_target.get(invoker) {
            Ok(it) => Target::from(*it),
            Err(_) => {
                diesel_debug!(
                    "[bevy_diesel] spawn_system: invoker {:?} has no InvokerTarget, defaulting to origin",
                    invoker,
                );
                Target::default()
            }
        };
        let Ok(spawn_config) = q_effect.get(effect_entity) else {
            diesel_debug!("[diesel] spawn_system: GoOff for {:?} — no SpawnConfig, skipping", effect_entity);
            continue;
        };
        let root = q_child_of.root_ancestor(effect_entity);

        diesel_debug!("[diesel] spawn_system: received GoOff for {:?}, template='{}', invoker={:?}",
            effect_entity, spawn_config.template_id, invoker);

        let spawn_targets: Vec<Target<B::Pos>> = generate_targets::<B>(
            &spawn_config.spawn_position_generator,
            &mut ctx,
            invoker,
            invoker_target,
            root,
            B::Pos::default(),
            passed,
        )
        .into_iter()
        .map(|(t, _)| t)
        .collect();

        if spawn_targets.is_empty() {
            diesel_debug!("[diesel]   spawn_targets EMPTY — skipping! invoker={:?} invoker_target={:?}", invoker, invoker_target);
            continue;
        }
        diesel_debug!("[diesel]   spawn_targets count: {}, positions: {:?}", spawn_targets.len(), spawn_targets.iter().map(|t| t.position).collect::<Vec<_>>());

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
                diesel_debug!("[diesel]   target_targets EMPTY — skipping!");
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
                warn!(
                    "[bevy_diesel] spawn_system: as_child_of={:?} resolved to None for \
                     effect {:?} (template='{}', invoker={:?}). Spawn skipped.",
                    parent_target_type, effect_entity, spawn_config.template_id, invoker,
                );
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
            let Some(template_fn) = template_registry.get(&spawn_config.template_id) else {
                panic!(
                    "[bevy_diesel] Template '{}' not found in TemplateRegistry. \
                     Registered: {:?}. Ensure the template is registered before \
                     any SpawnConfig references it.",
                    spawn_config.template_id,
                    template_registry.keys(),
                );
            };
            diesel_debug!("[diesel] spawning entity {:?} from template '{}'", spawned_entity, spawn_config.template_id);
            template_fn(&mut commands, Some(spawned_entity));

            // Register gauge sources for cross-entity attribute expressions.
            // The aliases are stored in the DependencyGraph immediately; when
            // Attributes + modifiers are applied later (during command flush),
            // expressions like "Damage@root" or "Cooldown@ability" will resolve.
            attributes.register_source(spawned_entity, "root", root);
            if let Some(ability) = find_ability(effect_entity, &q_invoker, &q_ability) {
                attributes.register_source(spawned_entity, "ability", ability);
            }

            // Inherit gather scope as base attributes (suffix stripped).
            // Runs after template_fn so scope wins on collision.
            if spawn_config.inherit_scope && !go_off.gather.is_empty() {
                for (key, val) in &go_off.gather {
                    let attr_name = key.split('@').next().unwrap_or(key);
                    attributes.set_base(spawned_entity, attr_name, *val);
                }
            }

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

            let invoker_position = B::position_of(&ctx, invoker).unwrap_or_else(|| {
                diesel_debug!(
                    "[bevy_diesel] spawn_system: invoker {:?} has no position, defaulting to origin",
                    invoker,
                );
                B::Pos::default()
            });
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
            let targets = generate_targets::<B>(
                generator,
                ctx,
                invoker,
                invoker_target,
                root,
                spawn_target.position,
                passed_target,
            );
            all_targets.extend(targets.into_iter().map(|(t, _)| t));
        }
        all_targets
    } else {
        let mut all_targets = Vec::new();
        for passed_target in in_targets.iter() {
            let t = generate_targets::<B>(
                generator,
                ctx,
                invoker,
                invoker_target,
                root,
                B::Pos::default(),
                *passed_target,
            );
            all_targets.extend(t.into_iter().map(|(t, _)| t));
        }
        all_targets
    }
}
