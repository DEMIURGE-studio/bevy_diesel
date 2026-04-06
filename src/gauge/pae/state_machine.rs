use bevy::prelude::*;
use bevy_gauge::prelude::*;
use bevy_gearbox::prelude::*;

// ---------------------------------------------------------------------------
// Marker + modifier components
// ---------------------------------------------------------------------------

/// Marker component for the PAE state machine root entity.
#[derive(Component, Default, Debug, Clone)]
#[require(
    Attributes,
    AttributeRequirements,
    AppliedModifiers,
    ActivatedModifiers,
    UnappliedState,
)]
pub struct PersistentAttributeEffect;

/// Modifiers applied when the PAE enters `AppliedState`.
/// These remain until the PAE returns to `UnappliedState`.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut)]
pub struct AppliedModifiers(pub ModifierSet);

/// Modifiers applied when the PAE enters `ActiveState`.
/// These are removed when the PAE exits `ActiveState` (e.g., on suspend).
#[derive(Component, Debug, Clone, Default, Deref, DerefMut)]
pub struct ActivatedModifiers(pub ModifierSet);

// ---------------------------------------------------------------------------
// EffectTarget relationship
// ---------------------------------------------------------------------------

/// Which entity this PAE applies modifiers to.
#[derive(Component, Debug, Clone, Copy)]
pub struct EffectTarget(pub Entity);

// ---------------------------------------------------------------------------
// RequiresStatsOf / RequirementsOf relationship
// ---------------------------------------------------------------------------

/// Relationship target: entities that require stats from this entity.
#[derive(Component, Default, Debug, PartialEq, Eq, Reflect)]
#[relationship_target(relationship = RequiresStatsOf, linked_spawn)]
#[reflect(Component, FromWorld, Default)]
pub struct RequirementsOf(Vec<Entity>);

