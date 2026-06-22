//! OverKeys-style configuration import (ADR 0008, 0028).
//!
//! OverKeys is an import target and design inspiration, never a code source. Its
//! configuration is row-array based, so it imports only as a Fallback Layout:
//! each row becomes a line of 1u keys with synthetic geometry. This also backs
//! the Kanata MVP path, where an OverKeys-style companion profile supplies the
//! keymap/layout data Kanata itself does not (ADR 0010).

use super::{ImportCandidate, ImportError, Importer};
use crate::action::RawAction;
use crate::geometry::KeyGeometry;
use crate::ids::{KeyId, LayerId, SourceId};
use crate::model::keymap::{Layer, LayerEntry, LogicalKeymap};
use crate::model::physical::{PhysicalKey, PhysicalLayout};
use crate::model::KeyboardModel;
use crate::profile::SourceRef;
use crate::provenance::{Provenance, SourceKind};
use crate::resolve::semantic;
use serde_json::Value as JsonValue;

/// Imports OverKeys-style row-array configurations.
pub struct OverKeysImporter {
    id: String,
}

impl Default for OverKeysImporter {
    fn default() -> Self {
        Self {
            id: "overkeys".to_string(),
        }
    }
}

impl OverKeysImporter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Importer for OverKeysImporter {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> SourceKind {
        SourceKind::OverKeys
    }

    fn import(&self, input: &str) -> Result<ImportCandidate, ImportError> {
        let json: JsonValue = serde_json::from_str(input).map_err(|e| ImportError::Parse {
            format: "overkeys".to_string(),
            detail: e.to_string(),
        })?;

        // Accept either `{ "layout": [[..]] }` or a bare `[[..]]` rows array.
        let rows = json
            .get("layout")
            .or_else(|| json.get("keys"))
            .unwrap_or(&json)
            .as_array()
            .ok_or_else(|| ImportError::Unsupported {
                format: "overkeys".to_string(),
                detail: "expected a `layout` array of rows".to_string(),
            })?;

        let source_id = SourceId::new(&self.id);
        let prov = |raw: &str| Provenance::new(source_id.clone(), SourceKind::OverKeys).with_raw(raw);
        let resolver = |n: u16| LayerId::new(format!("layer-{n}"));

        let mut keys = Vec::new();
        let mut layer = Layer::new(LayerId::new("layer-0"), 0).with_name("Base");

        for (row_index, row) in rows.iter().enumerate() {
            let cols = row.as_array().ok_or_else(|| ImportError::Unsupported {
                format: "overkeys".to_string(),
                detail: format!("row {row_index} is not an array"),
            })?;
            for (col_index, cell) in cols.iter().enumerate() {
                let label = cell.as_str().unwrap_or("").to_string();
                if label.is_empty() {
                    continue;
                }
                let id = KeyId::new(format!("r{row_index}c{col_index}"));
                keys.push(
                    PhysicalKey::new(id.clone(), KeyGeometry::unit(col_index as f64, row_index as f64))
                        .with_provenance(prov("overkeys-row")),
                );
                let raw = RawAction::HostEvent(label.clone());
                let semantic = semantic::derive(&raw, &resolver);
                layer
                    .entries
                    .insert(id, LayerEntry::new(raw, semantic).with_provenance(prov(&label)));
            }
        }

        let physical = PhysicalLayout::new(keys).as_fallback();
        let keymap = LogicalKeymap::new(vec![layer]).with_default(LayerId::new("layer-0"));
        let name = json.get("name").and_then(JsonValue::as_str).map(String::from);
        let mut model = KeyboardModel::new(physical, keymap);
        model.name = name.clone();

        Ok(ImportCandidate {
            source: SourceRef {
                id: source_id,
                kind: SourceKind::OverKeys,
                label: name.or_else(|| Some("OverKeys config".to_string())),
            },
            model,
            notes: vec!["Row arrays imported as a Fallback Layout.".to_string()],
            best_effort_preview: true,
        })
    }
}
