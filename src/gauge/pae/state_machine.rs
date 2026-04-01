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

/// Entity handles returned by [`pae_state_machine`].
pub struct PaeEntities {
    pub root: Entity,
    pub unapplied: Entity,
    pub applied: Entity,
    pub active: Entity,
}

/// Build a Persistent Attribute Effect state machine.
///
/// Creates a 3-state machine (Unapplied → Applied → Active) with modifier
/// application on state transitions and stat-gated activation.
pub fn pae_state_machine(
    commands: &mut Commands,
    entity: Option<Entity>,
) -> PaeEntities {
    let machine_entity = entity.unwrap_or_else(|| commands.spawn_empty().id());

    let mut unapplied_state = Entity::PLACEHOLDER;
    let mut applied_state = Entity::PLACEHOLDER;
    let mut active_state = Entity::PLACEHOLDER;

    commands.entity(machine_entity).with_children(|parent| {
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
            SubstateOf(machine_entity),
            StateComponent(UnappliedState),
        ));

        commands.entity(applied_state).insert((
            SubstateOf(machine_entity),
            StateComponent(AppliedState),
        ));

        commands.entity(active_state).insert((
            SubstateOf(machine_entity),
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
            RequiresStatsOf(machine_entity),
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

        commands.entity(machine_entity).insert((
            Name::new("PAE Machine"),
            PersistentAttributeEffect,
            StateMachine::new(),
            InitialState(unapplied_state),
        ));
    });

    PaeEntities {
        root: machine_entity,
        unapplied: unapplied_state,
        applied: applied_state,
        active: active_state,
    }
}
