use bevy::prelude::*;

use crate::effect::SubEffectOf;
use crate::target::TargetMutator;
use crate::backend::SpatialBackend;

/// Convenience builder that wraps a component in a sub-effect node with a
/// `TargetMutator::invoker()` for target resolution.
pub fn apply_sub_effect<B: SpatialBackend>(
    effect: impl Component,
) -> impl FnOnce(&mut EntityCommands) {
    move |apply: &mut EntityCommands| {
        apply.insert(TargetMutator::<B>::invoker());
        let apply_entity = apply.id();
        apply.with_children(|parent| {
            parent.spawn((
                Name::new("SubEffect"),
                SubEffectOf(apply_entity),
                effect,
            ));
        });
    }
}
