use bevy::prelude::*;
use bevy_gearbox::{prelude::*, GearboxPlugin};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(GearboxPlugin);
    app
}

#[derive(Component, Default)]
struct SourceComp { f: f32, i: i32, b: bool }

// Marker types for params
struct PF; struct PI; struct PB;

impl FloatParamBinding<SourceComp> for PF { fn extract(s: &SourceComp) -> f32 { s.f } }
impl IntParamBinding<SourceComp> for PI { fn extract(s: &SourceComp) -> i32 { s.i } }
impl BoolParamBinding<SourceComp> for PB { fn extract(s: &SourceComp) -> bool { s.b } }

#[test]
fn sync_params_update_components() {
    let mut app = test_app();

    let e = app.world_mut().spawn((SourceComp::default(), FloatParam::<PF>::default(), IntParam::<PI>::default(), BoolParam::<PB>::default())).id();

    // Set source values
    if let Ok(mut ent) = app.world_mut().get_entity_mut(e) {
        if let Some(mut s) = ent.get_mut::<SourceComp>() { s.f = 3.5; s.i = 7; s.b = true; }
    }

    app.add_systems(Update, (
        sync_float_param::<SourceComp, PF>,
        sync_int_param::<SourceComp, PI>,
        sync_bool_param::<SourceComp, PB>,
    ));
    app.update();

    let f = app.world().get::<FloatParam<PF>>(e).unwrap().get();
    let i = app.world().get::<IntParam<PI>>(e).unwrap().get();
    let b = app.world().get::<BoolParam<PB>>(e).unwrap().get();
    assert_eq!(f, 3.5);
    assert_eq!(i, 7);
    assert_eq!(b, true);
}

#[test]
fn apply_param_guards_manage_guard_presence() {
    let mut app = test_app();

    // Build minimal machine root with a source state and an edge
    let root = app.world_mut().spawn_empty().id();
    let s = app.world_mut().spawn_empty().id();
    let t = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(s).insert(SubstateOf(root));
    app.world_mut().entity_mut(t).insert(SubstateOf(root));

    // Edge guarded by float/int/bool conditions
    let edge = app.world_mut().spawn((
        Source(s),
        Target(t),
        FloatInRange::<PF>::new(1.0, 2.0, 0.0),
        IntInRange::<PI>::new(5, 10, 0),
        BoolEquals::<PB>::new(true),
    )).id();

    // Root has the params
    let root_ent = app.world_mut().entity_mut(root).id();
    app.world_mut().entity_mut(root_ent).insert((
        FloatParam::<PF>::default(),
        IntParam::<PI>::default(),
        BoolParam::<PB>::default(),
    ));

    // Systems to apply guards
    app.add_systems(Update, (
        apply_float_param_guards::<PF>,
        apply_int_param_guards::<PI>,
        apply_bool_param_guards::<PB>,
    ));

    // 1) With default params (0,0,false) all should be blocked -> guards present
    app.update();
    {
        let guards = app.world().get::<Guards>(edge).unwrap();
        assert!(!guards.check(), "guards should block when values out of range or not matching");
    }

    // 2) Set to passing values -> guards removed
    if let Ok(mut ent) = app.world_mut().get_entity_mut(root) {
        if let Some(mut f) = ent.get_mut::<FloatParam<PF>>() { f.set(1.5); }
        if let Some(mut i) = ent.get_mut::<IntParam<PI>>() { i.set(7); }
        if let Some(mut b) = ent.get_mut::<BoolParam<PB>>() { b.set(true); }
    }
    app.update();
    {
        let guards = app.world().get::<Guards>(edge).unwrap();
        assert!(guards.check(), "guards should be cleared when conditions pass");
    }
}