impl<'a> IntoIterator for &'a RequirementsOf {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = std::slice::Iter<'a, Entity>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl RequirementsOf {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

/// Relationship: "this entity requires stats from that entity."
#[derive(Component, Clone, PartialEq, Eq, Debug, Reflect)]
#[relationship(relationship_target = RequirementsOf)]
#[reflect(Component, PartialEq, Debug, FromWorld, Clone)]
pub struct RequiresStatsOf(#[entities] pub Entity);

impl FromWorld for RequiresStatsOf {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        RequiresStatsOf(Entity::PLACEHOLDER)
    }
}

// ---------------------------------------------------------------------------
// State components
// ---------------------------------------------------------------------------

#[derive(Component, Reflect, Clone, Default, Debug)]
pub struct UnappliedState;

#[derive(Component, Reflect, Clone, Default, Debug)]
pub struct AppliedState;

#[derive(Component, Reflect, Clone, Default, Debug)]
pub struct ActiveState;

// ---------------------------------------------------------------------------
// Transition messages
// ---------------------------------------------------------------------------

/// Trigger to apply a PAE: Unapplied → Applied.
#[derive(Message, Reflect, Clone)]
pub struct PAETryApply {
    pub target: Entity,
}

impl GearboxMessage for PAETryApply {
    type Validator = AcceptAll;
    fn target(&self) -> Entity { self.target }
}

/// Trigger to suspend a PAE: Active → Applied.
#[derive(Message, Reflect, Clone)]
pub struct PAESuspend {
    pub target: Entity,
}

impl GearboxMessage for PAESuspend {
    type Validator = AcceptAll;
    fn target(&self) -> Entity { self.target }
}

/// Trigger to unapply a PAE: Applied/Active → Unapplied.
#[derive(Message, Reflect, Clone)]
pub struct PAEUnapplyApproved {
    pub target: Entity,
}

impl GearboxMessage for PAEUnapplyApproved {
    type Validator = AcceptAll;
    fn target(&self) -> Entity { self.target }
}

// ---------------------------------------------------------------------------
// State machine builder
// ---------------------------------------------------------------------------

/// Entity handles returned by [`pae_state`] and [`pae_state_machine`].
///
/// `container` is the entity that owns the PAE components
/// (`PersistentAttributeEffect`, `AppliedModifiers`, `ActivatedModifiers`,
/// `EffectTarget`). For top-level [`pae_state_machine`] this is the same
/// entity as the state-machine chart root. For nested [`pae_state`] it is
/// the entity passed as `pae_container` and is a substate of some host
/// state in a larger chart.
pub struct PaeEntities {
    /// Alias of `container`. Retained for callers that treat the PAE as a
    /// standalone chart root.
    pub root: Entity,
    pub container: Entity,
    pub unapplied: Entity,
    pub applied: Entity,
    pub active: Entity,
}

/// Lay Persistent Attribute Effect infrastructure under an existing state.
///
/// Inserts the PAE container components on `pae_container` and spawns the
/// three PAE substates (Unapplied / Applied / Active) as
/// `SubstateOf(pae_container)`, along with the five standard transition
/// edges. Guards on the Applied → Active edge use
/// `RequiresStatsOf(effect_target)`, and modifiers apply/unapply against
/// `effect_target`'s attributes.
///
/// The caller is responsible for placing `pae_container` in the wider
/// chart (either as the chart root with its own `StateMachine` +
/// `InitialState`, or as a substate of some host state). The returned
/// [`PaeEntities`] exposes the substates so the caller can add their own
/// transitions in or out of the PAE region — for example, a composed
/// caller that wants auto-application when the host state activates can
/// spawn an `AlwaysEdge` from `unapplied` to `applied`.
///
/// Use [`pae_state_machine`] for the common "PAE is a standalone chart
/// rooted at its own entity" case.
pub fn pae_state(
    commands: &mut Commands,
    pae_container: Entity,
    effect_target: Entity,
) -> PaeEntities {
    let mut unapplied_state = Entity::PLACEHOLDER;
    let mut applied_state = Entity::PLACEHOLDER;
    let mut active_state = Entity::PLACEHOLDER;

    commands.entity(pae_container).with_children(|parent| {
        unapplied_state = parent.spawn(Name::new("Unapplied")).id();
        applied_state = parent.spawn(Name::new("Applied")).id();
        active_state = parent.spawn(Name::new("Active")).id();

        let edge_unapplied_to_applied = parent.spawn(Name::new("Edge Unapplied->Applied")).id();
        let edge_applied_to_active = parent.spawn(Name::new("Edge Applied->Active")).id();
        let edge_active_to_applied = parent.spawn(Name::new("Edge Active->Applied")).id();
        let edge_to_unapplied = parent.spawn(Name::new("Edge Applied->Unapplied")).id();
        let edge_active_to_unapplied = parent.spawn(Name::new("Edge Active->Unapplied")).id();

        let commands = parent.commands_mut();

        commands.entity(unapplied_state).insert((
            SubstateOf(pae_container),
            StateComponent(UnappliedState),
        ));

        commands.entity(applied_state).insert((
            SubstateOf(pae_container),
            StateComponent(AppliedState),
        ));

        commands.entity(active_state).insert((
            SubstateOf(pae_container),
            StateComponent(ActiveState),
        ));

        commands.entity(edge_unapplied_to_applied).insert((
            Source(unapplied_state),
            Target(applied_state),
            MessageEdge::<PAETryApply>::default(),
        ));

        commands.entity(edge_applied_to_active).insert((
            Source(applied_state),
            Target(active_state),
            AlwaysEdge,
            Guards::new(),
            RequiresStatsOf(effect_target),
        ));

        commands.entity(edge_active_to_applied).insert((
            Source(active_state),
            Target(applied_state),
            MessageEdge::<PAESuspend>::default(),
        ));

        commands.entity(edge_to_unapplied).insert((
            Source(applied_state),
            Target(unapplied_state),
            MessageEdge::<PAEUnapplyApproved>::default(),
        ));

        commands.entity(edge_active_to_unapplied).insert((
            Source(active_state),
            Target(unapplied_state),
            MessageEdge::<PAEUnapplyApproved>::default(),
        ));

        commands
            .entity(pae_container)
            .insert(PersistentAttributeEffect);
    });

    PaeEntities {
        root: pae_container,
        container: pae_container,
        unapplied: unapplied_state,
        applied: applied_state,
        active: active_state,
    }
}

/// Build a standalone Persistent Attribute Effect state machine.
///
/// Thin wrapper over [`pae_state`] that additionally makes the container
/// a chart root (`StateMachine` + `InitialState(unapplied)`) with a `Name`.
/// The PAE is self-targeted: modifiers apply to the same entity that owns
/// the PAE container.
pub fn pae_state_machine(
    commands: &mut Commands,
    entity: Option<Entity>,
) -> PaeEntities {
    let machine_entity = entity.unwrap_or_else(|| commands.spawn_empty().id());
    let pae = pae_state(commands, machine_entity, machine_entity);
    commands.entity(machine_entity).insert((
        Name::new("PAE Machine"),
        StateMachine::new(),
        InitialState(pae.unapplied),
    ));
    pae
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use bevy_gearbox::prelude::{Source, Target};

    /// Build a minimal app with just enough to flush `Commands` and read
    /// component state back. No plugins — we only care that entity
    /// relationships and components land where we expect after `pae_state`
    /// / `pae_state_machine` run.
    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app
    }

