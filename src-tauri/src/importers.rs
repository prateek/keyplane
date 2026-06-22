use crate::domain::{
    default_kanata_tcp_config, derive_action, ActivationKind, BackendConfig, BackendHealth,
    BackendStatus, CapabilityFlag, DisplayTargeting, HealthState, ImportCandidate, ImportSummary,
    KeyGeometry, Layer, LegendSlot, LegendSlotKind, LogicalKeymap, MatrixPosition,
    OverlayWindowConfig, PhysicalKey, PhysicalLayout, Profile, SentinelKeyBinding, Source,
    SourceAuthority, SourceConflict, SourcePrecedenceRule, SourceRef, StyleDensity,
    VisibilityPolicy, VisualStyle, VisualStyleColors,
};
use crate::kanata_backend;
use crate::sentinel_backend;
use qmk_via_api::keycodes::Keycode;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ImportError {
    #[error("JSON parse failed: {0}")]
    Json(String),
    #[error("Import is missing {0}")]
    Missing(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VialDeviceSnapshot {
    pub vid: u16,
    pub pid: u16,
    pub uid: String,
    pub protocol_version: u32,
    pub definition_json: JsonValue,
    pub raw_matrices: Vec<Vec<Vec<u16>>>,
}

pub fn import_vial_json(contents: &str) -> Result<ImportCandidate, ImportError> {
    let json: JsonValue =
        serde_json::from_str(contents).map_err(|err| ImportError::Json(err.to_string()))?;
    let uid = json
        .get("uid")
        .and_then(json_scalar_to_string)
        .unwrap_or_else(|| "vial-file".to_string());
    let source = Source {
        id: format!("vial-file-{}", sanitize_id(&uid)),
        name: format!("Vial file {}", uid),
        kind: "vial-file-import".to_string(),
        authority: SourceAuthority::BestEffortPreview,
    };
    let layer_values = import_vial_layer_values(&json);
    let physical_keys = vial_geometry_rows(&json)
        .map(|rows| import_kle_rows_as_fallback_layout(rows, &source.id))
        .filter(|keys| !keys.is_empty())
        .or_else(|| {
            layer_values
                .as_ref()
                .map(|layers| import_matrix_rows_as_fallback_layout(layers, &source.id, "vial"))
                .filter(|keys| !keys.is_empty())
        })
        .ok_or(ImportError::Missing("layouts.keymap or layout"))?;
    let layers = import_layers(layer_values.as_deref(), &physical_keys, &source.id, "vial");
    let preserved_sections = preserved_top_level_sections(&json);
    let mut source_provenance: Vec<SourceRef> = physical_keys
        .iter()
        .map(|key| key.provenance.clone())
        .collect();
    source_provenance.extend(top_level_source_refs(
        &json,
        &source.id,
        &preserved_sections,
    ));
    let backend_health = BackendHealth {
        backend_id: source.id.clone(),
        state: HealthState::Stale,
        message: "Imported file provides Best-Effort Preview only; no live layer channel"
            .to_string(),
    };
    let profile = Profile {
        schema_version: 1,
        id: format!("profile-{}", source.id),
        keyboard_id: format!("keyboard-{}", source.id),
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
            config: None,
        }],
        sentinel_keys: Vec::new(),
        visual_style: VisualStyle {
            id: "style-vial-preview".to_string(),
            variant_id: "vial-preview".to_string(),
            density: StyleDensity::Standard,
            colors: VisualStyleColors::default(),
        },
        overlay_window: OverlayWindowConfig {
            visibility: VisibilityPolicy::Pinned,
            visible: true,
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
        source_precedence: vec![
            SourcePrecedenceRule {
                field_scope: ":keyboard/physical-layout".to_string(),
                source_order: vec!["user-overrides".to_string(), source.id.clone()],
            },
            SourcePrecedenceRule {
                field_scope: ":keyboard/keymap".to_string(),
                source_order: vec!["user-overrides".to_string(), source.id.clone()],
            },
        ],
        user_overrides: Vec::new(),
        source_provenance,
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

pub fn import_vial_device_snapshot(
    snapshot: VialDeviceSnapshot,
) -> Result<ImportCandidate, ImportError> {
    let uid_slug = sanitize_id_or(&snapshot.uid, "device");
    let source = Source {
        id: format!(
            "vial-device-{:04x}-{:04x}-{}",
            snapshot.vid, snapshot.pid, uid_slug
        ),
        name: format!("Vial device {:04x}:{:04x}", snapshot.vid, snapshot.pid),
        kind: "vial-device-import".to_string(),
        authority: SourceAuthority::BestEffortPreview,
    };
    let layer_values = raw_matrices_to_imported_layers(&snapshot.raw_matrices);
    if layer_values.is_empty() {
        return Err(ImportError::Missing("Vial device keymap matrix"));
    }
    let physical_keys = vial_geometry_rows(&snapshot.definition_json)
        .map(|rows| import_kle_rows_as_fallback_layout(rows, &source.id))
        .filter(|keys| !keys.is_empty())
        .or_else(|| {
            if layer_values.is_empty() {
                None
            } else {
                let keys = import_matrix_rows_as_fallback_layout(&layer_values, &source.id, "vial");
                (!keys.is_empty()).then_some(keys)
            }
        })
        .ok_or(ImportError::Missing(
            "Vial device layouts.keymap or keymap matrix",
        ))?;
    let has_geometry = vial_geometry_rows(&snapshot.definition_json).is_some();
    let layers = import_layers(Some(&layer_values), &physical_keys, &source.id, "vial");
    let preserved_sections = vec![
        "vial-device-definition".to_string(),
        "vial-device-keymap".to_string(),
    ];
    let mut source_provenance: Vec<SourceRef> = physical_keys
        .iter()
        .map(|key| key.provenance.clone())
        .collect();
    source_provenance.push(SourceRef {
        source_id: source.id.clone(),
        field_path: ":source/raw vial-device-definition".to_string(),
        raw: Some(snapshot.definition_json.to_string()),
    });
    source_provenance.push(SourceRef {
        source_id: source.id.clone(),
        field_path: ":source/raw vial-device-keymap".to_string(),
        raw: Some(format!("{:?}", snapshot.raw_matrices)),
    });
    source_provenance.push(SourceRef {
        source_id: source.id.clone(),
        field_path: ":source/raw vial-device-protocol".to_string(),
        raw: Some(format!(
            "vial_protocol={} vid={:04x} pid={:04x} uid={}",
            snapshot.protocol_version, snapshot.vid, snapshot.pid, snapshot.uid
        )),
    });
    let backend_health = BackendHealth {
        backend_id: source.id.clone(),
        state: HealthState::Stale,
        message:
            "Imported Vial device geometry and keymap as Best-Effort Preview; no live layer channel"
                .to_string(),
    };
    let profile = Profile {
        schema_version: 1,
        id: format!("profile-{}", source.id),
        keyboard_id: format!("keyboard-{}", source.id),
        name: format!("{} Preview", source.name),
        sources: vec![source.clone()],
        physical_layout: PhysicalLayout {
            keys: physical_keys.clone(),
            fallback: !has_geometry,
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
            config: None,
        }],
        sentinel_keys: Vec::new(),
        visual_style: VisualStyle {
            id: "style-vial-device-preview".to_string(),
            variant_id: "vial-device-preview".to_string(),
            density: StyleDensity::Standard,
            colors: VisualStyleColors::default(),
        },
        overlay_window: OverlayWindowConfig {
            visibility: VisibilityPolicy::Pinned,
            visible: true,
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
        source_precedence: vec![
            SourcePrecedenceRule {
                field_scope: ":keyboard/physical-layout".to_string(),
                source_order: vec!["user-overrides".to_string(), source.id.clone()],
            },
            SourcePrecedenceRule {
                field_scope: ":keyboard/keymap".to_string(),
                source_order: vec!["user-overrides".to_string(), source.id.clone()],
            },
        ],
        user_overrides: Vec::new(),
        source_provenance,
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
        id: format!("style-keyviz-{}", sanitize_id(keyviz_style)),
        variant_id: format!("keyviz-{}", sanitize_id(keyviz_style)),
        density: keyviz_density(keyviz_style),
        colors: keyviz_style_colors(&json),
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

    let conflicts = visual_style_conflicts(
        &base_profile.visual_style,
        &imported_style,
        &active_style_source_id,
        &source.id,
    );

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

pub fn import_overkeys_companion_json(contents: &str) -> Result<ImportCandidate, ImportError> {
    let json: JsonValue =
        serde_json::from_str(contents).map_err(|err| ImportError::Json(err.to_string()))?;
    let layouts = overkeys_layouts(&json);
    let layer_values: Vec<Vec<ImportedLayerCell>> =
        layouts.iter().map(|layout| layout.cells.clone()).collect();

    if layer_values.is_empty() {
        return Err(ImportError::Missing("userLayouts.keys"));
    }

    let display_mappings = overkeys_display_mappings(&json);
    let kanata_config = overkeys_kanata_config(&json);
    let sentinel_keys = overkeys_sentinel_bindings(&layouts, &json);
    let primary_layout_name = layouts
        .first()
        .map(|layout| layout.name.as_str())
        .unwrap_or("OverKeys layout");
    let source_suffix = sanitize_id_or(primary_layout_name, "layout");
    let source = Source {
        id: format!("overkeys-companion-{}", source_suffix),
        name: format!("OverKeys {} companion", primary_layout_name),
        kind: "overkeys-companion-import".to_string(),
        authority: SourceAuthority::BestEffortPreview,
    };
    let kanata_source = Source {
        id: kanata_backend::KANATA_BACKEND_ID.to_string(),
        name: "Kanata TCP".to_string(),
        kind: "kanata".to_string(),
        authority: SourceAuthority::Authoritative,
    };
    let sentinel_source = Source {
        id: sentinel_backend::SENTINEL_BACKEND_ID.to_string(),
        name: "Sentinel Keys".to_string(),
        kind: "sentinel-keys".to_string(),
        authority: SourceAuthority::Inferred,
    };
    let physical_keys =
        import_matrix_rows_as_fallback_layout(&layer_values, &source.id, "overkeys");
    let mut layers = import_layers(
        Some(layer_values.as_slice()),
        &physical_keys,
        &source.id,
        "overkeys",
    );
    apply_overkeys_display_mappings(&mut layers, &display_mappings);

    for (layer, layout) in layers.iter_mut().zip(layouts.iter()) {
        layer.name = layout.name.clone();
    }

    let preserved_sections = preserved_top_level_sections(&json);
    let mut source_provenance: Vec<SourceRef> = physical_keys
        .iter()
        .map(|key| key.provenance.clone())
        .collect();
    source_provenance.extend(top_level_source_refs(
        &json,
        &source.id,
        &preserved_sections,
    ));
    let backend_health = BackendHealth {
        backend_id: source.id.clone(),
        state: HealthState::Stale,
        message: "Imported OverKeys companion profile provides renderable layout/keymap data as Best-Effort Preview"
            .to_string(),
    };
    let mut kanata_backend = kanata_backend::kanata_backend_status(
        HealthState::Disconnected,
        "Kanata TCP runtime is not connected; imported OverKeys companion profile supplies layout/keymap data",
    );
    kanata_backend.config = Some(kanata_config);
    let mut sources = vec![source.clone(), kanata_source];
    let mut runtime_backends = vec![
        BackendStatus {
            id: source.id.clone(),
            name: source.name.clone(),
            capabilities: vec![
                CapabilityFlag::ImportGeometry,
                CapabilityFlag::ImportKeymaps,
                CapabilityFlag::PreviewOnly,
            ],
            health: backend_health,
            config: None,
        },
        kanata_backend,
    ];
    let mut runtime_source_order = vec![kanata_backend::KANATA_BACKEND_ID.to_string()];
    if !sentinel_keys.is_empty() {
        sources.push(sentinel_source);
        runtime_backends.push(sentinel_backend::sentinel_backend_status(
            HealthState::PermissionMissing,
            "Input monitoring permission is required before imported OverKeys triggers can infer layers",
        ));
        runtime_source_order.push(sentinel_backend::SENTINEL_BACKEND_ID.to_string());
    }
    runtime_source_order.push(source.id.clone());
    let profile = Profile {
        schema_version: 1,
        id: format!("profile-{}", source.id),
        keyboard_id: format!("keyboard-{}", source.id),
        name: format!("{} Preview", source.name),
        sources,
        physical_layout: PhysicalLayout {
            keys: physical_keys.clone(),
            fallback: true,
        },
        keymap: LogicalKeymap {
            layers: layers.clone(),
        },
        runtime_backends,
        sentinel_keys,
        visual_style: VisualStyle {
            id: "style-overkeys-preview".to_string(),
            variant_id: "overkeys-preview".to_string(),
            density: StyleDensity::Standard,
            colors: VisualStyleColors::default(),
        },
        overlay_window: OverlayWindowConfig {
            visibility: VisibilityPolicy::Pinned,
            visible: true,
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
        source_precedence: vec![
            SourcePrecedenceRule {
                field_scope: ":runtime/state".to_string(),
                source_order: runtime_source_order,
            },
            SourcePrecedenceRule {
                field_scope: ":keyboard/physical-layout".to_string(),
                source_order: vec!["user-overrides".to_string(), source.id.clone()],
            },
            SourcePrecedenceRule {
                field_scope: ":keyboard/keymap".to_string(),
                source_order: vec!["user-overrides".to_string(), source.id.clone()],
            },
        ],
        user_overrides: Vec::new(),
        source_provenance,
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

pub fn import_zmk_keymap(contents: &str) -> Result<ImportCandidate, ImportError> {
    let parsed_layers = parse_zmk_keymap_layers(contents);

    if parsed_layers.is_empty() {
        return Err(ImportError::Missing("keymap bindings"));
    }

    let source = Source {
        id: "zmk-keymap".to_string(),
        name: "ZMK keymap".to_string(),
        kind: "zmk-keymap-import".to_string(),
        authority: SourceAuthority::BestEffortPreview,
    };
    let layer_values: Vec<Vec<ImportedLayerCell>> = parsed_layers
        .iter()
        .map(|layer| layer.cells.clone())
        .collect();
    let physical_keys = import_matrix_rows_as_fallback_layout(&layer_values, &source.id, "zmk");
    let mut layers = import_layers(
        Some(layer_values.as_slice()),
        &physical_keys,
        &source.id,
        "zmk",
    );

    for (layer, parsed_layer) in layers.iter_mut().zip(parsed_layers.iter()) {
        layer.name = parsed_layer.name.clone();
    }

    let preserved_sections = vec!["zmk-keymap".to_string()];
    let mut source_provenance: Vec<SourceRef> = physical_keys
        .iter()
        .map(|key| key.provenance.clone())
        .collect();
    source_provenance.push(SourceRef {
        source_id: source.id.clone(),
        field_path: ":source/raw zmk-keymap".to_string(),
        raw: Some(contents.trim().to_string()),
    });
    let backend_health = BackendHealth {
        backend_id: source.id.clone(),
        state: HealthState::Stale,
        message: "Imported ZMK keymap is Best-Effort Preview; no live ZMK or KeyPeek connection"
            .to_string(),
    };
    let profile = Profile {
        schema_version: 1,
        id: format!("profile-{}", source.id),
        keyboard_id: format!("keyboard-{}", source.id),
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
            config: None,
        }],
        sentinel_keys: Vec::new(),
        visual_style: VisualStyle {
            id: "style-zmk-preview".to_string(),
            variant_id: "zmk-preview".to_string(),
            density: StyleDensity::Standard,
            colors: VisualStyleColors::default(),
        },
        overlay_window: OverlayWindowConfig {
            visibility: VisibilityPolicy::Pinned,
            visible: true,
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
        source_precedence: vec![
            SourcePrecedenceRule {
                field_scope: ":keyboard/physical-layout".to_string(),
                source_order: vec!["user-overrides".to_string(), source.id.clone()],
            },
            SourcePrecedenceRule {
                field_scope: ":keyboard/keymap".to_string(),
                source_order: vec!["user-overrides".to_string(), source.id.clone()],
            },
        ],
        user_overrides: Vec::new(),
        source_provenance,
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

fn keyviz_style_colors(json: &JsonValue) -> VisualStyleColors {
    VisualStyleColors {
        keycap_background: json_color_at(json, "/color/color"),
        keycap_text: json_color_at(json, "/text/color"),
        keycap_border: json_color_at(json, "/border/color"),
        modifier_accent: json_color_at(json, "/modifier/color"),
        overlay_background: json_color_at(json, "/background/color"),
    }
}

fn json_color_at(json: &JsonValue, pointer: &str) -> Option<String> {
    json.pointer(pointer)
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| is_hex_color(value))
        .map(ToOwned::to_owned)
}

fn visual_style_conflicts(
    base_style: &VisualStyle,
    imported_style: &VisualStyle,
    active_style_source_id: &str,
    imported_style_source_id: &str,
) -> Vec<SourceConflict> {
    let mut conflicts = Vec::new();
    push_style_conflict(
        &mut conflicts,
        ":visual/style :style/id",
        &base_style.id,
        &imported_style.id,
        active_style_source_id,
        imported_style_source_id,
    );
    push_style_conflict(
        &mut conflicts,
        ":visual/style :style/variant-id",
        &base_style.variant_id,
        &imported_style.variant_id,
        active_style_source_id,
        imported_style_source_id,
    );
    push_style_conflict(
        &mut conflicts,
        ":visual/style :style/density",
        style_density_label(&base_style.density),
        style_density_label(&imported_style.density),
        active_style_source_id,
        imported_style_source_id,
    );
    push_style_conflict(
        &mut conflicts,
        ":visual/style :style/colors :color/keycap-background",
        style_color_value(&base_style.colors.keycap_background),
        style_color_value(&imported_style.colors.keycap_background),
        active_style_source_id,
        imported_style_source_id,
    );
    push_style_conflict(
        &mut conflicts,
        ":visual/style :style/colors :color/keycap-text",
        style_color_value(&base_style.colors.keycap_text),
        style_color_value(&imported_style.colors.keycap_text),
        active_style_source_id,
        imported_style_source_id,
    );
    push_style_conflict(
        &mut conflicts,
        ":visual/style :style/colors :color/keycap-border",
        style_color_value(&base_style.colors.keycap_border),
        style_color_value(&imported_style.colors.keycap_border),
        active_style_source_id,
        imported_style_source_id,
    );
    push_style_conflict(
        &mut conflicts,
        ":visual/style :style/colors :color/modifier-accent",
        style_color_value(&base_style.colors.modifier_accent),
        style_color_value(&imported_style.colors.modifier_accent),
        active_style_source_id,
        imported_style_source_id,
    );
    push_style_conflict(
        &mut conflicts,
        ":visual/style :style/colors :color/overlay-background",
        style_color_value(&base_style.colors.overlay_background),
        style_color_value(&imported_style.colors.overlay_background),
        active_style_source_id,
        imported_style_source_id,
    );
    conflicts
}

fn push_style_conflict(
    conflicts: &mut Vec<SourceConflict>,
    field_path: &str,
    active_value: &str,
    imported_value: &str,
    active_style_source_id: &str,
    imported_style_source_id: &str,
) {
    if active_value == imported_value {
        return;
    }

    conflicts.push(SourceConflict {
        field_path: field_path.to_string(),
        selected_source_id: imported_style_source_id.to_string(),
        candidates: vec![
            crate::domain::SourceCandidate {
                source_id: active_style_source_id.to_string(),
                value: active_value.to_string(),
                selected: false,
            },
            crate::domain::SourceCandidate {
                source_id: imported_style_source_id.to_string(),
                value: imported_value.to_string(),
                selected: true,
            },
        ],
    });
}

fn style_color_value(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("nil")
}

fn style_density_label(value: &StyleDensity) -> &'static str {
    match value {
        StyleDensity::Compact => "compact",
        StyleDensity::Standard => "standard",
        StyleDensity::Rich => "rich",
    }
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn promote_style_precedence(profile: &mut Profile, source_id: &str, active_style_source_id: &str) {
    if let Some(rule) = profile
        .source_precedence
        .iter_mut()
        .find(|rule| rule.field_scope == ":visual/style")
    {
        rule.source_order.retain(|candidate| {
            candidate != "user-overrides"
                && candidate != source_id
                && candidate != active_style_source_id
        });
        rule.source_order
            .insert(0, active_style_source_id.to_string());
        rule.source_order.insert(0, source_id.to_string());
        rule.source_order.insert(0, "user-overrides".to_string());
        return;
    }

    profile.source_precedence.push(SourcePrecedenceRule {
        field_scope: ":visual/style".to_string(),
        source_order: vec![
            "user-overrides".to_string(),
            source_id.to_string(),
            active_style_source_id.to_string(),
        ],
    });
}

#[derive(Debug, Clone)]
struct ImportedLayerCell {
    raw: String,
    row: Option<usize>,
    col: Option<usize>,
}

struct OverkeysLayout {
    name: String,
    cells: Vec<ImportedLayerCell>,
    trigger: Option<String>,
    activation: Option<ActivationKind>,
}

#[derive(Debug, Clone, Default)]
struct OverkeysDisplayMappings {
    aliases: BTreeMap<String, String>,
    custom_aliases: BTreeMap<String, String>,
    custom_shift_mappings: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct ZmkParsedLayer {
    name: String,
    cells: Vec<ImportedLayerCell>,
}

fn json_scalar_to_string(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(value) => Some(value.clone()),
        JsonValue::Number(value) => Some(value.to_string()),
        JsonValue::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn parse_zmk_keymap_layers(contents: &str) -> Vec<ZmkParsedLayer> {
    let mut layers = Vec::new();
    let mut current_layer_name: Option<String> = None;
    let mut collecting_bindings = false;
    let mut collected_layer_name = String::new();
    let mut row_index = 0_usize;
    let mut cells = Vec::new();

    for raw_line in contents.lines() {
        let line = strip_zmk_line_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if !collecting_bindings {
            if let Some(layer_name) = zmk_layer_name_from_line(line) {
                current_layer_name = Some(layer_name);
            }

            if let Some((_, after_open)) = line.split_once('<') {
                if line.contains("bindings") {
                    collecting_bindings = true;
                    collected_layer_name = current_layer_name
                        .clone()
                        .unwrap_or_else(|| format!("layer-{}", layers.len()));
                    cells.clear();
                    row_index = 0;
                    let (row, done) = zmk_binding_row_segment(after_open);
                    row_index += push_zmk_binding_row(&row, row_index, &mut cells);
                    if done {
                        finish_zmk_layer(&mut layers, &mut cells, &collected_layer_name);
                        collecting_bindings = false;
                    }
                }
            }
        } else {
            let (row, done) = zmk_binding_row_segment(line);
            row_index += push_zmk_binding_row(&row, row_index, &mut cells);
            if done {
                finish_zmk_layer(&mut layers, &mut cells, &collected_layer_name);
                collecting_bindings = false;
            }
        }

        if line.contains("};") {
            current_layer_name = None;
        }
    }

    layers
}

fn strip_zmk_line_comment(line: &str) -> &str {
    line.split_once("//")
        .map(|(before, _)| before)
        .unwrap_or(line)
}

fn zmk_layer_name_from_line(line: &str) -> Option<String> {
    let before_open = line.strip_suffix('{')?.trim();
    if before_open.is_empty()
        || before_open == "/"
        || before_open == "keymap"
        || before_open.contains("compatible")
    {
        return None;
    }

    let candidate = before_open
        .split(':')
        .next_back()
        .unwrap_or(before_open)
        .split_whitespace()
        .next()?;

    if candidate == "keymap" || candidate == "/" {
        None
    } else {
        Some(candidate.to_string())
    }
}

fn zmk_binding_row_segment(line: &str) -> (String, bool) {
    let end_index = line
        .find(">;")
        .or_else(|| line.find('>'))
        .unwrap_or(line.len());
    let done = end_index < line.len();
    let row = line[..end_index]
        .replace("bindings", "")
        .replace('=', "")
        .replace('<', "")
        .replace(';', "");

    (row.trim().to_string(), done)
}

fn push_zmk_binding_row(row: &str, row_index: usize, cells: &mut Vec<ImportedLayerCell>) -> usize {
    let bindings = parse_zmk_binding_row(row);
    if bindings.is_empty() {
        return 0;
    }

    for (col_index, raw) in bindings.into_iter().enumerate() {
        cells.push(ImportedLayerCell {
            raw,
            row: Some(row_index),
            col: Some(col_index),
        });
    }

    1
}

fn parse_zmk_binding_row(row: &str) -> Vec<String> {
    let tokens: Vec<&str> = row.split_whitespace().collect();
    let mut bindings = Vec::new();
    let mut index = 0_usize;

    while index < tokens.len() {
        let token = tokens[index];
        if !token.starts_with('&') {
            index += 1;
            continue;
        }

        let arity = zmk_binding_arity(token);
        let end = (index + 1 + arity).min(tokens.len());
        bindings.push(tokens[index..end].join(" "));
        index = end;
    }

    bindings
}

fn zmk_binding_arity(behavior: &str) -> usize {
    match behavior.to_ascii_lowercase().as_str() {
        "&trans" | "&none" => 0,
        "&kp" | "&mo" | "&tog" | "&to" => 1,
        "&lt" | "&mt" => 2,
        _ => 1,
    }
}

fn finish_zmk_layer(
    layers: &mut Vec<ZmkParsedLayer>,
    cells: &mut Vec<ImportedLayerCell>,
    layer_name: &str,
) {
    if cells.is_empty() {
        return;
    }

    layers.push(ZmkParsedLayer {
        name: layer_name.to_string(),
        cells: std::mem::take(cells),
    });
}

fn overkeys_layouts(json: &JsonValue) -> Vec<OverkeysLayout> {
    let default_name = json.get("defaultUserLayout").and_then(JsonValue::as_str);
    let Some(user_layouts) = json.get("userLayouts").and_then(JsonValue::as_array) else {
        return Vec::new();
    };
    let mut layouts: Vec<OverkeysLayout> = user_layouts
        .iter()
        .filter_map(overkeys_layout_from_json)
        .collect();

    if let Some(default_name) = default_name {
        if let Some(index) = layouts
            .iter()
            .position(|layout| layout.name == default_name)
        {
            let default_layout = layouts.remove(index);
            layouts.insert(0, default_layout);
        }
    }

    layouts
}

fn overkeys_layout_from_json(layout: &JsonValue) -> Option<OverkeysLayout> {
    let rows = layout.get("keys").and_then(JsonValue::as_array)?;
    let cells = row_arrays_to_layer_cells(rows);
    if cells.is_empty() {
        return None;
    }
    let name = layout
        .get("name")
        .and_then(json_scalar_to_string)
        .unwrap_or_else(|| "OverKeys layout".to_string());

    Some(OverkeysLayout {
        name,
        cells,
        trigger: layout
            .get("trigger")
            .and_then(json_scalar_to_string)
            .map(|trigger| trigger.trim().to_string())
            .filter(|trigger| !trigger.is_empty()),
        activation: overkeys_activation_kind(layout.get("type").and_then(json_scalar_to_string)),
    })
}

fn overkeys_aliases(json: &JsonValue) -> BTreeMap<String, String> {
    json.get("aliases")
        .and_then(JsonValue::as_object)
        .map(|aliases| {
            aliases
                .iter()
                .filter_map(|(raw, label)| {
                    json_scalar_to_string(label).map(|label| (raw.clone(), label))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn overkeys_display_mappings(json: &JsonValue) -> OverkeysDisplayMappings {
    OverkeysDisplayMappings {
        aliases: overkeys_aliases(json),
        custom_aliases: overkeys_custom_aliases(json),
        custom_shift_mappings: overkeys_custom_shift_mappings(json),
    }
}

fn overkeys_custom_aliases(json: &JsonValue) -> BTreeMap<String, String> {
    json.get("customAliases")
        .and_then(JsonValue::as_object)
        .map(|aliases| {
            aliases
                .iter()
                .filter_map(|(raw, combo)| {
                    overkeys_combo_label(combo).map(|label| (raw.clone(), label))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn overkeys_combo_label(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::Array(values) => {
            let labels: Vec<String> = values
                .iter()
                .filter_map(json_scalar_to_string)
                .map(|label| label.trim().to_string())
                .filter(|label| !label.is_empty())
                .collect();
            if labels.is_empty() {
                None
            } else {
                Some(labels.join(" + "))
            }
        }
        _ => json_scalar_to_string(value)
            .map(|label| label.trim().to_string())
            .filter(|label| !label.is_empty()),
    }
}

fn overkeys_custom_shift_mappings(json: &JsonValue) -> BTreeMap<String, String> {
    json.get("customShiftMappings")
        .and_then(JsonValue::as_object)
        .map(|mappings| {
            mappings
                .iter()
                .filter_map(|(raw, shifted)| {
                    json_scalar_to_string(shifted)
                        .map(|shifted| shifted.trim().to_string())
                        .filter(|shifted| !shifted.is_empty())
                        .map(|shifted| (raw.clone(), shifted))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn overkeys_sentinel_bindings(
    layouts: &[OverkeysLayout],
    json: &JsonValue,
) -> Vec<SentinelKeyBinding> {
    let trigger_overrides = overkeys_trigger_overrides(json);
    layouts
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(layer_index, layout)| {
            let host_input_code = layout.trigger.clone().or_else(|| {
                overkeys_trigger_override_for_layout(&layout.name, &trigger_overrides)
            })?;
            Some(SentinelKeyBinding {
                host_input_code,
                layer_id: format!("layer-{}", layer_index),
                activation: layout
                    .activation
                    .clone()
                    .unwrap_or(ActivationKind::Momentary),
            })
        })
        .collect()
}

fn overkeys_trigger_overrides(json: &JsonValue) -> BTreeMap<String, String> {
    json.get("triggers")
        .and_then(JsonValue::as_object)
        .map(|triggers| {
            triggers
                .iter()
                .filter_map(|(layer_name, host_input_code)| {
                    json_scalar_to_string(host_input_code)
                        .map(|host_input_code| host_input_code.trim().to_string())
                        .filter(|host_input_code| !host_input_code.is_empty())
                        .map(|host_input_code| (layer_name.clone(), host_input_code))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn overkeys_trigger_override_for_layout(
    layout_name: &str,
    triggers: &BTreeMap<String, String>,
) -> Option<String> {
    triggers.get(layout_name).cloned().or_else(|| {
        triggers
            .iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(layout_name))
            .map(|(_, host_input_code)| host_input_code.clone())
    })
}

fn overkeys_activation_kind(value: Option<String>) -> Option<ActivationKind> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "held" | "momentary" => Some(ActivationKind::Momentary),
        "toggle" => Some(ActivationKind::Toggle),
        "lock" => Some(ActivationKind::Lock),
        _ => Some(ActivationKind::Unknown),
    }
}

fn overkeys_kanata_config(json: &JsonValue) -> BackendConfig {
    let BackendConfig::KanataTcp {
        host: default_host,
        port: default_port,
    } = default_kanata_tcp_config();
    BackendConfig::KanataTcp {
        host: json
            .get("kanataHost")
            .and_then(JsonValue::as_str)
            .map(str::trim)
            .filter(|host| !host.is_empty())
            .map(str::to_string)
            .unwrap_or(default_host),
        port: json
            .get("kanataPort")
            .and_then(json_tcp_port)
            .unwrap_or(default_port),
    }
}

fn json_tcp_port(value: &JsonValue) -> Option<u16> {
    match value {
        JsonValue::Number(number) => number
            .as_u64()
            .and_then(|port| u16::try_from(port).ok())
            .filter(|port| *port > 0),
        JsonValue::String(text) => text.trim().parse::<u16>().ok().filter(|port| *port > 0),
        _ => None,
    }
}

fn apply_overkeys_display_mappings(layers: &mut [Layer], mappings: &OverkeysDisplayMappings) {
    if mappings.aliases.is_empty()
        && mappings.custom_aliases.is_empty()
        && mappings.custom_shift_mappings.is_empty()
    {
        return;
    }

    for action in layers.iter_mut().flat_map(|layer| layer.actions.iter_mut()) {
        if let Some(label) = mappings.aliases.get(&action.raw.value) {
            action.semantic.label = label.clone();
            upsert_overkeys_legend_slot(&mut action.legend.slots, LegendSlotKind::Primary, label);
        }

        if let Some(shifted) = mappings.custom_shift_mappings.get(&action.raw.value) {
            upsert_overkeys_legend_slot(&mut action.legend.slots, LegendSlotKind::Shifted, shifted);
        }

        if let Some(combo) = mappings.custom_aliases.get(&action.raw.value) {
            upsert_overkeys_legend_slot(
                &mut action.legend.slots,
                LegendSlotKind::ActionHint,
                combo,
            );
        }
    }
}

fn upsert_overkeys_legend_slot(slots: &mut Vec<LegendSlot>, kind: LegendSlotKind, text: &str) {
    if let Some(slot) = slots.iter_mut().find(|slot| slot.slot == kind) {
        slot.text = text.to_string();
        return;
    }

    let index = match kind {
        LegendSlotKind::Primary => 0,
        LegendSlotKind::Shifted => slots
            .iter()
            .position(|slot| slot.slot != LegendSlotKind::Primary)
            .unwrap_or(slots.len()),
        LegendSlotKind::ActionHint => slots
            .iter()
            .position(|slot| slot.slot == LegendSlotKind::Icon)
            .unwrap_or(slots.len()),
        _ => slots.len(),
    };
    slots.insert(
        index,
        LegendSlot {
            slot: kind,
            text: text.to_string(),
        },
    );
}

fn row_arrays_to_layer_cells(rows: &[JsonValue]) -> Vec<ImportedLayerCell> {
    rows.iter()
        .enumerate()
        .flat_map(|(row_index, row)| {
            row.as_array().into_iter().flat_map(move |row_values| {
                row_values
                    .iter()
                    .enumerate()
                    .filter_map(move |(col_index, value)| {
                        raw_action_from_json(value).map(|raw| ImportedLayerCell {
                            raw,
                            row: Some(row_index),
                            col: Some(col_index),
                        })
                    })
            })
        })
        .collect()
}

fn vial_geometry_rows(json: &JsonValue) -> Option<&[JsonValue]> {
    json.pointer("/layouts/keymap")
        .and_then(JsonValue::as_array)
        .map(|rows| rows.as_slice())
        .or_else(|| {
            let layout = json.get("layout")?;
            if is_layer_matrix(layout) {
                return None;
            }
            let rows = layout.as_array()?;
            if rows.iter().any(row_looks_like_kle) {
                Some(rows.as_slice())
            } else {
                None
            }
        })
}

fn row_looks_like_kle(row: &JsonValue) -> bool {
    let Some(items) = row.as_array() else {
        return false;
    };

    items.iter().any(|item| {
        item.is_string()
            || item.as_object().is_some_and(|props| {
                props
                    .keys()
                    .any(|key| matches!(key.as_str(), "x" | "y" | "w" | "h" | "r" | "rx" | "ry"))
            })
    })
}

fn import_vial_layer_values(json: &JsonValue) -> Option<Vec<Vec<ImportedLayerCell>>> {
    let layer_source = json
        .get("layers")
        .or_else(|| json.pointer("/keymap/layers"))
        .or_else(|| json.get("layout").filter(|value| is_layer_matrix(value)))?;
    let layers = flatten_layer_collection(layer_source);

    if layers.is_empty() {
        None
    } else {
        Some(layers)
    }
}

pub(crate) fn vial_matrix_dimensions(json: &JsonValue) -> Option<(usize, usize)> {
    let keys = vial_geometry_rows(json)?;
    let mut max_row = None::<u16>;
    let mut max_col = None::<u16>;

    for key in import_kle_rows_as_fallback_layout(keys, "vial-device-dimensions") {
        let Some(matrix) = key.matrix else {
            continue;
        };
        max_row = Some(max_row.map_or(matrix.row, |candidate| candidate.max(matrix.row)));
        max_col = Some(max_col.map_or(matrix.col, |candidate| candidate.max(matrix.col)));
    }

    Some((usize::from(max_row?) + 1, usize::from(max_col?) + 1))
}

fn is_layer_matrix(value: &JsonValue) -> bool {
    let Some(layers) = value.as_array() else {
        return false;
    };

    !layers.is_empty()
        && layers.iter().all(|layer| {
            let Some(rows) = layer.as_array() else {
                return false;
            };
            !rows.is_empty()
                && rows.iter().all(|row| {
                    row.as_array().is_some_and(|cells| {
                        !cells.is_empty()
                            && cells
                                .iter()
                                .all(|cell| raw_action_from_json(cell).is_some())
                    })
                })
        })
}

fn raw_matrices_to_imported_layers(raw_matrices: &[Vec<Vec<u16>>]) -> Vec<Vec<ImportedLayerCell>> {
    raw_matrices
        .iter()
        .map(|layer| {
            layer
                .iter()
                .enumerate()
                .flat_map(|(row_index, row)| {
                    row.iter()
                        .enumerate()
                        .map(move |(col_index, keycode)| ImportedLayerCell {
                            raw: qmk_keycode_label(*keycode),
                            row: Some(row_index),
                            col: Some(col_index),
                        })
                })
                .collect()
        })
        .collect()
}

fn qmk_keycode_label(keycode: u16) -> String {
    match keycode {
        0x0000 => "KC_NO".to_string(),
        0x0001 => "KC_TRNS".to_string(),
        _ => Keycode::try_from(keycode)
            .map(|keycode| normalize_qmk_keycode_label(keycode.as_ref()))
            .unwrap_or_else(|_| format!("0x{keycode:04X}")),
    }
}

fn normalize_qmk_keycode_label(value: &str) -> String {
    value
        .replace("KC_TRANSPARENT", "KC_TRNS")
        .replace("KC_ESCAPE", "KC_ESC")
        .replace("KC_BACKSPACE", "KC_BSPC")
        .replace("KC_SPACE", "KC_SPC")
        .replace("KC_LEFT_CTRL", "KC_LCTL")
        .replace("KC_RIGHT_CTRL", "KC_RCTL")
        .replace("KC_LEFT_SHIFT", "KC_LSFT")
        .replace("KC_RIGHT_SHIFT", "KC_RSFT")
        .replace("KC_LEFT_ALT", "KC_LALT")
        .replace("KC_RIGHT_ALT", "KC_RALT")
        .replace("KC_LEFT_GUI", "KC_LGUI")
        .replace("KC_RIGHT_GUI", "KC_RGUI")
}

fn flatten_layer_collection(layer_source: &JsonValue) -> Vec<Vec<ImportedLayerCell>> {
    let Some(layers) = layer_source.as_array() else {
        return Vec::new();
    };

    layers.iter().filter_map(flatten_layer).collect()
}

fn flatten_layer(layer: &JsonValue) -> Option<Vec<ImportedLayerCell>> {
    let values = layer.as_array()?;
    let cells: Vec<ImportedLayerCell> = if values.iter().all(JsonValue::is_array) {
        values
            .iter()
            .enumerate()
            .flat_map(|(row_index, row)| {
                row.as_array().into_iter().flat_map(move |row_values| {
                    row_values
                        .iter()
                        .enumerate()
                        .filter_map(move |(col_index, value)| {
                            raw_action_from_json(value).map(|raw| ImportedLayerCell {
                                raw,
                                row: Some(row_index),
                                col: Some(col_index),
                            })
                        })
                })
            })
            .collect()
    } else {
        values
            .iter()
            .filter_map(|value| {
                raw_action_from_json(value).map(|raw| ImportedLayerCell {
                    raw,
                    row: None,
                    col: None,
                })
            })
            .collect()
    };

    if cells.is_empty() {
        None
    } else {
        Some(cells)
    }
}

fn raw_action_from_json(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(value) => Some(value.clone()),
        JsonValue::Number(value) => Some(value.to_string()),
        JsonValue::Bool(value) => Some(value.to_string()),
        JsonValue::Array(_) | JsonValue::Object(_) => serde_json::to_string(value).ok(),
        JsonValue::Null => None,
    }
}

fn import_matrix_rows_as_fallback_layout(
    layers: &[Vec<ImportedLayerCell>],
    source_id: &str,
    key_prefix: &str,
) -> Vec<PhysicalKey> {
    let Some(base_layer) = layers.iter().find(|layer| !layer.is_empty()) else {
        return Vec::new();
    };
    let inferred_columns = inferred_column_count(base_layer);

    base_layer
        .iter()
        .enumerate()
        .map(|(key_index, cell)| {
            let matrix = matrix_from_cell(cell);
            let (grid_row, grid_col) = matrix
                .as_ref()
                .map(|matrix| (usize::from(matrix.row), usize::from(matrix.col)))
                .unwrap_or_else(|| (key_index / inferred_columns, key_index % inferred_columns));
            let id = matrix
                .as_ref()
                .map(|matrix| format!("{}-r{}c{}", key_prefix, matrix.row, matrix.col))
                .unwrap_or_else(|| format!("{}-key-{}", key_prefix, key_index));

            PhysicalKey {
                id: id.clone(),
                matrix,
                geometry: KeyGeometry {
                    x: grid_col as f32,
                    y: grid_row as f32,
                    width: 1.0,
                    height: 1.0,
                    rotation: 0.0,
                },
                provenance: SourceRef {
                    source_id: source_id.to_string(),
                    field_path: format!(":keyboard/physical-layout {}", id),
                    raw: Some(fallback_matrix_source_label(cell, key_index)),
                },
            }
        })
        .collect()
}

fn inferred_column_count(cells: &[ImportedLayerCell]) -> usize {
    cells
        .iter()
        .filter_map(|cell| cell.col)
        .max()
        .map(|col| col + 1)
        .unwrap_or_else(|| cells.len().max(1))
        .max(1)
}

fn matrix_from_cell(cell: &ImportedLayerCell) -> Option<MatrixPosition> {
    Some(MatrixPosition {
        row: u16::try_from(cell.row?).ok()?,
        col: u16::try_from(cell.col?).ok()?,
    })
}

fn fallback_matrix_source_label(cell: &ImportedLayerCell, key_index: usize) -> String {
    match (cell.row, cell.col) {
        (Some(row), Some(col)) => format!("layout matrix {},{}", row, col),
        _ => format!("layer index {}", key_index),
    }
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

fn import_layers(
    layer_values: Option<&[Vec<ImportedLayerCell>]>,
    keys: &[PhysicalKey],
    source_id: &str,
    dialect: &str,
) -> Vec<Layer> {
    let Some(layer_values) = layer_values else {
        return vec![Layer {
            id: "layer-0".to_string(),
            name: "Imported".to_string(),
            actions: keys
                .iter()
                .map(|key| {
                    derive_action(
                        dialect,
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
        .map(|(layer_index, layer)| Layer {
            id: format!("layer-{}", layer_index),
            name: format!("Layer {}", layer_index),
            actions: keys
                .iter()
                .enumerate()
                .map(|(key_index, key)| {
                    let raw = layer_cell_for_key(layer, key_index, key)
                        .map(|cell| cell.raw.as_str())
                        .unwrap_or("KC_NO");
                    derive_action(
                        dialect,
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
        .collect()
}

fn layer_cell_for_key<'a>(
    layer: &'a [ImportedLayerCell],
    key_index: usize,
    key: &PhysicalKey,
) -> Option<&'a ImportedLayerCell> {
    if let Some(matrix) = &key.matrix {
        if let Some(cell) = layer.iter().find(|cell| {
            cell.row == Some(usize::from(matrix.row)) && cell.col == Some(usize::from(matrix.col))
        }) {
            return Some(cell);
        }
    }

    layer.get(key_index)
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

    let mut sections: Vec<String> = object.keys().cloned().collect();
    sections.sort();
    sections
}

fn top_level_source_refs(json: &JsonValue, source_id: &str, sections: &[String]) -> Vec<SourceRef> {
    sections
        .iter()
        .filter_map(|section| {
            json.get(section).and_then(|value| {
                serde_json::to_string(value).ok().map(|raw| SourceRef {
                    source_id: source_id.to_string(),
                    field_path: format!(":source/raw {}", section),
                    raw: Some(raw),
                })
            })
        })
        .collect()
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

fn sanitize_id_or(value: &str, fallback: &str) -> String {
    let sanitized = sanitize_id(value);
    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
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

    const VIAL_MATRIX_ORDER_FIXTURE: &str = r#"
    {
      "uid": "Matrix-Order",
      "layouts": {
        "keymap": [
          ["0,1\nRight", "0,0\nLeft"]
        ]
      },
      "layout": [
        [
          ["KC_A", "KC_B"]
        ]
      ]
    }
    "#;

    const NOCFREE_BACKUP_FIXTURE: &str = r#"
    {
      "version": 1,
      "uid": 123456,
      "layout": [
        [
          ["KC_ESC", "KC_Q", "KC_W"],
          ["KC_TAB", "KC_A", "KC_S"]
        ],
        [
          ["KC_TRNS", "MO(2)", "KC_NO"],
          ["KC_LSFT", "MACRO00", "KC_MS_UP"]
        ]
      ],
      "encoder_layout": [[], []],
      "layout_options": 0,
      "macro": [["KC_H", "KC_I"]],
      "vial_protocol": 6,
      "via_protocol": 11,
      "tap_dance": [["KC_A", "KC_B"]],
      "combo": [["KC_Q", "KC_W", "KC_ESC"]],
      "key_override": [
        {
          "trigger": "KC_A",
          "trigger_mods": 0,
          "layers": 0,
          "negative_mod_mask": 0,
          "suppressed_mods": 0,
          "replacement": "KC_B",
          "options": 0
        }
      ],
      "alt_repeat_key": [],
      "settings": {"1": 0}
    }
    "#;

    const OVERKEYS_COMPANION_FIXTURE: &str = r#"
    {
      "defaultUserLayout": "Colemak Example",
      "kanataHost": "127.0.0.1",
      "kanataPort": 4039,
      "aliases": {
        "spc": "Space"
      },
      "customAliases": {
        "UNDO": ["Control", "Z"]
      },
      "customShiftMappings": {
        "openBracket": "{"
      },
      "triggers": {
        "nav": "caps"
      },
      "styles": {
        "theme": "nord"
      },
      "userLayouts": [
        {
          "name": "nav",
          "trigger": "F14",
          "type": "held",
          "keys": [
            ["spc", "Up", "UNDO"],
            ["Left", "Down", "openBracket"]
          ]
        },
        {
          "name": "Colemak Example",
          "keys": [
            ["spc", "W", "UNDO"],
            ["A", "R", "openBracket"]
          ]
        }
      ]
    }
    "#;

    const ZMK_KEYMAP_FIXTURE: &str = r#"
    / {
      keymap {
        compatible = "zmk,keymap";

        default_layer {
          bindings = <
            &kp Q &kp W &lt 1 SPACE
            &kp A &mo 1 &kp S
          >;
        };

        nav_layer {
          bindings = <
            &trans &kp UP &tog 2
            &kp LEFT &none &kp RIGHT
          >;
        };
      };
    };
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
        assert_eq!(
            candidate.preview_profile.keyboard_id,
            format!("keyboard-{}", candidate.source.id)
        );
        assert!(candidate.preview_profile.physical_layout.fallback);
        assert_eq!(candidate.summary.imported_keys, 4);
        assert_eq!(candidate.summary.imported_layers, 2);
        assert!(candidate
            .preview_profile
            .source_precedence
            .iter()
            .any(|rule| {
                rule.field_scope == ":keyboard/keymap"
                    && rule.source_order[0] == "user-overrides"
                    && rule.source_order[1] == candidate.source.id
            }));
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
    fn vial_import_maps_layer_cells_by_matrix_position_when_geometry_order_differs() {
        let candidate = import_vial_json(VIAL_MATRIX_ORDER_FIXTURE).expect("fixture imports");
        let layer = &candidate.preview_profile.keymap.layers[0];

        let right = layer
            .actions
            .iter()
            .find(|action| action.key_id == "vial-r0c1")
            .expect("right matrix key imported");
        let left = layer
            .actions
            .iter()
            .find(|action| action.key_id == "vial-r0c0")
            .expect("left matrix key imported");

        assert_eq!(right.raw.value, "KC_B");
        assert_eq!(left.raw.value, "KC_A");
    }

    #[test]
    fn vial_device_snapshot_imports_raw_matrices_by_matrix_position() {
        let candidate = import_vial_device_snapshot(VialDeviceSnapshot {
            vid: 0xfeed,
            pid: 0xcafe,
            uid: "Device UID".to_string(),
            protocol_version: 6,
            definition_json: serde_json::json!({
                "layouts": {
                    "keymap": [
                        ["0,1\nRight", "0,0\nLeft"]
                    ]
                }
            }),
            raw_matrices: vec![vec![vec![0x0004, 0x0005]], vec![vec![0x0001, 0x0000]]],
        })
        .expect("device snapshot imports");
        let base = &candidate.preview_profile.keymap.layers[0];
        let nav = &candidate.preview_profile.keymap.layers[1];
        let right = base
            .actions
            .iter()
            .find(|action| action.key_id == "vial-r0c1")
            .expect("right matrix key imported");
        let left = base
            .actions
            .iter()
            .find(|action| action.key_id == "vial-r0c0")
            .expect("left matrix key imported");
        let transparent_left = nav
            .actions
            .iter()
            .find(|action| action.key_id == "vial-r0c0")
            .expect("left matrix key imported on layer 1");

        assert_eq!(candidate.source.kind, "vial-device-import");
        assert_eq!(
            candidate.source.authority,
            SourceAuthority::BestEffortPreview
        );
        assert!(candidate.best_effort_preview);
        assert!(!candidate.preview_profile.physical_layout.fallback);
        assert_eq!(candidate.summary.imported_keys, 2);
        assert_eq!(candidate.summary.imported_layers, 2);
        assert_eq!(right.raw.value, "KC_B");
        assert_eq!(left.raw.value, "KC_A");
        assert_eq!(transparent_left.raw.value, "KC_TRNS");
        assert!(candidate
            .summary
            .preserved_sections
            .contains(&"vial-device-definition".to_string()));
        assert!(candidate
            .preview_profile
            .source_provenance
            .iter()
            .any(|source_ref| {
                source_ref.field_path == ":source/raw vial-device-protocol"
                    && source_ref
                        .raw
                        .as_deref()
                        .is_some_and(|raw| raw.contains("vial_protocol=6"))
            }));
    }

    #[test]
    fn nocfree_backup_layout_matrix_imports_layers_and_fallback_geometry() {
        let candidate = import_vial_json(NOCFREE_BACKUP_FIXTURE).expect("fixture imports");

        assert_eq!(candidate.source.id, "vial-file-123456");
        assert!(candidate.preview_profile.physical_layout.fallback);
        assert_eq!(candidate.summary.imported_keys, 6);
        assert_eq!(candidate.summary.imported_layers, 2);
        assert_eq!(
            candidate.preview_profile.physical_layout.keys[0].id,
            "vial-r0c0"
        );
        assert_eq!(
            candidate.preview_profile.physical_layout.keys[5].matrix,
            Some(MatrixPosition { row: 1, col: 2 })
        );
        assert_eq!(
            candidate.preview_profile.physical_layout.keys[5].geometry.x,
            2.0
        );
        assert_eq!(
            candidate.preview_profile.keymap.layers[1].actions[1]
                .raw
                .value,
            "MO(2)"
        );
        assert_eq!(
            candidate.preview_profile.keymap.layers[1].actions[1]
                .semantic
                .kind,
            crate::domain::SemanticActionKind::LayerMomentary
        );
    }

    #[test]
    fn nocfree_backup_preserves_top_level_sections_as_source_provenance() {
        let candidate = import_vial_json(NOCFREE_BACKUP_FIXTURE).expect("fixture imports");

        for section in [
            "combo",
            "encoder_layout",
            "key_override",
            "layout",
            "macro",
            "settings",
            "tap_dance",
            "uid",
            "version",
            "via_protocol",
            "vial_protocol",
        ] {
            assert!(candidate
                .summary
                .preserved_sections
                .contains(&section.to_string()));
            assert!(candidate
                .preview_profile
                .source_provenance
                .iter()
                .any(|source_ref| {
                    source_ref.field_path == format!(":source/raw {}", section)
                        && source_ref.raw.is_some()
                }));
        }
    }

    #[test]
    #[ignore = "requires KEYPLANE_LOCAL_VIL_CANDIDATE to point at a private local .vil export"]
    fn local_vil_candidate_file_imports_when_env_is_set() {
        let path = std::env::var("KEYPLANE_LOCAL_VIL_CANDIDATE")
            .expect("KEYPLANE_LOCAL_VIL_CANDIDATE is set");
        let contents = std::fs::read_to_string(path).expect("local candidate file is readable");
        let candidate = import_vial_json(&contents).expect("local candidate imports");

        assert!(candidate.best_effort_preview);
        assert!(candidate.summary.imported_keys > 0);
        assert!(candidate.summary.imported_layers > 0);
    }

    #[test]
    fn overkeys_companion_imports_row_arrays_as_fallback_layout() {
        let candidate =
            import_overkeys_companion_json(OVERKEYS_COMPANION_FIXTURE).expect("fixture imports");

        assert_eq!(candidate.source.id, "overkeys-companion-colemak-example");
        assert_eq!(candidate.source.kind, "overkeys-companion-import");
        assert!(candidate.best_effort_preview);
        assert!(candidate.preview_profile.physical_layout.fallback);
        assert!(candidate
            .preview_profile
            .sources
            .iter()
            .any(|source| source.id == kanata_backend::KANATA_BACKEND_ID
                && source.kind == "kanata"
                && source.authority == SourceAuthority::Authoritative));
        assert!(candidate
            .preview_profile
            .sources
            .iter()
            .any(|source| source.id == sentinel_backend::SENTINEL_BACKEND_ID
                && source.kind == "sentinel-keys"
                && source.authority == SourceAuthority::Inferred));
        assert_eq!(candidate.summary.imported_keys, 6);
        assert_eq!(candidate.summary.imported_layers, 2);
        assert_eq!(
            candidate.preview_profile.physical_layout.keys[0].id,
            "overkeys-r0c0"
        );
        assert_eq!(
            candidate.preview_profile.physical_layout.keys[5].matrix,
            Some(MatrixPosition { row: 1, col: 2 })
        );
        assert_eq!(
            candidate.preview_profile.keymap.layers[0].name,
            "Colemak Example"
        );
        assert_eq!(candidate.preview_profile.keymap.layers[1].name, "nav");
        assert_eq!(
            candidate.preview_profile.keymap.layers[0].actions[0]
                .raw
                .dialect,
            "overkeys"
        );
        assert_eq!(
            candidate.preview_profile.keymap.layers[0].actions[0]
                .raw
                .value,
            "spc"
        );
        assert!(candidate.preview_profile.keymap.layers[0].actions[0]
            .legend
            .slots
            .iter()
            .any(|slot| slot.slot == LegendSlotKind::Primary && slot.text == "Space"));
        assert_eq!(
            candidate.preview_profile.keymap.layers[0].actions[0]
                .semantic
                .label,
            "Space"
        );
        let custom_alias_action = candidate.preview_profile.keymap.layers[0]
            .actions
            .iter()
            .find(|action| action.raw.value == "UNDO")
            .expect("custom alias action imported");
        assert!(custom_alias_action
            .legend
            .slots
            .iter()
            .any(|slot| slot.slot == LegendSlotKind::ActionHint && slot.text == "Control + Z"));
        let custom_shift_action = candidate.preview_profile.keymap.layers[0]
            .actions
            .iter()
            .find(|action| action.raw.value == "openBracket")
            .expect("custom shift mapping action imported");
        assert!(custom_shift_action
            .legend
            .slots
            .iter()
            .any(|slot| slot.slot == LegendSlotKind::Shifted && slot.text == "{"));
        assert!(candidate
            .preview_profile
            .source_precedence
            .iter()
            .any(|rule| {
                rule.field_scope == ":runtime/state"
                    && rule.source_order[0] == kanata_backend::KANATA_BACKEND_ID
                    && rule.source_order[1] == sentinel_backend::SENTINEL_BACKEND_ID
                    && rule.source_order[2] == candidate.source.id
            }));
        assert!(candidate
            .preview_profile
            .source_precedence
            .iter()
            .any(|rule| {
                rule.field_scope == ":keyboard/keymap"
                    && rule.source_order[0] == "user-overrides"
                    && rule.source_order[1] == candidate.source.id
            }));
    }

    #[test]
    fn overkeys_companion_pairs_renderable_profile_with_kanata_runtime_backend() {
        let candidate =
            import_overkeys_companion_json(OVERKEYS_COMPANION_FIXTURE).expect("fixture imports");
        let profile = &candidate.preview_profile;

        let import_backend = profile
            .runtime_backends
            .iter()
            .find(|backend| backend.id == candidate.source.id)
            .expect("OverKeys import backend exists");
        assert_eq!(import_backend.health.state, HealthState::Stale);
        assert_eq!(
            import_backend.capabilities,
            vec![
                CapabilityFlag::ImportGeometry,
                CapabilityFlag::ImportKeymaps,
                CapabilityFlag::PreviewOnly,
            ]
        );

        let kanata_status = profile
            .runtime_backends
            .iter()
            .find(|backend| backend.id == kanata_backend::KANATA_BACKEND_ID)
            .expect("Kanata runtime backend exists");
        assert_eq!(kanata_status.health.state, HealthState::Disconnected);
        assert!(kanata_status
            .capabilities
            .contains(&CapabilityFlag::StreamLayerStack));
        assert!(kanata_status
            .capabilities
            .contains(&CapabilityFlag::PollState));

        let sentinel_status = profile
            .runtime_backends
            .iter()
            .find(|backend| backend.id == sentinel_backend::SENTINEL_BACKEND_ID)
            .expect("Sentinel Keys runtime backend exists");
        assert_eq!(sentinel_status.health.state, HealthState::PermissionMissing);
        assert!(sentinel_status
            .capabilities
            .contains(&CapabilityFlag::StreamLayerStack));
    }

    #[test]
    fn overkeys_companion_imports_layer_triggers_as_sentinel_key_bindings() {
        let candidate =
            import_overkeys_companion_json(OVERKEYS_COMPANION_FIXTURE).expect("fixture imports");
        let profile = &candidate.preview_profile;

        assert_eq!(
            profile.sentinel_keys,
            vec![SentinelKeyBinding {
                host_input_code: "F14".to_string(),
                layer_id: "layer-1".to_string(),
                activation: ActivationKind::Momentary,
            }]
        );
    }

    #[test]
    fn overkeys_companion_imports_legacy_trigger_map_as_sentinel_key_bindings() {
        let candidate = import_overkeys_companion_json(
            r#"
            {
              "defaultUserLayout": "base",
              "triggers": {
                "nav": "caps"
              },
              "userLayouts": [
                {
                  "name": "base",
                  "keys": [["A"]]
                },
                {
                  "name": "nav",
                  "keys": [["B"]]
                }
              ]
            }
            "#,
        )
        .expect("fixture imports");

        assert_eq!(
            candidate.preview_profile.sentinel_keys,
            vec![SentinelKeyBinding {
                host_input_code: "caps".to_string(),
                layer_id: "layer-1".to_string(),
                activation: ActivationKind::Momentary,
            }]
        );
    }

    #[test]
    fn overkeys_companion_preserves_aliases_triggers_styles_and_kanata_settings() {
        let candidate =
            import_overkeys_companion_json(OVERKEYS_COMPANION_FIXTURE).expect("fixture imports");

        for section in [
            "aliases",
            "customAliases",
            "customShiftMappings",
            "defaultUserLayout",
            "kanataHost",
            "kanataPort",
            "styles",
            "triggers",
            "userLayouts",
        ] {
            assert!(candidate
                .summary
                .preserved_sections
                .contains(&section.to_string()));
            assert!(candidate
                .preview_profile
                .source_provenance
                .iter()
                .any(|source_ref| {
                    source_ref.field_path == format!(":source/raw {}", section)
                        && source_ref.raw.is_some()
                }));
        }

        let kanata_config = candidate
            .preview_profile
            .runtime_backends
            .iter()
            .find(|backend| backend.id == kanata_backend::KANATA_BACKEND_ID)
            .and_then(|backend| backend.config.clone());
        assert_eq!(
            kanata_config,
            Some(BackendConfig::KanataTcp {
                host: "127.0.0.1".to_string(),
                port: 4039,
            })
        );
    }

    #[test]
    fn zmk_keymap_imports_binding_rows_as_best_effort_preview() {
        let candidate = import_zmk_keymap(ZMK_KEYMAP_FIXTURE).expect("fixture imports");

        assert_eq!(candidate.source.kind, "zmk-keymap-import");
        assert!(candidate.best_effort_preview);
        assert!(candidate.preview_profile.physical_layout.fallback);
        assert_eq!(candidate.summary.imported_keys, 6);
        assert_eq!(candidate.summary.imported_layers, 2);
        assert_eq!(
            candidate.preview_profile.physical_layout.keys[0].id,
            "zmk-r0c0"
        );
        assert_eq!(
            candidate.preview_profile.physical_layout.keys[5].matrix,
            Some(MatrixPosition { row: 1, col: 2 })
        );
        assert_eq!(
            candidate.preview_profile.keymap.layers[0].name,
            "default_layer"
        );
        assert_eq!(
            candidate.preview_profile.keymap.layers[0].actions[2]
                .raw
                .value,
            "&lt 1 SPACE"
        );
        assert_eq!(
            candidate.preview_profile.keymap.layers[0].actions[2]
                .semantic
                .kind,
            crate::domain::SemanticActionKind::LayerTap
        );
        assert_eq!(
            candidate.preview_profile.keymap.layers[1].actions[2]
                .semantic
                .target_layer
                .as_deref(),
            Some("layer-2")
        );
        assert!(candidate
            .preview_profile
            .source_precedence
            .iter()
            .any(|rule| {
                rule.field_scope == ":keyboard/keymap"
                    && rule.source_order[0] == "user-overrides"
                    && rule.source_order[1] == candidate.source.id
            }));
    }

    #[test]
    fn zmk_keymap_preserves_raw_source_text() {
        let candidate = import_zmk_keymap(ZMK_KEYMAP_FIXTURE).expect("fixture imports");

        assert_eq!(
            candidate.summary.preserved_sections,
            vec!["zmk-keymap".to_string()]
        );
        assert!(candidate
            .preview_profile
            .source_provenance
            .iter()
            .any(|source_ref| {
                source_ref.field_path == ":source/raw zmk-keymap"
                    && source_ref
                        .raw
                        .as_deref()
                        .is_some_and(|raw| raw.contains("zmk,keymap"))
            }));
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
            candidate.preview_profile.visual_style.id,
            "style-keyviz-lowprofile"
        );
        assert_eq!(
            candidate.preview_profile.visual_style.variant_id,
            "keyviz-lowprofile"
        );
        assert_eq!(
            candidate.preview_profile.visual_style.density,
            StyleDensity::Rich
        );
        assert_eq!(
            candidate
                .preview_profile
                .visual_style
                .colors
                .keycap_background
                .as_deref(),
            Some("#ffffff")
        );
        assert_eq!(
            candidate
                .preview_profile
                .visual_style
                .colors
                .keycap_text
                .as_deref(),
            Some("#000000")
        );
        assert_eq!(
            candidate
                .preview_profile
                .visual_style
                .colors
                .keycap_border
                .as_deref(),
            Some("#1a1a1a")
        );
        assert_eq!(
            candidate
                .preview_profile
                .visual_style
                .colors
                .modifier_accent
                .as_deref(),
            Some("#3a86ff")
        );
        assert_eq!(
            candidate
                .preview_profile
                .visual_style
                .colors
                .overlay_background
                .as_deref(),
            Some("#ffffff99")
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

        assert_eq!(candidate.conflicts.len(), 7);
        let variant_conflict = candidate
            .conflicts
            .iter()
            .find(|conflict| conflict.field_path == ":visual/style :style/variant-id")
            .expect("variant conflict exists");
        assert_eq!(
            variant_conflict.field_path,
            ":visual/style :style/variant-id"
        );
        assert_eq!(
            variant_conflict.selected_source_id,
            "keyviz-style-lowprofile"
        );
        assert!(candidate
            .preview_profile
            .source_precedence
            .iter()
            .any(|rule| {
                rule.field_scope == ":visual/style"
                    && rule.source_order[0] == "user-overrides"
                    && rule.source_order[1] == "keyviz-style-lowprofile"
                    && rule.source_order[2] == "fake-backend"
            }));
        assert!(variant_conflict.candidates.iter().any(|candidate| {
            candidate.source_id == "fake-backend"
                && candidate.value == "keyplane-default"
                && !candidate.selected
        }));
        assert!(variant_conflict.candidates.iter().any(|candidate| {
            candidate.source_id == "keyviz-style-lowprofile"
                && candidate.value == "keyviz-lowprofile"
                && candidate.selected
        }));
        assert!(candidate.conflicts.iter().any(|conflict| {
            conflict.field_path == ":visual/style :style/colors :color/keycap-background"
                && conflict.candidates.iter().any(|candidate| {
                    candidate.source_id == "fake-backend"
                        && candidate.value == "nil"
                        && !candidate.selected
                })
                && conflict.candidates.iter().any(|candidate| {
                    candidate.source_id == "keyviz-style-lowprofile"
                        && candidate.value == "#ffffff"
                        && candidate.selected
                })
        }));
    }

    #[test]
    fn keyviz_style_import_exposes_visual_density_conflicts() {
        let base_profile = crate::fake_backend::fake_profile();
        let minimal_fixture =
            KEYVIZ_STYLE_FIXTURE.replace("\"style\": \"lowprofile\"", "\"style\": \"minimal\"");
        let candidate =
            import_keyviz_style_json(&minimal_fixture, &base_profile).expect("style imports");

        let density_conflict = candidate
            .conflicts
            .iter()
            .find(|conflict| conflict.field_path == ":visual/style :style/density")
            .expect("density conflict exists");
        assert!(density_conflict.candidates.iter().any(|candidate| {
            candidate.source_id == "fake-backend"
                && candidate.value == "rich"
                && !candidate.selected
        }));
        assert!(density_conflict.candidates.iter().any(|candidate| {
            candidate.source_id == "keyviz-style-minimal"
                && candidate.value == "compact"
                && candidate.selected
        }));
    }
}
