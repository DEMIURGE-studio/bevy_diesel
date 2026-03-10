use bevy::prelude::*;
use bevy_diesel::pipeline::generate_targets;
use bevy_diesel::spawn::TemplateRegistry;

use crate::{AvianBackend, AvianContext};
use crate::prelude::{
    GoOff, InvokedBy, SpawnConfig, Target, TargetType, TargetGenerator,
};

bevy_diesel::go_off!(OnSpawnOrigin, Vec3);
bevy_diesel::go_off!(OnSpawnTarget, Vec3);
bevy_diesel::go_off!(OnSpawnInvoker, Vec3);

pub fn plugin(app: &mut App) {
    app.add_observer(spawn_entities_observer);
}

/// Forward OnSpawnOrigin to GoOff. Attach with `.observe(on_spawn_origin)` on
/// an entity whose state machine should react to its own spawn position.
pub fn on_spawn_origin(ev: On<OnSpawnOrigin>, mut commands: Commands) {
    commands.trigger(GoOff::new(ev.entity, ev.targets.clone()));
}

/// Forward OnSpawnTarget to GoOff.
pub fn on_spawn_target(ev: On<OnSpawnTarget>, mut commands: Commands) {
    commands.trigger(GoOff::new(ev.entity, ev.targets.clone()));
}

/// Forward OnSpawnInvoker to GoOff.
pub fn on_spawn_invoker(ev: On<OnSpawnInvoker>, mut commands: Commands) {
    commands.trigger(GoOff::new(ev.entity, ev.targets.clone()));
}

pub fn spawn_entities_observer(
    go_off: On<GoOff>,
    q_effect: Query<&SpawnConfig>,
    q_invoker: Query<&InvokedBy>,
    q_child_of: Query<&ChildOf>,
    q_target: Query<&Target>,
    q_global_transform: Query<&GlobalTransform>,
    template_registry: Res<TemplateRegistry>,
    mut ctx: AvianContext,
    mut commands: Commands,
) {
    let effect_entity = go_off.entity;
    let in_targets = go_off.targets.clone();
    if in_targets.is_empty() {
        return;
    }

    let invoker = q_invoker.root_ancestor(effect_entity);
    let Ok(target) = q_target.get(invoker) else {
        return;
    };
    let Ok(spawn_config) = q_effect.get(effect_entity) else {
        return;
    };
    let root = q_child_of.root_ancestor(effect_entity);

    // Generate spawn positions (MapEach over input targets)
    let mut spawn_targets = Vec::new();
    for passed_target in in_targets.iter() {
        let mut chunk = generate_targets::<AvianBackend>(
            &spawn_config.spawn_position_generator,
            &mut ctx,
            invoker,
            *target,
            root,
            Vec3::ZERO,
            *passed_target,
        );
        spawn_targets.append(&mut chunk);
    }

    if spawn_targets.is_empty() {
        return;
    }

    // Generate target positions if a spawn_target_generator is provided
    let target_targets = if let Some(target_generator) = &spawn_config.spawn_target_generator {
        let targets = generate_targets_with_spawn_positions(
            target_generator,
            &spawn_targets,
            &in_targets,
            invoker,
            *target,
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

    // Resolve parent entity if parenting is specified
    let parent_entity = if let Some(parent_target_type) = &spawn_config.as_child_of {
        let parent = match parent_target_type {
            TargetType::Invoker => Some(invoker),
            TargetType::InvokerTarget => target.entity,
            TargetType::Root => Some(root),
            TargetType::Passed => in_targets.first().and_then(|t| t.entity),
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

    // Spawn entities
    for (i, spawn_target) in spawn_targets.iter().enumerate() {
        let final_transform =
            calculate_spawn_transform(spawn_target.position, parent_entity, &q_global_transform);

        let spawned_entity = commands.spawn((final_transform, InvokedBy(invoker))).id();
        template_registry
            .get(&spawn_config.template_id)
            .unwrap()(&mut commands, Some(spawned_entity));

        match (&target_targets, parent_entity) {
            (Some(targets), Some(parent)) => {
                let target_target = targets.get(i % targets.len()).copied().unwrap_or(*spawn_target);
                commands
                    .entity(spawned_entity)
                    .insert((target_target, ChildOf(parent)));
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

        let invoker_position = ctx
            .transforms
            .get(invoker)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);
        commands.trigger(OnSpawnInvoker::new(
            spawned_entity,
            vec![Target::entity(invoker, invoker_position)],
        ));
    }
}

fn generate_targets_with_spawn_positions(
    generator: &TargetGenerator,
    spawn_targets: &[Target],
    in_targets: &[Target],
    invoker: Entity,
    target: Target,
    root: Entity,
    ctx: &mut AvianContext,
) -> Vec<Target> {
    if matches!(generator.target_type, TargetType::Spawn) {
        let mut all_targets = Vec::new();
        for (idx, spawn_target) in spawn_targets.iter().enumerate() {
            let passed_target = in_targets
                .get(idx % in_targets.len())
                .copied()
                .unwrap_or(*spawn_target);
            let mut targets = generate_targets::<AvianBackend>(
                generator,
                ctx,
                invoker,
                target,
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
            let mut t = generate_targets::<AvianBackend>(
                generator,
                ctx,
                invoker,
                target,
                root,
                Vec3::ZERO,
                *passed_target,
            );
            all_targets.append(&mut t);
        }
        all_targets
    }
}

fn calculate_spawn_transform(
    world_position: Vec3,
    parent_entity: Option<Entity>,
    q_global_transform: &Query<&GlobalTransform>,
) -> Transform {
    if let Some(parent) = parent_entity {
        if let Ok(parent_gt) = q_global_transform.get(parent) {
            let local_position = parent_gt.affine().inverse().transform_point3(world_position);
            Transform::from_translation(local_position)
        } else {
            Transform::from_translation(world_position)
        }
    } else {
        Transform::from_translation(world_position)
    }
}
