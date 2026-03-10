/// Creates an [`EntityEvent`] struct that bridges bevy_gearbox state machine
/// transitions with diesel's `GoOff<P>` effect propagation.
///
/// # Usage
///
/// ```ignore
/// bevy_diesel::go_off!(StartInvoke, Vec3);
/// bevy_diesel::go_off!(OnRepeat, Vec3);
/// ```
#[macro_export]
macro_rules! go_off {
    ($Event:ident, $Pos:ty) => {
        #[derive(
            ::bevy::prelude::EntityEvent,
            Clone,
            Debug,
            ::bevy::prelude::Reflect,
        )]
        #[bevy_gearbox::transition_event]
        pub struct $Event {
            #[event_target]
            pub entity: ::bevy::prelude::Entity,
            #[reflect(ignore)]
            pub targets: ::std::vec::Vec<$crate::target::Target<$Pos>>,
        }

        $crate::go_off!(@impl $Event, $Pos);
    };

    (@impl $Event:ty, $Pos:ty) => {
        impl $Event {
            pub fn new(
                entity: ::bevy::prelude::Entity,
                targets: ::std::vec::Vec<$crate::target::Target<$Pos>>,
            ) -> Self {
                Self { entity, targets }
            }
        }

        impl ::bevy_gearbox::transitions::TransitionEvent for $Event {
            type ExitEvent = ::bevy_gearbox::NoEvent;
            type EdgeEvent = ::bevy_gearbox::NoEvent;
            type EntryEvent = $crate::effect::GoOff<$Pos>;
            type Validator = ::bevy_gearbox::AcceptAll;

            fn to_entry_event(
                &self,
                entering: ::bevy::prelude::Entity,
                _exiting: ::bevy::prelude::Entity,
                _edge: ::bevy::prelude::Entity,
            ) -> Option<$crate::effect::GoOff<$Pos>> {
                Some($crate::effect::GoOff::new(
                    entering,
                    self.targets.clone(),
                ))
            }
        }

        impl From<::std::vec::Vec<$crate::target::Target<$Pos>>>
            for $Event
        {
            fn from(
                value: ::std::vec::Vec<$crate::target::Target<$Pos>>,
            ) -> Self {
                Self {
                    entity: ::bevy::prelude::Entity::PLACEHOLDER,
                    targets: value,
                }
            }
        }

        impl $crate::gearbox::repeater::Repeatable for $Event {
            fn repeat_tick(entity: ::bevy::prelude::Entity) -> Self {
                Self {
                    entity,
                    targets: ::std::vec::Vec::new(),
                }
            }
        }
    };
}
