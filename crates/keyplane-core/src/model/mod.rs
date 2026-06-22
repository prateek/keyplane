//! The normalized Keyboard Model (ADR 0014).
//!
//! Combines Physical Layout, Logical Keymap, and Visual Style without treating
//! any external source format as canonical. Runtime State is intentionally not
//! part of the static model — it changes live and is owned by the
//! [`Composer`](crate::compose::Composer), which folds the model and runtime
//! into a [`KeyboardSnapshot`](crate::snapshot::KeyboardSnapshot).

pub mod keymap;
pub mod physical;
pub mod runtime;
pub mod style;

pub use keymap::{Layer, LayerEntry, LogicalKeymap};
pub use physical::{PhysicalKey, PhysicalLayout};
pub use runtime::{ActivationKind, ActiveLayer, LayerStack, RuntimeState, StateConfidence};
pub use style::{StyleVariant, VisualStyle};

use crate::ids::LayerId;
use serde::{Deserialize, Serialize};

/// The static, normalized representation of one keyboard.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KeyboardModel {
    /// Human-facing keyboard name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub physical_layout: PhysicalLayout,
    pub keymap: LogicalKeymap,
    pub style: VisualStyle,
}

impl KeyboardModel {
    pub fn new(physical_layout: PhysicalLayout, keymap: LogicalKeymap) -> Self {
        Self {
            name: None,
            physical_layout,
            keymap,
            style: VisualStyle::default(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_style(mut self, style: VisualStyle) -> Self {
        self.style = style;
        self
    }

    /// The base layer id: the keymap default, or the lowest-indexed layer.
    pub fn base_layer(&self) -> Option<LayerId> {
        self.keymap.default_layer.clone().or_else(|| {
            self.keymap
                .layers
                .iter()
                .min_by_key(|l| l.index)
                .map(|l| l.id.clone())
        })
    }
}
