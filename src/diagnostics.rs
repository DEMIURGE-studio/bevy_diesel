/// Conditional debug logging, compiled out without `diesel_diagnostics`.
#[cfg(feature = "diesel_diagnostics")]
macro_rules! diesel_debug {
    ($($arg:tt)*) => { bevy::prelude::debug!($($arg)*) }
}

#[cfg(not(feature = "diesel_diagnostics"))]
macro_rules! diesel_debug {
    ($($arg:tt)*) => {};
}

pub(crate) use diesel_debug;
