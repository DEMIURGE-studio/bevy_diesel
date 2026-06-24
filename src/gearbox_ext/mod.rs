pub mod go_off;
pub mod repeater;
pub mod templates;

pub mod prelude {
    pub use crate::go_off;
    pub use crate::gearbox_ext::repeater::{Repeater, repeater_tick};
    #[allow(deprecated)]
    pub use crate::gearbox_ext::templates::apply_sub_effect;
}
