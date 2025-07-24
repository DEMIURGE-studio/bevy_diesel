use avian3d::prelude::SpatialQuery;
use bevy::prelude::*;
use bevy::ecs::system::{ParamSet, SystemParam};



struct Target {}

// Produces a list of targets from the context
trait CollectorNodeRunner: SystemParam {
    fn collect(&self, origin: Target) -> Vec<Target>;
}

trait FilterNodeRunner: SystemParam {
    fn filter(&self, targets: Vec<Target>) -> Vec<Target>;
}

#[derive(SystemParam)]
struct Avian3dCollectorNodeRunner<'w, 's> {
    spatial_query: SpatialQuery<'w, 's>,
}

impl<'w, 's> CollectorNodeRunner for Avian3dCollectorNodeRunner<'w, 's> {
    fn collect(&self, origin: Target) -> Vec<Target> {
        todo!()
    }
}

#[derive(SystemParam)]
struct SpatialFilterNodeRunner<'w, 's> {
    transforms: ParamSet<'w, 's, (
        Query<'w, 's, &'static Transform>,
        Query<'w, 's, &'static GlobalTransform>,
    )>,
}

enum SpatialFilterNode {
    WithinRadius(f32),
}

impl<'w, 's> FilterNodeRunner for SpatialFilterNodeRunner<'w, 's> {
    fn filter(&self, targets: Vec<Target>) -> Vec<Target> {
        todo!()
    }
}