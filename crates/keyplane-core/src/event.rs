//! Runtime Events (ADR 0014, ADR 0023, ADR 0036).
//!
//! Incremental updates emitted after a [`KeyboardSnapshot`](crate::snapshot)
//! has loaded. Because Rust owns resolution, a layer-stack change ships the
//! freshly resolved keys so the frontend swaps rendered values without
//! re-resolving anything.

use crate::health::BackendHealth;
use crate::ids::KeyId;
use crate::model::{LayerStack, StateConfidence};
use crate::resolve::ResolvedKey;
use serde::{Deserialize, Serialize};

/// An incremental update to Runtime State.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RuntimeEvent {
    /// The active Layer Stack changed. Carries the new stack, confidence, and
    /// the re-resolved keys so the overlay can render directly.
    LayerStack {
        layer_stack: LayerStack,
        confidence: StateConfidence,
        keys: Vec<ResolvedKey>,
    },
    /// The set of pressed Physical Keys changed.
    PressedKeys { pressed: Vec<KeyId> },
    /// A backend's health changed (permission, disconnect, stale, error).
    BackendHealth { health: BackendHealth },
}
