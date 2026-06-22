use crate::domain::{
    derive_action, BackendHealth, BackendStatus, CapabilityFlag, DisplayTargeting, HealthState,
    ImportCandidate, ImportSummary, KeyGeometry, Layer, LogicalKeymap, MatrixPosition,
    OverlayWindowConfig, PhysicalKey, PhysicalLayout, Profile, Source, SourceAuthority,
    SourceConflict, SourcePrecedenceRule, SourceRef, StyleDensity, VisibilityPolicy, VisualStyle,
};
use serde_json::Value as JsonValue;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ImportError {
    #[error("Vial JSON parse failed: {0}")]
    Json(String),
    #[error("Vial import is missing {0}")]
    Missing(&'static str),
}

pub fn import_vial_json(contents: &str) -> Result<ImportCandidate, ImportError> {
    let json: JsonValue =
        serde_json::from_str(contents).map_err(|err| ImportError::Json(err.to_string()))?;
    let uid = json
        .get("uid")
        .and_then(JsonValue::as_str)
        .unwrap_or("vial-file");
    let key_rows = json
        .pointer("/layouts/keymap")
        .or_else(|| json.get("layout"))
        .and_then(JsonValue::as_array)
        .ok_or(ImportError::Missing("layouts.keymap"))?;
    let source = Source {
        id: format!("vial-file-{}", sanitize_id(uid)),
        name: format!("Vial file {}", uid),
        kind: "vial-file-import".to_string(),
        authority: SourceAuthority::BestEffortPreview,
    };
    let physical_keys = import_kle_rows_as_fallback_layout(key_rows, &source.id);
    let layers = import_layers(&json, &physical_keys, &source.id);
    let preserved_sections = preserved_top_level_sections(&json);
    let backend_health = BackendHealth {
        backend_id: source.id.clone(),
        state: HealthState::Stale,
        message: "Imported file provides Best-Effort Preview only; no live layer channel"
            .to_string(),
    };
    let profile = Profile {
        schema_version: 1,
        id: format!("profile-{}", source.id),
        name: format!("{} Preview", source.name),
        sources: vec![source.clone()],
        physical_layout: PhysicalLayout {
            keys: physical_keys.clone(),
            fallback: true,
        },
        keymap: LogicalKeymap {
            layers: layers.clone(),
        },
        runtime_backends: vec![BackendStatus {
            id: source.id.clone(),
            name: source.name.clone(),
            capabilities: vec![
                CapabilityFlag::ImportGeometry,
                CapabilityFlag::ImportKeymaps,
                CapabilityFlag::PreviewOnly,
            ],
            health: backend_health,
        }],
        visual_style: VisualStyle {
            variant_id: "vial-preview".to_string(),
            density: StyleDensity::Standard,
        },
        overlay_window: OverlayWindowConfig {
            visibility: VisibilityPolicy::Pinned,
            click_through: true,
            positioning_mode: false,
            display_targeting: DisplayTargeting {
                display_id: None,
                x: 80.0,
                y: 80.0,
                width: 920.0,
                height: 320.0,
                opacity: 0.9,
            },
        },
        source_precedence: vec![SourcePrecedenceRule {
            field_scope: ":keyboard/physical-layout".to_string(),
            source_order: vec![source.id.clone(), "user-overrides".to_string()],
        }],
        user_overrides: Vec::new(),
        source_provenance: physical_keys
            .iter()
            .map(|key| key.provenance.clone())
            .collect(),
    };

    Ok(ImportCandidate {
        id: format!("candidate-{}", source.id),
        source,
        best_effort_preview: true,
        preview_profile: profile,
        conflicts: Vec::<SourceConflict>::new(),
        summary: ImportSummary {
            imported_keys: physical_keys.len(),
            imported_layers: layers.len(),
            preserved_sections,
        },
    })
}

fn import_kle_rows_as_fallback_layout(rows: &[JsonValue], source_id: &str) -> Vec<PhysicalKey> {
    let mut keys = Vec::new();
    let mut cursor_y = 0.0_f32;

    for row in rows {
        let Some(items) = row.as_array() else {
            continue;
        };
        let mut cursor_x = 0.0_f32;
        let mut next_width = 1.0_f32;
        let mut next_height = 1.0_f32;
        let mut next_rotation = 0.0_f32;

        for item in items {
            if let Some(props) = item.as_object() {
                if let Some(x) = props.get("x").and_then(JsonValue::as_f64) {
                    cursor_x += x as f32;
                }
                if let Some(y) = props.get("y").and_then(JsonValue::as_f64) {
                    cursor_y += y as f32;
                }
                if let Some(width) = props.get("w").and_then(JsonValue::as_f64) {
                    next_width = width as f32;
                }
                if let Some(height) = props.get("h").and_then(JsonValue::as_f64) {
                    next_height = height as f32;
                }
                if let Some(rotation) = props.get("r").and_then(JsonValue::as_f64) {
                    next_rotation = rotation as f32;
                }
                continue;
            }

            let Some(label) = item.as_str() else {
                continue;
            };
            let matrix = parse_vial_matrix(label);
            let id = matrix
                .as_ref()
                .map(|matrix| format!("vial-r{}c{}", matrix.row, matrix.col))
                .unwrap_or_else(|| format!("vial-key-{}", keys.len()));
            keys.push(PhysicalKey {
                id: id.clone(),
                matrix,
                geometry: KeyGeometry {
                    x: cursor_x,
                    y: cursor_y,
                    width: next_width,
                    height: next_height,
                    rotation: next_rotation,
                },
                provenance: SourceRef {
                    source_id: source_id.to_string(),
                    field_path: format!(":keyboard/physical-layout {}", id),
                    raw: Some(label.to_string()),
                },
            });
            cursor_x += next_width;
            next_width = 1.0;
            next_height = 1.0;
            next_rotation = 0.0;
        }
        cursor_y += 1.0;
    }

    keys
}

fn import_layers(json: &JsonValue, keys: &[PhysicalKey], source_id: &str) -> Vec<Layer> {
    let layer_values = json
        .get("layers")
        .or_else(|| json.pointer("/keymap/layers"))
        .and_then(JsonValue::as_array);

    let Some(layer_values) = layer_values else {
        return vec![Layer {
            id: "layer-0".to_string(),
            name: "Imported".to_string(),
            actions: keys
                .iter()
                .map(|key| {
                    derive_action(
                        "vial",
                        key.provenance.raw.as_deref().unwrap_or("KC_NO"),
                        SourceRef {
                            source_id: source_id.to_string(),
                            field_path: format!(":keyboard/keymap :layer-0 {}", key.id),
                            raw: key.provenance.raw.clone(),
                        },
                        &key.id,
                    )
                })
                .collect(),
        }];
    };

    layer_values
        .iter()
        .enumerate()
        .filter_map(|(layer_index, layer)| {
            let values = layer.as_array()?;
            Some(Layer {
                id: format!("layer-{}", layer_index),
                name: format!("Layer {}", layer_index),
                actions: keys
                    .iter()
                    .enumerate()
                    .map(|(key_index, key)| {
                        let raw = values
                            .get(key_index)
                            .and_then(JsonValue::as_str)
                            .unwrap_or("KC_NO");
                        derive_action(
                            "vial",
                            raw,
                            SourceRef {
                                source_id: source_id.to_string(),
                                field_path: format!(
                                    ":keyboard/keymap :layer-{} {}",
                                    layer_index, key.id
                                ),
                                raw: Some(raw.to_string()),
                            },
                            &key.id,
                        )
                    })
                    .collect(),
            })
        })
        .collect()
}

fn parse_vial_matrix(label: &str) -> Option<MatrixPosition> {
    let first_line = label.lines().next()?.trim();
    let (row, col) = first_line.split_once(',')?;
    Some(MatrixPosition {
        row: row.trim().parse().ok()?,
        col: col.trim().parse().ok()?,
    })
}

fn preserved_top_level_sections(json: &JsonValue) -> Vec<String> {
    let Some(object) = json.as_object() else {
        return Vec::new();
    };

    let mut sections: Vec<String> = object
        .keys()
        .filter(|key| !matches!(key.as_str(), "layouts" | "layout" | "layers"))
        .cloned()
        .collect();
    sections.sort();
    sections
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIAL_FIXTURE: &str = r#"
    {
      "uid": "NocFree-Example",
      "protocol": 6,
      "layouts": {
        "keymap": [
          ["0,0", "0,1"],
          [{"w": 1.5}, "1,0", "1,1"]
        ]
      },
      "layers": [
        ["KC_ESC", "KC_Q", "KC_TAB", "KC_A"],
        ["KC_TRNS", "KC_UP", "KC_TRNS", "KC_LEFT"]
      ],
      "macros": [{"name": "hello"}],
      "combos": []
    }
    "#;

    #[test]
    fn vial_file_import_produces_best_effort_import_candidate() {
        let candidate = import_vial_json(VIAL_FIXTURE).expect("fixture imports");

        assert!(candidate.best_effort_preview);
        assert!(candidate.preview_profile.physical_layout.fallback);
        assert_eq!(candidate.summary.imported_keys, 4);
        assert_eq!(candidate.summary.imported_layers, 2);
        assert!(candidate
            .summary
            .preserved_sections
            .contains(&"macros".to_string()));
        assert_eq!(
            candidate.preview_profile.runtime_backends[0].health.state,
            HealthState::Stale
        );
    }

    #[test]
    fn vial_import_preserves_raw_actions_in_source_provenance() {
        let candidate = import_vial_json(VIAL_FIXTURE).expect("fixture imports");
        let nav = &candidate.preview_profile.keymap.layers[1];

        assert_eq!(nav.actions[0].raw.value, "KC_TRNS");
        assert_eq!(nav.actions[0].provenance.raw.as_deref(), Some("KC_TRNS"));
    }
}
