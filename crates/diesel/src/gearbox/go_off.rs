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
            pub target: $crate::target::Target<$Pos>,
        }

        $crate::go_off!(@impl $Event, $Pos);
    };

    (@impl $Event:ty, $Pos:ty) => {
        impl $Event {
            pub fn new(
                entity: ::bevy::prelude::Entity,
                target: $crate::target::Target<$Pos>,
            ) -> Self {
                Self { entity, target }
            }
        }

        impl ::bevy_gearbox::GearboxMessage for $Event {
            type Validator = ::bevy_gearbox::AcceptAll;
            fn machine(&self) -> ::bevy::prelude::Entity { self.entity }
        }

        impl ::bevy_gearbox::SideEffect<$Event> for $crate::effect::GoOffOrigin<$Pos> {
            fn produce(
                matched: &::bevy_gearbox::Matched<$Event>,
            ) -> Option<Self> {
                Some($crate::effect::GoOffOrigin::new(
                    matched.target,
                    matched.message.target,
                ))
            }
        }
    };
}
