use bevy::prelude::*;

use crate::backend::SpatialBackend;
use crate::effect::{GoOff, GoOffOrigin, SubEffects};
use crate::invoker::{InvokedBy, resolve_invoker, resolve_root};
use crate::target::{InvokerTarget, Target, TargetGenerator, TargetMutator, TargetType};

/// Resolve → offset → gather. Returns unfiltered results.
pub fn generate_targets<B: SpatialBackend>(
    generator: &TargetGenerator<B>,
    ctx: &mut B::Context<'_, '_>,
    invoker: Entity,
    invoker_target: Target<B::Pos>,
    root: Entity,
    spawn_pos: B::Pos,
    passed: Target<B::Pos>,
) -> Vec<Target<B::Pos>> {
    // Stage 1: Resolve TargetType to a base Target<P>
    let base_target = match &generator.target_type {
        TargetType::Invoker => {
            let pos = B::position_of(ctx, invoker).unwrap_or_default();
            Target::entity(invoker, pos)
        }
        TargetType::Root => {
            let pos = B::position_of(ctx, root).unwrap_or_default();
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
    match &generator.gatherer {
        None => {
            // Identity: return the resolved+offset target as-is
            vec![offset_target]
        }
        Some(gatherer) => {
            // Fully delegated to backend
            B::gather(ctx, offset_target.position, gatherer, invoker)
        }
    }
}

/// Reads [`GoOffOrigin`] messages, walks the [`SubEffects`] tree for each one,
/// resolves targets at each level, and writes a flat [`GoOff`] for every
/// descendant effect entity. No recursion — single pass using a stack.
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
    let mut origin_count = 0u32;
    for origin in reader.read() {
        origin_count += 1;
        let root_entity = origin.entity;
        let root_targets = origin.targets.clone();

        let invoker = resolve_invoker(&q_invoker, root_entity);
        let root = resolve_root(&q_child_of, root_entity);
        let invoker_target: Target<B::Pos> = q_invoker_target
            .get(invoker)
            .copied()
            .map(Target::from)
            .unwrap_or_default();

        // Fire GoOff on the root entity itself
        writer.write(GoOff::new(root_entity, root_targets.clone()));

        // Walk the tree: (entity, targets_for_this_entity)
        let mut stack: Vec<(Entity, Vec<Target<B::Pos>>)> = vec![(root_entity, root_targets)];

        while let Some((parent, in_targets)) = stack.pop() {
            let Ok(subs) = q_sub_effects.get(parent) else {
                continue;
            };

            for &child in subs.into_iter() {
                let out_targets = if let Ok(Some(mutator)) = q_target_mutator.get(child) {
                    let mut aggregated = Vec::new();
                    for passed in in_targets.iter() {
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
                        aggregated.append(&mut targets);
                    }
                    aggregated
                } else {
                    in_targets.clone()
                };

                writer.write(GoOff::new(child, out_targets.clone()));
                stack.push((child, out_targets));
            }
        }
    }
}

// Keep the old name as an alias for code that references it
pub use propagate_system as propagate_observer;
