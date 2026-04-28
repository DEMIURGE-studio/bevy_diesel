use bevy::prelude::*;

use crate::backend::SpatialBackend;
use crate::diagnostics::diesel_debug;
use crate::effect::{GoOff, GoOffOrigin, SubEffects};
use crate::invoker::{InvokedBy, resolve_invoker, resolve_root};
use crate::target::{Scope, InvokerTarget, Target, TargetGenerator, TargetMutator, TargetType};

/// Resolve → offset → gather. Returns unfiltered results.
pub fn generate_targets<B: SpatialBackend>(
    generator: &TargetGenerator<B>,
    ctx: &mut B::Context<'_, '_>,
    invoker: Entity,
    invoker_target: Target<B::Pos>,
    root: Entity,
    spawn_pos: B::Pos,
    passed: Target<B::Pos>,
) -> Vec<(Target<B::Pos>, Scope)> {
    // Stage 1: Resolve TargetType to a base Target<P>
    let base_target = match &generator.target_type {
        TargetType::Invoker => {
            let pos = B::position_of(ctx, invoker).unwrap_or_else(|| {
                diesel_debug!(
                    "[bevy_diesel] generate_targets: invoker {:?} has no position, defaulting to origin",
                    invoker,
                );
                B::Pos::default()
            });
            Target::entity(invoker, pos)
        }
        TargetType::Root => {
            let pos = B::position_of(ctx, root).unwrap_or_else(|| {
                diesel_debug!(
                    "[bevy_diesel] generate_targets: root {:?} has no position, defaulting to origin",
                    root,
                );
                B::Pos::default()
            });
            Target::entity(root, pos)
        }
        TargetType::InvokerTarget => invoker_target,
        TargetType::Spawn => Target::position(spawn_pos),
        TargetType::Passed => passed,
        TargetType::Position(pos) => Target::position(*pos),
    };

    // Stage 2: Apply backend offset
    let offset_pos = B::apply_offset(ctx, base_target.position, &generator.offset);
    let offset_target = Target {
        entity: base_target.entity,
        position: offset_pos,
    };

    // Stage 3: Gather
    let mut results = match &generator.gatherer {
        None => {
            // Identity: return the resolved+offset target with no scope.
            vec![(offset_target, Scope::new())]
        }
        Some(gatherer) => {
            // Fully delegated to backend; backend attaches per-target scope.
            B::gather(ctx, offset_target.position, gatherer, invoker)
        }
    };

    let invoker_pos = B::position_of(ctx, invoker).unwrap_or_default();
    for (target, scope) in &mut results {
        let dist = B::distance(&invoker_pos, &target.position);
        scope.push(("InvokerDistance@scope", dist));
    }
    results
}

/// Reads [`GoOffOrigin`] messages, walks the [`SubEffects`] tree for each one,
/// resolves targets at each level, and writes a [`GoOff`] for every
/// descendant effect entity × target. No recursion — single pass using a stack.
pub fn propagate_system<B: SpatialBackend>(
    mut reader: MessageReader<GoOffOrigin<B::Pos>>,
    mut ctx: B::Context<'_, '_>,
    q_sub_effects: Query<&SubEffects>,
    q_target_mutator: Query<Option<&TargetMutator<B>>>,
    q_invoker: Query<&InvokedBy>,
    q_child_of: Query<&ChildOf>,
    q_invoker_target: Query<&InvokerTarget<B::Pos>>,
    mut writer: MessageWriter<GoOff<B::Pos>>,
) {
    for origin in reader.read() {
        let root_entity = origin.entity;
        let passed_target = origin.target;
        diesel_debug!("[diesel] propagate: received GoOffOrigin for {:?}", root_entity);

        let invoker = resolve_invoker(&q_invoker, root_entity);
        let root = resolve_root(&q_child_of, root_entity);
        let invoker_target: Target<B::Pos> = match q_invoker_target.get(invoker) {
            Ok(it) => Target::from(*it),
            Err(_) => {
                diesel_debug!(
                    "[bevy_diesel] propagate_system: invoker {:?} has no InvokerTarget, defaulting to origin",
                    invoker,
                );
                Target::default()
            }
        };

        let origin_scope = origin.scope.clone();
        let root_targets: Vec<(Target<B::Pos>, Scope)> =
            if let Ok(Some(mutator)) = q_target_mutator.get(root_entity) {
                let mut targets = generate_targets::<B>(
                    &mutator.generator,
                    &mut ctx,
                    invoker,
                    invoker_target,
                    root,
                    B::Pos::default(),
                    passed_target,
                );
                targets = B::apply_filter(
                    &mut ctx,
                    targets,
                    &mutator.generator.filter,
                    invoker,
                    passed_target.position,
                );
                // Merge origin scope; gather keys win on collision.
                merge_scope_into_targets(&mut targets, &origin_scope);
                targets
            } else {
                vec![(passed_target, origin_scope)]
            };

        // Fire GoOff on the root entity itself (one per resolved target).
        for (target, scope) in &root_targets {
            writer.write(GoOff::with_scope(root_entity, *target, scope.clone()));
        }

        // Walk the tree: (entity, (target, scope) pairs for this entity)
        let mut stack: Vec<(Entity, Vec<(Target<B::Pos>, Scope)>)> =
            vec![(root_entity, root_targets)];

        while let Some((parent, in_targets)) = stack.pop() {
            let Ok(subs) = q_sub_effects.get(parent) else {
                diesel_debug!("[diesel]   {:?} has no SubEffects — leaf node", parent);
                continue;
            };

            diesel_debug!("[diesel]   {:?} has {} sub-effects", parent, subs.into_iter().count());
            for &child in subs.into_iter() {
                let out_targets: Vec<(Target<B::Pos>, Scope)> =
                    if let Ok(Some(mutator)) = q_target_mutator.get(child) {
                        let mut aggregated = Vec::new();
                        for (passed, parent_scope) in in_targets.iter() {
                            let mut targets = generate_targets::<B>(
                                &mutator.generator,
                                &mut ctx,
                                invoker,
                                invoker_target,
                                root,
                                B::Pos::default(),
                                *passed,
                            );
                            let origin_pos = passed.position;
                            targets = B::apply_filter(
                                &mut ctx,
                                targets,
                                &mutator.generator.filter,
                                invoker,
                                origin_pos,
                            );
                            // Inherit parent scope; gather keys win.
                            merge_scope_into_targets(&mut targets, parent_scope);
                            aggregated.append(&mut targets);
                        }
                        aggregated
                    } else {
                        in_targets.clone()
                    };

                // Write one GoOff per (target, scope) pair.
                diesel_debug!("[diesel]   -> writing GoOff for child {:?}, {} targets", child, out_targets.len());
                for (target, scope) in &out_targets {
                    writer.write(GoOff::with_scope(child, *target, scope.clone()));
                }
                stack.push((child, out_targets));
            }
        }
    }
}

// Keep the old name as an alias for code that references it
pub use propagate_system as propagate_observer;

/// Add `parent_scope` keys to each target's scope; existing keys win.
fn merge_scope_into_targets<P: Clone + Copy + Send + Sync + Default + std::fmt::Debug + 'static>(
    targets: &mut Vec<(Target<P>, Scope)>,
    parent_scope: &Scope,
) {
    if parent_scope.is_empty() {
        return;
    }
    for (_, scope) in targets.iter_mut() {
        for (k, v) in parent_scope {
            if !scope.iter().any(|(pk, _)| *pk == *k) {
                scope.push((*k, *v));
            }
        }
    }
}
