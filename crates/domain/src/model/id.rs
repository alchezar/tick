//! Newtype wrappers for domain identifiers.

use core::fmt::{Display, Formatter, Result as FmtResult};

use uuid::Uuid;

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(Uuid);

        impl $name {
            /// Generates a new random identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Returns the inner [`Uuid`].
            #[must_use]
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
                Display::fmt(&self.0, f)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }
    };
}

define_id!(
    /// Unique identifier for a [`Task`](super::Task).
    TaskId
);

define_id!(
    /// Unique identifier for a [`Project`](super::Project).
    ProjectId
);

define_id!(
    /// Unique identifier for a [`StatusChange`](super::StatusChange).
    StatusChangeId
);
