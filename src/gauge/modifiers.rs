use std::fmt::Debug;

use bevy::prelude::*;
use bevy_gauge::prelude::{AttributesMut, ModifierSet};
use bevy_gearbox::prelude::Active;

use crate::backend::SpatialBackend;
use crate::diagnostics::diesel_debug;
use crate::effect::GoOff;
use crate::invoker::{InvokedBy, resolve_invoker, resolve_root};
use crate::pipeline::generate_targets;
use crate::target::{InvokerTarget, Target, TargetGenerator};

/// Component wrapping a [`ModifierSet`] that is applied to target entities.
///
/// Used by sustained effects (via [`sustained_modifier_apply`] /
/// [`sustained_modifier_remove`]).
#[derive(Component, Clone, Debug, Default, Deref, DerefMut)]
pub struct AttributeModifiers(pub ModifierSet);

// ---------------------------------------------------------------------------
// Sustained modifiers: apply on Active, remove on Active removed
// ---------------------------------------------------------------------------

/// Config component for sustained modifier effects. Placed on the effect
/// entity alongside [`AttributeModifiers`]. The `generator` resolves the
/// target at activation time; the resolved entity is stored in
/// [`SustainedTarget`] so it can be removed later even if the targeting
/// context has changed.
///
/// Defaults to `TargetType::Invoker` — equipment buffs the player.
#[derive(Component, Clone, Debug)]
pub struct SustainedModifierConfig<B: SpatialBackend> {
    pub generator: TargetGenerator<B>,
}

impl<B: SpatialBackend> Default for SustainedModifierConfig<B> {
    fn default() -> Self {
        Self {
            generator: TargetGenerator::at_invoker(),
        }
    }
}

impl<B: SpatialBackend> SustainedModifierConfig<B> {
    pub fn new(generator: TargetGenerator<B>) -> Self {
        Self { generator }
    }

    /// Target the invoker entity directly (default — equipment buffs).
    pub fn invoker() -> Self {
        Self::default()
    }

    /// Target the invoker's current target.
    pub fn invoker_target() -> Self {
        Self::new(TargetGenerator::at_invoker_target())
    }
}

/// Inserted by [`sustained_modifier_apply`] when modifiers are applied.
/// Stores the resolved target entity so [`sustained_modifier_remove`] can
/// find it on deactivation — the original targeting context may have changed
/// by then.
#[derive(Component, Clone, Copy, Debug)]
pub struct SustainedTarget(pub Entity);

/// When a sustained modifier effect gains `Active`, resolve its target via
/// the config's `TargetGenerator`, apply the modifiers, and store the
/// resolved target in [`SustainedTarget`].
pub fn sustained_modifier_apply<B: SpatialBackend>(
    q_new: Query<
        (Entity, &AttributeModifiers, &SustainedModifierConfig<B>),
        Added<Active>,
    >,
    mut ctx: B::Context<'_, '_>,
    q_invoker: Query<&InvokedBy>,
    q_child_of: Query<&ChildOf>,
    q_invoker_target: Query<&InvokerTarget<B::Pos>>,
    mut attributes: AttributesMut,
    mut commands: Commands,
) {
    for (entity, modifiers, config) in &q_new {
        let invoker = resolve_invoker(&q_invoker, entity);
        let root = resolve_root(&q_child_of, entity);
        let invoker_target: Target<B::Pos> = match q_invoker_target.get(invoker) {
            Ok(it) => Target::from(*it),
            Err(_) => {
                diesel_debug!(
                    "[bevy_diesel] sustained_modifier_apply: invoker {:?} has no InvokerTarget, defaulting to origin",
                    invoker,
                );
                Target::default()
            }
        };

        let targets = generate_targets::<B>(
            &config.generator,
            &mut ctx,
            invoker,
            invoker_target,
            root,
            B::Pos::default(),
            invoker_target,
        );

        // Apply to the first resolved entity target. Sustained modifiers
        // are single-target by nature (you can't "un-apply from N entities"
        // without tracking all of them). If a multi-target sustained effect
        // is needed later, SustainedTarget becomes a Vec.
        let target_count = targets.len();
        if let Some(target) = targets.into_iter().find_map(|t| t.entity) {
            modifiers.apply(target, &mut attributes);
            commands.entity(entity).insert(SustainedTarget(target));
        } else {
            diesel_debug!(
                "[bevy_diesel] sustained_modifier_apply: entity {:?} resolved {} targets but \
                 none had an entity. Sustained modifiers require an entity target.",
                entity, target_count,
            );
        }
    }
}

/// When a sustained modifier effect loses `Active`, look up the stored
/// [`SustainedTarget`] and remove the modifiers.
pub fn sustained_modifier_remove(
    mut removed: RemovedComponents<Active>,
    q_modifiers: Query<&AttributeModifiers>,
    q_sustained_target: Query<&SustainedTarget>,
    mut attributes: AttributesMut,
    mut commands: Commands,
) {
    for entity in removed.read() {
        let Ok(modifiers) = q_modifiers.get(entity) else {
            continue;
        };
        let Ok(sustained_target) = q_sustained_target.get(entity) else {
            warn!(
                "[bevy_diesel] sustained_modifier_remove: entity {:?} lost Active and has \
                 AttributeModifiers but no SustainedTarget. Modifiers may have leaked.",
                entity,
            );
            continue;
        };
        modifiers.remove(sustained_target.0, &mut attributes);
        if let Ok(mut ec) = commands.get_entity(entity) {
            ec.try_remove::<SustainedTarget>();
        }
    }
}
