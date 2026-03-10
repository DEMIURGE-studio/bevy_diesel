use std::fmt::Debug;

use bevy::prelude::*;

use crate::effect::GoOff;

/// Prints a message to stdout when the effect entity receives a GoOff event.
#[derive(Component, Clone, Debug)]
pub struct PrintLn(pub String);

impl PrintLn {
    pub fn new(message: &str) -> Self {
        Self(message.to_string())
    }
}

pub fn print_effect<P: Clone + Copy + Send + Sync + Default + Debug + 'static>(
    go_off: On<GoOff<P>>,
    query: Query<&PrintLn>,
) {
    let entity = go_off.entity;
    let Ok(print) = query.get(entity) else {
        return;
    };
    println!("Entity {:?} says: {:?}", entity, print.0);
}
