pub mod go_off;
pub mod repeater;
pub mod templates;

pub mod prelude {
    pub use crate::go_off;
    pub use crate::gearbox::DieselGearboxPlugin;
    pub use crate::gearbox::repeater::{OnComplete, Repeatable, Repeater, template_repeater};
    pub use crate::gearbox::templates::apply_sub_effect;
}

use bevy::prelude::*;

pub struct DieselGearboxPlugin<E: repeater::Repeatable>
where
    for<'a> <E as Event>::Trigger<'a>: Default,
{
    _marker: std::marker::PhantomData<E>,
}

impl<E: repeater::Repeatable> Default for DieselGearboxPlugin<E>
where
    for<'a> <E as Event>::Trigger<'a>: Default,
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<E: repeater::Repeatable> Plugin for DieselGearboxPlugin<E>
where
    for<'a> <E as Event>::Trigger<'a>: Default,
{
    fn build(&self, app: &mut App) {
        app.add_observer(repeater::repeater_observer::<E>)
            .add_observer(repeater::reset_repeater)
            .register_type::<repeater::Repeater>();
    }
}
