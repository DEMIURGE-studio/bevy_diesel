use bevy::prelude::*;

#[derive(Component)]
pub struct Invoker(pub Entity);

impl Invoker {
    pub fn get_invoker(
        entity: Entity, 
        child_of_query: &Query<&ChildOf>,
        invoker_query: &Query<&Invoker>,
    ) -> Option<Entity> {
        for ancestor in child_of_query.iter_ancestors(entity) {
            if let Some(invoker) = invoker_query.get(ancestor).ok() {
                return Some(invoker.0);
            }
        }
        None
    }
}