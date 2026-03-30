pub mod go_off;
pub mod repeater;
pub mod templates;

pub mod prelude {
    pub use crate::go_off;
    pub use crate::gearbox::repeater::{OnComplete, Repeater, repeater_tick, template_repeater};
    pub use crate::gearbox::templates::apply_sub_effect;
}
