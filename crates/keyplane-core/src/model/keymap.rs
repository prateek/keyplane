//! Logical Keymap: layer data keyed by Stable Element IDs (ADR 0005, 0028).
//!
//! Layers store entries by [`KeyId`], not row position, so imports and
//! migrations survive geometry changes. Each entry preserves its
//! [`RawAction`] and a derived [`SemanticAction`].

use crate::action::{RawAction, SemanticAction};
use crate::ids::{KeyId, LayerId};
use crate::provenance::Provenance;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One key's binding on one layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerEntry {
    pub raw: RawAction,
    pub semantic: SemanticAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

impl LayerEntry {
    pub fn new(raw: RawAction, semantic: SemanticAction) -> Self {
        Self {
            raw,
            semantic,
            provenance: None,
        }
    }

    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }
}

/// A single layer: an ordered position plus its per-key entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    /// Display index / firmware layer number.
    pub index: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Entries keyed by [`KeyId`]. A missing key on a layer is treated as
    /// transparent during resolution.
    pub entries: BTreeMap<KeyId, LayerEntry>,
}

impl Layer {
    pub fn new(id: impl Into<LayerId>, index: u16) -> Self {
        Self {
            id: id.into(),
            index,
            name: None,
            entries: BTreeMap::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn set(mut self, key: impl Into<KeyId>, entry: LayerEntry) -> Self {
        self.entries.insert(key.into(), entry);
        self
    }

    pub fn entry(&self, key: &KeyId) -> Option<&LayerEntry> {
        self.entries.get(key)
    }
}

/// The full Logical Keymap: ordered layers plus the base/default layer id.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalKeymap {
    pub layers: Vec<Layer>,
    /// The id of the base layer that is always at the bottom of the Layer Stack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_layer: Option<LayerId>,
}

impl LogicalKeymap {
    pub fn new(layers: Vec<Layer>) -> Self {
        let default_layer = layers.first().map(|l| l.id.clone());
        Self {
            layers,
            default_layer,
        }
    }

    pub fn with_default(mut self, layer: impl Into<LayerId>) -> Self {
        self.default_layer = Some(layer.into());
        self
    }

    pub fn layer(&self, id: &LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| &l.id == id)
    }

    pub fn layer_by_index(&self, index: u16) -> Option<&Layer> {
        self.layers.iter().find(|l| l.index == index)
    }
}
