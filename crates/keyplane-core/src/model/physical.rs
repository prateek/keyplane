//! Physical Layout: per-key coordinate geometry (ADR 0028).

use crate::geometry::{KeyGeometry, MatrixPosition};
use crate::ids::KeyId;
use crate::provenance::Provenance;
use serde::{Deserialize, Serialize};

/// An individual Physical Key, identified independently of its label or action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicalKey {
    pub id: KeyId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix: Option<MatrixPosition>,
    pub geometry: KeyGeometry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

impl PhysicalKey {
    pub fn new(id: impl Into<KeyId>, geometry: KeyGeometry) -> Self {
        Self {
            id: id.into(),
            matrix: None,
            geometry,
            provenance: None,
        }
    }

    pub fn with_matrix(mut self, matrix: MatrixPosition) -> Self {
        self.matrix = Some(matrix);
        self
    }

    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }
}

/// The geometry and identity of every physical key on the board.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PhysicalLayout {
    pub keys: Vec<PhysicalKey>,
    /// True when this layout was inferred from a source lacking real per-key
    /// geometry (e.g. OverKeys row arrays imported as a Fallback Layout).
    #[serde(default, skip_serializing_if = "is_false")]
    pub fallback: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl PhysicalLayout {
    pub fn new(keys: Vec<PhysicalKey>) -> Self {
        Self {
            keys,
            fallback: false,
        }
    }

    /// Mark this as a Fallback Layout (ADR 0028) — lower-fidelity geometry.
    pub fn as_fallback(mut self) -> Self {
        self.fallback = true;
        self
    }

    pub fn key(&self, id: &KeyId) -> Option<&PhysicalKey> {
        self.keys.iter().find(|k| &k.id == id)
    }

    /// The bounding box `(width, height)` in keycap units, for overlay sizing.
    pub fn extent(&self) -> (f64, f64) {
        let mut w = 0.0_f64;
        let mut h = 0.0_f64;
        for key in &self.keys {
            w = w.max(key.geometry.x + key.geometry.w);
            h = h.max(key.geometry.y + key.geometry.h);
        }
        (w, h)
    }
}
