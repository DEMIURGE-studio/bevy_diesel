use bevy::prelude::*;

use crate::backend::SpatialBackend;
use crate::effect::{GoOff, SubEffects};
use crate::invoker::{InvokedBy, resolve_invoker, resolve_root};
use crate::target::{InvokerTarget, Target, TargetGenerator, TargetMutator, TargetType};

/// Core pipeline function: resolve → offset → gather.
///
/// Returns **unfiltered** results. Filtering is applied separately via
/// `B::apply_filter` (called automatically by `propagate_observer`).
///
/// # Arguments
///
/// * `generator` — The targeting pipeline configuration.
/// * `ctx` — The backend's runtime context (spatial queries, transforms, RNG, etc.).
/// * `invoker` — The entity that invoked the ability.
/// * `invoker_target` — The invoker's current target (pre-resolved).
/// * `root` — The root entity of the ability hierarchy.
/// * `spawn_pos` — The spawn position context (for `TargetType::Spawn`).
/// * `passed` — The incoming target from the parent effect (for `TargetType::Passed`).
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

/// Generic GoOff propagation observer. Handles context resolution, MapEach
/// propagation, and child triggering — backends provide spatial logic via
/// their `SpatialBackend` trait implementation.
///
/// Register with: `app.add_observer(propagate_observer::<MyBackend>);`
pub fn propagate_observer<B: SpatialBackend>(
    go_off: On<GoOff<B::Pos>>,
    mut ctx: B::Context<'_, '_>,
    q_sub_effects: Query<&SubEffects>,
    q_target_mutator: Query<Option<&TargetMutator<B>>>,
    q_invoker: Query<&InvokedBy>,
    q_child_of: Query<&ChildOf>,
    q_invoker_target: Query<&InvokerTarget<B::Pos>>,
    mut commands: Commands,
) {
    let parent = go_off.entity;
    let in_targets = go_off.targets.clone();

    let Ok(subs) = q_sub_effects.get(parent) else {
        return;
    };

    // Resolve invoker and root context
    let invoker = resolve_invoker(&q_invoker, parent);
    let root = resolve_root(&q_child_of, parent);

    let invoker_target: Target<B::Pos> = q_invoker_target.get(invoker).copied().map(Target::from).unwrap_or_default();

    // For each sub-effect child, apply MapEach propagation
    for &child in subs.into_iter() {
        let out_targets = if let Ok(Some(mutator)) = q_target_mutator.get(child) {
            let mut aggregated = Vec::new();

            for passed in in_targets.iter() {
                // Core: resolve + offset + gather
                let mut targets = generate_targets::<B>(
                    &mutator.generator,
                    &mut ctx,
                    invoker,
                    invoker_target,
                    root,
                    B::Pos::default(),
                    *passed,
                );

                // Backend: post-gather filtering
                let origin = passed.position;
                targets =
                    B::apply_filter(&mut ctx, targets, &mutator.generator.filter, invoker, origin);

                aggregated.append(&mut targets);
            }
            aggregated
        } else {
            in_targets.clone() // pass-through
        };

        commands.trigger(GoOff::new(child, out_targets));
    }
}
