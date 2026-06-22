//! Typed Capability Flags and Backend Health (ADR 0033, ADR 0023).
//!
//! Backends declare what they can do through a [`CapabilitySet`] and report
//! their availability as a typed [`HealthState`] rather than ad hoc strings.
//! Permissions, disconnects, stale data, and protocol errors are Runtime State
//! (ADR 0023), so they travel in snapshots and events and stay visible in the
//! overlay and Source Inspector.

use serde::{Deserialize, Serialize};

/// A single declared ability of a Protocol Backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Can enumerate connected devices.
    DiscoverDevices,
    /// Can import Physical Layout geometry.
    ImportGeometry,
    /// Can import a Logical Keymap.
    ImportKeymap,
    /// Can stream authoritative live Layer Stack changes.
    StreamLayerStack,
    /// Can stream pressed physical keys.
    StreamPressedKeys,
    /// Can poll state on an interval (Best-Effort).
    Poll,
    /// Can only render a Best-Effort Preview, with no authoritative runtime.
    PreviewOnly,
}

/// The set of capabilities a backend declares.
///
/// Stored sorted and deduplicated so serialized output is deterministic.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilitySet(Vec<Capability>);

impl CapabilitySet {
    pub fn new(caps: impl IntoIterator<Item = Capability>) -> Self {
        let mut v: Vec<Capability> = caps.into_iter().collect();
        v.sort();
        v.dedup();
        Self(v)
    }

    pub fn has(&self, cap: Capability) -> bool {
        self.0.binary_search(&cap).is_ok()
    }

    /// Whether the backend can drive the overlay with authoritative live state.
    pub fn is_authoritative(&self) -> bool {
        self.has(Capability::StreamLayerStack)
    }

    pub fn iter(&self) -> impl Iterator<Item = Capability> + '_ {
        self.0.iter().copied()
    }
}

/// A typed Backend Health value (ADR 0033).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum HealthState {
    /// The backend is connected and reporting normally.
    Ok,
    /// A required OS permission is missing (e.g. macOS input monitoring).
    PermissionMissing { permission: String, detail: String },
    /// The device or transport is disconnected.
    Disconnected { detail: String },
    /// Data is being served but is older than the freshness budget.
    Stale { detail: String },
    /// The device or source is recognized but unsupported.
    Unsupported { detail: String },
    /// A source payload failed to parse.
    ParseError { detail: String },
    /// A backend protocol-level error occurred.
    ProtocolError { detail: String },
}

impl HealthState {
    /// Whether the overlay should treat this backend as currently usable.
    pub fn is_ok(&self) -> bool {
        matches!(self, HealthState::Ok)
    }

    /// A stable machine tag for the state, used by the frontend for styling.
    pub fn tag(&self) -> &'static str {
        match self {
            HealthState::Ok => "ok",
            HealthState::PermissionMissing { .. } => "permission-missing",
            HealthState::Disconnected { .. } => "disconnected",
            HealthState::Stale { .. } => "stale",
            HealthState::Unsupported { .. } => "unsupported",
            HealthState::ParseError { .. } => "parse-error",
            HealthState::ProtocolError { .. } => "protocol-error",
        }
    }
}

/// The full health report for one backend, carried in snapshots and events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendHealth {
    pub backend_id: String,
    /// Human-facing backend name for the UI.
    pub name: String,
    pub capabilities: CapabilitySet,
    pub health: HealthState,
}

impl BackendHealth {
    pub fn new(
        backend_id: impl Into<String>,
        name: impl Into<String>,
        capabilities: CapabilitySet,
        health: HealthState,
    ) -> Self {
        Self {
            backend_id: backend_id.into(),
            name: name.into(),
            capabilities,
            health,
        }
    }
}
