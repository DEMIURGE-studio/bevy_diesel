/// Creates a [`Message`] struct that serves as a gearbox state machine
/// transition message in diesel's effect pipeline.
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
            ::bevy::prelude::Message,
            Clone,
            Debug,
        )]
        pub struct $Event {
            pub entity: ::bevy::prelude::Entity,
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

        impl ::bevy_gearbox::GearboxMessage for $Event {
            type Validator = ::bevy_gearbox::AcceptAll;
            fn machine(&self) -> ::bevy::prelude::Entity { self.entity }
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

        impl ::bevy_gearbox::SideEffect<$Event> for $crate::effect::GoOffOrigin<$Pos> {
            fn produce(
                matched: &::bevy_gearbox::Matched<$Event>,
            ) -> Option<Self> {
                Some($crate::effect::GoOffOrigin::new(
                    matched.target,
                    matched.message.targets.clone(),
                ))
            }
        }
    };
}
