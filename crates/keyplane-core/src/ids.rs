//! Stable Element IDs.
//!
//! Per ADR 0028 and the EDN profile contract, keys, layers, sources, and style
//! references carry persistent string identifiers that survive imports, edits,
//! and migrations. They are intentionally opaque strings, not display names.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! stable_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({:?})", stringify!($name), self.0)
            }
        }
    };
}

stable_id!(
    /// Stable identifier for a Physical Key.
    KeyId
);
stable_id!(
    /// Stable identifier for a layer in the Logical Keymap.
    LayerId
);
stable_id!(
    /// Stable identifier for a source (importer or Protocol Backend).
    SourceId
);
stable_id!(
    /// Stable identifier for a Visual Style reference.
    StyleId
);
