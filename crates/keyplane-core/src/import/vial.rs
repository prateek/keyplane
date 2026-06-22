//! NocFree/Vial `.vil` file import (ADR 0007, 0046).
//!
//! A `.vil` export carries the Logical Keymap (`layout` as a `[layer][row][col]`
//! array of keycodes) but no per-key geometry — geometry lives in the keyboard
//! definition, not the export. So the importer builds a Fallback Layout from the
//! matrix dimensions and marks the candidate as Best-Effort Preview: a `.vil`
//! file cannot report live layer state.

use super::{ImportCandidate, ImportError, Importer};
use crate::action::RawAction;
use crate::geometry::{KeyGeometry, MatrixPosition};
use crate::ids::{KeyId, LayerId, SourceId};
use crate::model::keymap::{Layer, LayerEntry, LogicalKeymap};
use crate::model::physical::{PhysicalKey, PhysicalLayout};
use crate::model::KeyboardModel;
use crate::profile::SourceRef;
use crate::provenance::{Provenance, SourceKind};
use crate::resolve::semantic;
use serde_json::Value as JsonValue;
use std::collections::BTreeSet;

/// Imports Vial `.vil` JSON exports.
pub struct VialFileImporter {
    id: String,
}

impl Default for VialFileImporter {
    fn default() -> Self {
        Self {
            id: "vial-file".to_string(),
        }
    }
}

impl VialFileImporter {
    pub fn new() -> Self {
        Self::default()
    }
}

/// The matrix key id for a `(row, col)` cell, stable across imports.
fn key_id(row: usize, col: usize) -> KeyId {
    KeyId::new(format!("r{row}c{col}"))
}

fn layer_id(index: usize) -> LayerId {
    LayerId::new(format!("layer-{index}"))
}

impl Importer for VialFileImporter {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Vial
    }

    fn import(&self, input: &str) -> Result<ImportCandidate, ImportError> {
        let json: JsonValue = serde_json::from_str(input).map_err(|e| ImportError::Parse {
            format: "vil".to_string(),
            detail: e.to_string(),
        })?;

        let layout = json.get("layout").and_then(JsonValue::as_array).ok_or_else(|| {
            ImportError::Unsupported {
                format: "vil".to_string(),
                detail: "missing `layout` array".to_string(),
            }
        })?;

        let source_id = SourceId::new(&self.id);
        let prov = |raw: &str| Provenance::new(source_id.clone(), SourceKind::Vial).with_raw(raw);
        let resolver = |n: u16| layer_id(n as usize);

        // Collect the set of populated matrix cells across all layers so the
        // Fallback Layout covers every key the keymap references.
        let mut cells: BTreeSet<(usize, usize)> = BTreeSet::new();
        let mut layers = Vec::new();

        for (layer_index, layer_json) in layout.iter().enumerate() {
            let rows = layer_json.as_array().ok_or_else(|| ImportError::Unsupported {
                format: "vil".to_string(),
                detail: format!("layer {layer_index} is not an array"),
            })?;
            let mut layer = Layer::new(layer_id(layer_index), layer_index as u16);
            for (row_index, row_json) in rows.iter().enumerate() {
                let cols = row_json.as_array().ok_or_else(|| ImportError::Unsupported {
                    format: "vil".to_string(),
                    detail: format!("layer {layer_index} row {row_index} is not an array"),
                })?;
                for (col_index, cell) in cols.iter().enumerate() {
                    let Some(raw) = cell_to_raw(cell) else {
                        // `-1` / null marks an absent matrix position; skip it.
                        continue;
                    };
                    cells.insert((row_index, col_index));
                    let token = raw.token();
                    let semantic = semantic::derive(&raw, &resolver);
                    layer.entries.insert(
                        key_id(row_index, col_index),
                        LayerEntry::new(raw, semantic).with_provenance(prov(&token)),
                    );
                }
            }
            layers.push(layer);
        }

        // Fallback Layout: a 1u grid placed at matrix coordinates.
        let keys = cells
            .iter()
            .map(|&(row, col)| {
                PhysicalKey::new(key_id(row, col), KeyGeometry::unit(col as f64, row as f64))
                    .with_matrix(MatrixPosition::new(row as u16, col as u16))
                    .with_provenance(prov("vil-matrix"))
            })
            .collect();
        let physical = PhysicalLayout::new(keys).as_fallback();

        let keymap = LogicalKeymap::new(layers).with_default(layer_id(0));
        let name = json
            .get("name")
            .and_then(JsonValue::as_str)
            .map(String::from);
        let mut model = KeyboardModel::new(physical, keymap);
        model.name = name.clone();

        Ok(ImportCandidate {
            source: SourceRef {
                id: source_id,
                kind: SourceKind::Vial,
                label: name.or_else(|| Some("Vial file".to_string())),
            },
            model,
            notes: vec![
                "Best-Effort Preview: a .vil file has no live layer state.".to_string(),
                "Geometry is a Fallback Layout inferred from the key matrix.".to_string(),
            ],
            best_effort_preview: true,
        })
    }
}

/// Interpret one `.vil` keymap cell. Strings are QMK tokens; integers are VIA
/// numeric codes; `-1`/null/empty mark an absent matrix position.
fn cell_to_raw(cell: &JsonValue) -> Option<RawAction> {
    match cell {
        JsonValue::String(s) if !s.is_empty() => Some(RawAction::Qmk(s.clone())),
        JsonValue::Number(n) => {
            let code = n.as_i64()?;
            if code < 0 {
                None
            } else {
                Some(RawAction::ViaCode(code as u16))
            }
        }
        _ => None,
    }
}
