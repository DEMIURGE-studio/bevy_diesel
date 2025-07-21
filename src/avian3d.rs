/// Spatial backend for avian3d
use ::avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use crate::invoker::*;
use crate::random::*;
use crate::*;
use crate::backend::*;

pub enum TargetType {
    Position,
    Entity,
}

pub enum Offset {
    None,
    Fixed(Vec3),
}

pub enum Shape {
    None,
    Sphere(f32),
}

pub enum Number {
    Fixed(usize),
    RandomRange(usize, usize),
}

pub struct Avian3DTargetGeneratorConfig {
    target_type: TargetType,
    initial: RelativeTo,
    offset: Offset,
    shape: Shape,
    number: Number,
}

/// Targets in 3d space will always have a position but may not have an entity.
#[derive(Clone, Component)]
pub struct Avian3DTarget {
    pub entity: Option<Entity>,
    pub position: Vec3,
}

impl Avian3DTarget {
    fn from_entity(entity: Entity, position: Vec3) -> Self {
        todo!()
    }

    fn from_position(position: Vec3) -> Self {
        todo!()
    }
}

#[derive(SystemParam)]
pub struct Avian3DSpatialBackend<'w, 's> {
    transform_query: Query<'w, 's, &'static Transform>,
    invoker_query: Query<'w, 's, &'static Invoker>,
    target_query: Query<'w, 's, &'static Avian3DTarget>,
    child_of_query: Query<'w, 's, &'static ChildOf>,
    rng_query: Query<'w, 's, &'static mut Rng>,
    spatial_query: SpatialQuery<'w, 's>,
}

impl<'w, 's> SpatialBackend for Avian3DSpatialBackend<'w, 's> {
    type Target = Avian3DTarget;

    type Config = Avian3DTargetGeneratorConfig;

    fn generate_targets(
        &mut self,
        config: &Self::Config,
        origin: &Self::Target,
        cue_target: &Self::Target,
        effect_entity: Entity,
    ) -> Vec<Self::Target> {
        let root_entity = self.child_of_query.root_ancestor(effect_entity);
        let invoker_entity = Invoker::get_invoker(effect_entity, &self.child_of_query, &self.invoker_query).unwrap(); // TODO Handle no invoker;
        let invoker_target = self.target_query.get(invoker_entity).unwrap(); // TODO Handle no invoker
        let mut rng = self.rng_query.get_mut(root_entity).unwrap(); // TODO Handle no rng
        
        let initial_target = match config.initial {
            RelativeTo::Invoker => {
                let invoker_position = self.transform_query.get(invoker_entity).unwrap().translation;
                Avian3DTarget::from_entity(invoker_entity, invoker_position)
            },
            RelativeTo::Root => {
                let root_position = self.transform_query.get(root_entity).unwrap().translation;
                Avian3DTarget::from_entity(root_entity, root_position)
            },
            RelativeTo::InvokerTarget => invoker_target.clone(),
            RelativeTo::CueTarget => cue_target.clone(),
            RelativeTo::Origin => origin.clone(),
        };

        let offset_target = match config.offset {
            Offset::None => initial_target,
            Offset::Fixed(vec3) => {
                let position = initial_target.position + vec3;
                Avian3DTarget::from_position(position)
            },
        };

        let number = match config.number {
            Number::Fixed(n) => n,
            Number::RandomRange(min, max) => {
                min // TODO
            }
        };

        let targets = match config.target_type {
            TargetType::Position => {
                match config.shape {
                    Shape::None => vec![offset_target],
                    Shape::Sphere(radius) => {
                        let mut targets = Vec::new();
                        for _ in 0..number {
                            let random_offset = rng.random_vec3_around_zero(radius);
                            let position = offset_target.position + random_offset;
                            targets.push(Avian3DTarget::from_position(position));
                        }
                        targets
                    },
                }
            },
            TargetType::Entity => {
                match config.shape {
                    Shape::None => vec![],
                    Shape::Sphere(radius) => {
                        self.spatial_query.shape_hits(
                            &Collider::sphere(radius), 
                            offset_target.position,
                            Quat::IDENTITY, 
                            Dir3::Y, 
                            number as u32, 
                            &ShapeCastConfig::default(), 
                            &SpatialQueryFilter::default(),
                        ).iter().map(|hit| {
                            let position = hit.point1;
                            Avian3DTarget::from_position(position)
                        }).collect()
                    }
                }
            },
        };

        targets
    }
}