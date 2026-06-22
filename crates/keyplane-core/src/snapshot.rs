//! The Keyboard Snapshot DTO (ADR 0014, ADR 0036).
//!
//! A point-in-time, fully-resolved view of the Keyboard Model the frontend can
//! render directly. Rust resolves every key here so the web UI never touches
//! protocol, HID, or keymap-resolution logic. Subsequent
//! [`RuntimeEvent`](crate::event::RuntimeEvent)s update it.

use crate::health::BackendHealth;
use crate::ids::LayerId;
use crate::model::style::VisualStyle;
use crate::model::{LayerStack, StateConfidence};
use crate::resolve::ResolvedKey;
use serde::{Deserialize, Serialize};

/// The DTO schema version for the snapshot/event contract consumed by the
/// frontend. Distinct from the EDN Profile Schema Version.
pub const SNAPSHOT_SCHEMA: u32 = 1;

/// Lightweight per-layer metadata for layer hints and the layer picker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerInfo {
    pub id: LayerId,
    pub index: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A renderable, point-in-time view of the keyboard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyboardSnapshot {
    pub schema: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyboard_name: Option<String>,
    /// Bounding box `(width, height)` in keycap units for overlay sizing.
    pub extent: (f64, f64),
    pub style: VisualStyle,
    pub layers: Vec<LayerInfo>,
    pub layer_stack: LayerStack,
    pub confidence: StateConfidence,
    /// Every Physical Key resolved under `layer_stack`.
    pub keys: Vec<ResolvedKey>,
    /// Health for every configured backend (ADR 0023) — always visible.
    pub backends: Vec<BackendHealth>,
}

impl KeyboardSnapshot {
    /// The id of the topmost active layer, which the overlay may highlight.
    pub fn top_layer(&self) -> Option<&LayerId> {
        self.layer_stack.top().map(|a| &a.layer)
    }
}