    #[test]
    fn pae_state_machine_builds_self_targeted_chart() {
        let mut app = test_app();
        let world = app.world_mut();

        let pae = world.run_system_once(|mut commands: Commands| {
            pae_state_machine(&mut commands, None)
        }).unwrap();

        // Container has the PAE marker + chart root components.
        assert!(world.entity(pae.container).get::<PersistentAttributeEffect>().is_some());
        assert!(world.entity(pae.container).get::<StateMachine>().is_some());
        assert!(world.entity(pae.container).get::<InitialState>().is_some());
        assert_eq!(pae.root, pae.container, "top-level root == container");

        // Self-targeted: the guard edge's RequiresStatsOf points at the container.
        let edges_with_requires = world
            .query::<(&Source, &RequiresStatsOf)>()
            .iter(world)
            .filter(|(src, _)| src.0 == pae.applied)
            .map(|(_, r)| r.0)
            .collect::<Vec<_>>();
        assert_eq!(
            edges_with_requires,
            vec![pae.container],
            "top-level PAE is self-targeted"
        );

        // Substates are SubstateOf(container).
        for state in [pae.unapplied, pae.applied, pae.active] {
            let parent = world.entity(state).get::<SubstateOf>().unwrap();
            assert_eq!(parent.0, pae.container);
        }
    }

    #[test]
    fn pae_state_nests_under_host_state_with_distinct_effect_target() {
        let mut app = test_app();
        let world = app.world_mut();

        // Simulate a host chart: a "host state" entity plus a separate
        // "effect target" entity (what the PAE's modifiers should apply to).
        let host_state = world.spawn(Name::new("HostState")).id();
        let effect_target = world.spawn(Name::new("EffectTarget")).id();
        let pae_container = world.spawn((Name::new("PaeContainer"), SubstateOf(host_state))).id();

        let pae = world
            .run_system_once(move |mut commands: Commands| {
                pae_state(&mut commands, pae_container, effect_target)
            })
            .unwrap();

        // Container got the PAE marker but NOT StateMachine / InitialState —
        // it's a nested container, not a chart root.
        assert!(world.entity(pae.container).get::<PersistentAttributeEffect>().is_some());
        assert!(
            world.entity(pae.container).get::<StateMachine>().is_none(),
            "nested pae_state must not mark the container as a chart root"
        );
        assert!(
            world.entity(pae.container).get::<InitialState>().is_none(),
            "nested pae_state leaves initial-state wiring to the caller"
        );

        // Container is still a substate of the host state (we set that up
        // before calling pae_state; the builder did not disturb it).
        let container_parent = world.entity(pae.container).get::<SubstateOf>().unwrap();
        assert_eq!(container_parent.0, host_state);

        // PAE substates are substates of the container.
        for state in [pae.unapplied, pae.applied, pae.active] {
            let parent = world.entity(state).get::<SubstateOf>().unwrap();
            assert_eq!(parent.0, pae.container);
        }

        // Guard edge's RequiresStatsOf points at effect_target, NOT at the
        // container — this is the key difference from self-targeted PAE.
        let edges_with_requires = world
            .query::<(&Source, &RequiresStatsOf)>()
            .iter(world)
            .filter(|(src, _)| src.0 == pae.applied)
            .map(|(_, r)| r.0)
            .collect::<Vec<_>>();
        assert_eq!(
            edges_with_requires,
            vec![effect_target],
            "nested PAE's guard edge must target the explicit effect_target"
        );
    }

    #[test]
    fn pae_state_edges_have_expected_source_target_pairs() {
        let mut app = test_app();
        let world = app.world_mut();

        let pae = world
            .run_system_once(|mut commands: Commands| pae_state_machine(&mut commands, None))
            .unwrap();

        // Collect every (Source, Target) pair that belongs to this PAE
        // (filter by knowing source is one of our three substates).
        let edges: Vec<(Entity, Entity)> = world
            .query::<(&Source, &Target)>()
            .iter(world)
            .filter(|(s, _)| s.0 == pae.unapplied || s.0 == pae.applied || s.0 == pae.active)
            .map(|(s, t)| (s.0, t.0))
            .collect();

        // Order-insensitive assertion on the five expected edges.
        let expected: Vec<(Entity, Entity)> = vec![
            (pae.unapplied, pae.applied),  // PAETryApply
            (pae.applied, pae.active),     // AlwaysEdge + Guards
            (pae.active, pae.applied),     // PAESuspend
            (pae.applied, pae.unapplied),  // PAEUnapplyApproved
            (pae.active, pae.unapplied),   // PAEUnapplyApproved
        ];
        for edge in &expected {
            assert!(
                edges.contains(edge),
                "expected edge {edge:?} missing from PAE chart, found: {edges:?}"
            );
        }
        assert_eq!(edges.len(), expected.len(), "unexpected extra edges: {edges:?}");
    }
}
