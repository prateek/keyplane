//! Source Provenance (ADR 0005, ADR 0014, ADR 0019).
//!
//! Every value the app renders records which source supplied it and what raw
//! representation that source carried. Losing values in a Source Conflict stay
//! inspectable through provenance even though the overlay renders only winners.

use crate::ids::SourceId;
use serde::{Deserialize, Serialize};

/// The class of a source, used by Source Precedence (ADR 0018) and the Source
/// Inspector. The ordering of variants is not significant; precedence is
/// resolved per field in [`crate::precedence`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    /// KeyPeek firmware module — authoritative for Runtime State.
    KeyPeek,
    /// Vial `.vil` file or live Vial.
    Vial,
    /// VIA keymap data.
    Via,
    /// ZMK / ZMK Studio data.
    Zmk,
    /// OverKeys-style configuration (import target only; ADR 0008).
    OverKeys,
    /// keyviz style JSON — affects Visual Style only (ADR 0013).
    Keyviz,
    /// Kanata remapper — authoritative for runtime layer state (ADR 0009).
    Kanata,
    /// Sentinel-key backend driven by Host Input Events (ADR 0016).
    Sentinel,
    /// The deterministic Fake Backend used for dev/test/demo.
    Fake,
    /// A user-authored value (User Override) — always wins (ADR 0018).
    User,
}

impl SourceKind {
    /// Whether this source is firmware-aware/authoritative for Runtime State.
    pub fn is_authoritative_runtime(self) -> bool {
        matches!(self, SourceKind::KeyPeek | SourceKind::Kanata | SourceKind::Fake)
    }
}

/// A record of which source supplied a value and the raw form it carried.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub source: SourceId,
    pub kind: SourceKind,
    /// The raw representation preserved from the source, if any (e.g. the
    /// original `.vil` keycode string). Kept for debugging and Source Inspector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

impl Provenance {
    pub fn new(source: impl Into<SourceId>, kind: SourceKind) -> Self {
        Self {
            source: source.into(),
            kind,
            raw: None,
        }
    }

    pub fn with_raw(mut self, raw: impl Into<String>) -> Self {
        self.raw = Some(raw.into());
        self
    }
}

/// A value paired with its Source Provenance.
///
/// Conflict resolution keeps every candidate (the winner plus losing
/// `alternatives`) so the Source Inspector can show the full picture and the
/// user can promote any candidate to a User Override.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sourced<T> {
    pub value: T,
    pub provenance: Provenance,
    /// Losing candidates for the same field, retained but not rendered.
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<Candidate<T>>,
}

/// One candidate value for a field, with its provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate<T> {
    pub value: T,
    pub provenance: Provenance,
}

impl<T> Sourced<T> {
    pub fn new(value: T, provenance: Provenance) -> Self {
        Self {
            value,
            provenance,
            alternatives: Vec::new(),
        }
    }

    /// Whether this field has a Source Conflict (more than one candidate).
    pub fn has_conflict(&self) -> bool {
        !self.alternatives.is_empty()
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Sourced<U> {
        Sourced {
            value: f(self.value),
            provenance: self.provenance,
            alternatives: Vec::new(),
        }
    }
}
