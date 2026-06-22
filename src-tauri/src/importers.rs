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
    #[error("JSON parse failed: {0}")]
    Json(String),
    #[error("Import is missing {0}")]
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

pub fn import_keyviz_style_json(
    contents: &str,
    base_profile: &Profile,
) -> Result<ImportCandidate, ImportError> {
    let json: JsonValue =
        serde_json::from_str(contents).map_err(|err| ImportError::Json(err.to_string()))?;
    let keyviz_style = json
        .pointer("/appearance/style")
        .and_then(JsonValue::as_str)
        .ok_or(ImportError::Missing("appearance.style"))?;
    let preserved_sections = preserved_keyviz_style_sections(&json)?;
    let source = Source {
        id: format!("keyviz-style-{}", sanitize_id(keyviz_style)),
        name: format!("keyviz {} style", keyviz_style),
        kind: "keyviz-style-import".to_string(),
        authority: SourceAuthority::BestEffortPreview,
    };
    let imported_style = VisualStyle {
        variant_id: format!("keyviz-{}", sanitize_id(keyviz_style)),
        density: keyviz_density(keyviz_style),
    };
    let active_style_source_id = base_profile
        .sources
        .first()
        .map(|source| source.id.clone())
        .unwrap_or_else(|| "active-profile".to_string());
    let mut preview_profile = base_profile.clone();

    if !preview_profile
        .sources
        .iter()
        .any(|candidate| candidate.id == source.id)
    {
        preview_profile.sources.push(source.clone());
    }
    preview_profile.visual_style = imported_style.clone();
    preview_profile.source_provenance.push(SourceRef {
        source_id: source.id.clone(),
        field_path: ":visual/style".to_string(),
        raw: Some(contents.trim().to_string()),
    });
    promote_style_precedence(&mut preview_profile, &source.id, &active_style_source_id);

    let conflicts = if base_profile.visual_style.variant_id == imported_style.variant_id {
        Vec::new()
    } else {
        vec![SourceConflict {
            field_path: ":visual/style :style/variant-id".to_string(),
            selected_source_id: source.id.clone(),
            candidates: vec![
                crate::domain::SourceCandidate {
                    source_id: active_style_source_id,
                    value: base_profile.visual_style.variant_id.clone(),
                    selected: false,
                },
                crate::domain::SourceCandidate {
                    source_id: source.id.clone(),
                    value: imported_style.variant_id.clone(),
                    selected: true,
                },
            ],
        }]
    };

    Ok(ImportCandidate {
        id: format!("candidate-{}", source.id),
        source,
        best_effort_preview: true,
        preview_profile,
        conflicts,
        summary: ImportSummary {
            imported_keys: 0,
            imported_layers: 0,
            preserved_sections,
        },
    })
}

fn preserved_keyviz_style_sections(json: &JsonValue) -> Result<Vec<String>, ImportError> {
    let required_sections = [
        "appearance",
        "layout",
        "color",
        "modifier",
        "text",
        "border",
        "background",
        "mouse",
    ];

    for section in required_sections {
        if !json.get(section).is_some_and(JsonValue::is_object) {
            return Err(ImportError::Missing(section));
        }
    }

    Ok(required_sections
        .into_iter()
        .map(|section| section.to_string())
        .collect())
}

fn keyviz_density(keyviz_style: &str) -> StyleDensity {
    match keyviz_style {
        "minimal" => StyleDensity::Compact,
        "laptop" | "lowprofile" | "pbt" => StyleDensity::Rich,
        _ => StyleDensity::Standard,
    }
}

fn promote_style_precedence(profile: &mut Profile, source_id: &str, active_style_source_id: &str) {
    if let Some(rule) = profile
        .source_precedence
        .iter_mut()
        .find(|rule| rule.field_scope == ":visual/style")
    {
        rule.source_order
            .retain(|candidate| candidate != source_id && candidate != active_style_source_id);
        rule.source_order
            .insert(0, active_style_source_id.to_string());
        rule.source_order.insert(0, source_id.to_string());
        return;
    }

    profile.source_precedence.push(SourcePrecedenceRule {
        field_scope: ":visual/style".to_string(),
        source_order: vec![source_id.to_string(), active_style_source_id.to_string()],
    });
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

    const KEYVIZ_STYLE_FIXTURE: &str = r##"
    {
      "appearance": {
        "monitor": null,
        "flexDirection": "column",
        "alignment": "bottom-center",
        "marginX": 100,
        "marginY": 100,
        "animation": "fade",
        "animationDuration": 0.25,
        "style": "lowprofile"
      },
      "layout": {
        "showIcon": true,
        "showSymbol": true,
        "showPressCount": true,
        "iconAlignment": "flex-end"
      },
      "color": {
        "color": "#ffffff",
        "secondaryColor": "#1a1a1a",
        "useGradient": true
      },
      "modifier": {
        "highlight": false,
        "color": "#3a86ff",
        "secondaryColor": "#000000",
        "textColor": "#000000",
        "borderColor": "#000000"
      },
      "text": {
        "size": 32,
        "color": "#000000",
        "caps": "capitalize",
        "variant": "text-short",
        "alignment": "center"
      },
      "border": {
        "enabled": true,
        "color": "#1a1a1a",
        "width": 2,
        "radius": 0.5
      },
      "background": {
        "enabled": true,
        "color": "#ffffff99"
      },
      "mouse": {
        "showClicks": false,
        "size": 150,
        "color": "#009dff",
        "keepHighlight": false,
        "showIndicator": true,
        "keepIndicator": true,
        "indicatorSize": 50,
        "indicatorOffsetX": 50,
        "indicatorOffsetY": 50
      }
    }
    "##;

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

    #[test]
    fn keyviz_style_import_only_changes_visual_style() {
        let base_profile = crate::fake_backend::fake_profile();
        let candidate =
            import_keyviz_style_json(KEYVIZ_STYLE_FIXTURE, &base_profile).expect("style imports");

        assert_eq!(candidate.source.kind, "keyviz-style-import");
        assert_eq!(candidate.summary.imported_keys, 0);
        assert_eq!(candidate.summary.imported_layers, 0);
        assert_eq!(
            candidate.preview_profile.visual_style.variant_id,
            "keyviz-lowprofile"
        );
        assert_eq!(
            candidate.preview_profile.visual_style.density,
            StyleDensity::Rich
        );
        assert_eq!(
            candidate.preview_profile.physical_layout,
            base_profile.physical_layout
        );
        assert_eq!(candidate.preview_profile.keymap, base_profile.keymap);
    }

    #[test]
    fn keyviz_style_import_exposes_a_visual_style_source_conflict() {
        let base_profile = crate::fake_backend::fake_profile();
        let candidate =
            import_keyviz_style_json(KEYVIZ_STYLE_FIXTURE, &base_profile).expect("style imports");

        assert_eq!(candidate.conflicts.len(), 1);
        assert_eq!(
            candidate.conflicts[0].field_path,
            ":visual/style :style/variant-id"
        );
        assert_eq!(
            candidate.conflicts[0].selected_source_id,
            "keyviz-style-lowprofile"
        );
        assert!(candidate.conflicts[0].candidates.iter().any(|candidate| {
            candidate.source_id == "fake-backend"
                && candidate.value == "keyplane-default"
                && !candidate.selected
        }));
        assert!(candidate.conflicts[0].candidates.iter().any(|candidate| {
            candidate.source_id == "keyviz-style-lowprofile"
                && candidate.value == "keyviz-lowprofile"
                && candidate.selected
        }));
    }
}
