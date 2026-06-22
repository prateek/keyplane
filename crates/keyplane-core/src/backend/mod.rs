//! Protocol Backends (ADR 0015, ADR 0033).
//!
//! A backend discovers, imports, or streams keyboard information from a
//! concrete source. Backends declare typed [`Capability
//! Flags`](crate::health::CapabilitySet) and report typed
//! [`HealthState`](crate::health::HealthState). They emit [`BackendUpdate`]s;
//! the [`Composer`](crate::compose::Composer) folds those into
//! [`RuntimeEvent`](crate::event::RuntimeEvent)s. Time and I/O live above this
//! crate so core resolution stays deterministic.

pub mod fake;

pub use fake::FakeBackend;

use crate::health::{CapabilitySet, HealthState};
use crate::ids::KeyId;
use crate::model::{LayerStack, StateConfidence};
use crate::provenance::SourceKind;
use serde::{Deserialize, Serialize};

/// Identity and declared abilities of a backend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendDescriptor {
    pub id: String,
    pub name: String,
    pub kind: SourceKind,
    pub capabilities: CapabilitySet,
}

/// An incremental change a backend reports about Runtime State or its health.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendUpdate {
    /// The authoritative/inferred active Layer Stack changed.
    LayerStack {
        stack: LayerStack,
        confidence: StateConfidence,
    },
    /// The set of pressed physical keys changed.
    PressedKeys(Vec<KeyId>),
    /// The backend's health changed.
    Health(HealthState),
}

/// A streaming Protocol Backend.
///
/// `poll` is pull-based and side-effect free with respect to the rest of the
/// app: the driver decides cadence. Returning an empty vec means "no change".
pub trait ProtocolBackend {
    fn descriptor(&self) -> BackendDescriptor;
    fn health(&self) -> HealthState;
    /// Drain any pending updates. The driver calls this on its own schedule.
    fn poll(&mut self) -> Vec<BackendUpdate>;
}
