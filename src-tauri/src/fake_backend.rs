use crate::domain::{
    apply_runtime_event, compose_snapshot, derive_action, global_display_fallback, ActivationKind,
    BackendHealth, BackendStatus, CapabilityFlag, HealthState, ImportCandidate, ImportSummary,
    KeyGeometry, KeyboardSnapshot, Layer, LayerActivation, LogicalKeymap, OverlayWindowConfig,
    PhysicalKey, PhysicalLayout, Profile, RuntimeEvent, RuntimeState, Source, SourceAuthority,
    SourceConflict, SourcePrecedenceRule, SourceRef, StateConfidence, StateConfidenceLevel,
    StyleDensity, UserOverride, VisibilityPolicy, VisualStyle, VisualStyleColors,
};
use crate::kanata_backend;
use crate::keypeek_backend;
use crate::overlay_backend;
use crate::sentinel_backend;

const FAKE_SOURCE_ID: &str = "fake-backend";
const PROFILE_ID: &str = "profile-keyplane-demo";

pub fn fake_profile() -> Profile {
    let fake_source = Source {
        id: FAKE_SOURCE_ID.to_string(),
        name: "Fake Backend".to_string(),
        kind: "fake".to_string(),
        authority: SourceAuthority::Authoritative,
    };
    let keypeek_source = Source {
        id: "keypeek-live".to_string(),
        name: "KeyPeek Live".to_string(),
        kind: "keypeek-firmware".to_string(),
        authority: SourceAuthority::Authoritative,
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
    let keys = fake_keys();
    let base_actions = [
        ("k-esc", "KC_ESC"),
        ("k-q", "KC_Q"),
        ("k-w", "KC_W"),
        ("k-e", "KC_E"),
        ("k-a", "KC_A"),
        ("k-s", "KC_S"),
        ("k-d", "KC_D"),
        ("k-f", "KC_F"),
        ("k-shift", "KC_LSFT"),
        ("k-z", "KC_Z"),
        ("k-space", "LT(1, KC_SPC)"),
        ("k-fn", "MO(1)"),
    ];
    let nav_actions = [
        ("k-esc", "KC_GRV"),
        ("k-q", "KC_TRNS"),
        ("k-w", "KC_UP"),
        ("k-e", "KC_TRNS"),
        ("k-a", "KC_LEFT"),
        ("k-s", "KC_DOWN"),
        ("k-d", "KC_RGHT"),
        ("k-f", "KC_TRNS"),
        ("k-shift", "KC_TRNS"),
        ("k-z", "KC_TRNS"),
        ("k-space", "KC_BSPC"),
        ("k-fn", "KC_TRNS"),
    ];

    let source_ref = |field_path: &str, raw: &str| SourceRef {
        source_id: FAKE_SOURCE_ID.to_string(),
        field_path: field_path.to_string(),
        raw: Some(raw.to_string()),
    };

    let layers = vec![
        Layer {
            id: "layer-0".to_string(),
            name: "Base".to_string(),
            actions: base_actions
                .iter()
                .map(|(key_id, raw)| {
                    derive_action(
                        "qmk",
                        raw,
                        source_ref(&format!(":keyboard/keymap :layer-0 {}", key_id), raw),
                        key_id,
                    )
                })
                .collect(),
        },
        Layer {
            id: "layer-1".to_string(),
            name: "Navigation".to_string(),
            actions: nav_actions
                .iter()
                .map(|(key_id, raw)| {
                    derive_action(
                        "qmk",
                        raw,
                        source_ref(&format!(":keyboard/keymap :layer-1 {}", key_id), raw),
                        key_id,
                    )
                })
                .collect(),
        },
    ];

    let health = ok_backend_health();

    Profile {
        schema_version: 1,
        id: PROFILE_ID.to_string(),
        name: "Keyplane Demo".to_string(),
        sources: vec![fake_source, keypeek_source, kanata_source, sentinel_source],
        physical_layout: PhysicalLayout {
            keys: keys.clone(),
            fallback: false,
        },
        keymap: LogicalKeymap { layers },
        runtime_backends: vec![
            BackendStatus {
                id: FAKE_SOURCE_ID.to_string(),
                name: "Fake Backend".to_string(),
                capabilities: vec![
                    CapabilityFlag::ImportGeometry,
                    CapabilityFlag::ImportKeymaps,
                    CapabilityFlag::StreamLayerStack,
                    CapabilityFlag::StreamPressedKeys,
                ],
                health,
            },
            overlay_backend::overlay_window_backend_status(
                HealthState::Ok,
                "Overlay Window is ready",
            ),
            keypeek_backend::keypeek_backend_status(
                HealthState::Disconnected,
                "No KeyPeek-compatible device is connected",
            ),
            kanata_backend::kanata_backend_status(
                HealthState::Disconnected,
                "Kanata TCP runtime is not connected",
            ),
            sentinel_backend::sentinel_backend_status(
                HealthState::PermissionMissing,
                "Input monitoring permission is required before Sentinel Keys can infer layers",
            ),
        ],
        sentinel_keys: vec![crate::domain::SentinelKeyBinding {
            host_input_code: "F24".to_string(),
            layer_id: "layer-1".to_string(),
            activation: ActivationKind::Momentary,
        }],
        visual_style: VisualStyle {
            variant_id: "keyplane-default".to_string(),
            density: StyleDensity::Rich,
            colors: VisualStyleColors::default(),
        },
        overlay_window: OverlayWindowConfig {
            visibility: VisibilityPolicy::Pinned,
            visible: true,
            click_through: true,
            positioning_mode: false,
            display_targeting: global_display_fallback(),
        },
        source_precedence: vec![
            SourcePrecedenceRule {
                field_scope: ":runtime/state".to_string(),
                source_order: vec![
                    "keypeek-live".to_string(),
                    kanata_backend::KANATA_BACKEND_ID.to_string(),
                    FAKE_SOURCE_ID.to_string(),
                    sentinel_backend::SENTINEL_BACKEND_ID.to_string(),
                ],
            },
            SourcePrecedenceRule {
                field_scope: ":keyboard/physical-layout".to_string(),
                source_order: vec![
                    "user-overrides".to_string(),
                    FAKE_SOURCE_ID.to_string(),
                    "vial-import".to_string(),
                ],
            },
        ],
        user_overrides: Vec::<UserOverride>::new(),
        source_provenance: keys.into_iter().map(|key| key.provenance).collect(),
    }
}

pub fn initial_runtime_state(profile: &Profile) -> RuntimeState {
    RuntimeState {
        layer_stack: vec![default_layer_activation()],
        pressed_keys: Vec::new(),
        backend_health: profile
            .runtime_backends
            .iter()
            .map(|backend| backend.health.clone())
            .collect(),
    }
}

pub fn initial_snapshot() -> KeyboardSnapshot {
    let profile = fake_profile();
    compose_snapshot(
        &profile,
        initial_runtime_state(&profile),
        Vec::<SourceConflict>::new(),
    )
}

pub fn demo_runtime_events() -> Vec<RuntimeEvent> {
    vec![
        RuntimeEvent::PressedKeysChanged {
            pressed_keys: vec!["k-fn".to_string()],
        },
        RuntimeEvent::LayerStackChanged {
            layer_stack: vec![nav_layer_activation(), default_layer_activation()],
        },
        RuntimeEvent::BackendHealthChanged {
            health: permission_missing_backend_health(),
        },
        RuntimeEvent::PressedKeysChanged {
            pressed_keys: vec!["k-fn".to_string(), "k-w".to_string()],
        },
        RuntimeEvent::LayerStackChanged {
            layer_stack: vec![default_layer_activation()],
        },
        RuntimeEvent::BackendHealthChanged {
            health: ok_backend_health(),
        },
        RuntimeEvent::PressedKeysChanged {
            pressed_keys: Vec::new(),
        },
    ]
}

pub fn snapshot_after_demo_events() -> KeyboardSnapshot {
    let mut snapshot = initial_snapshot();
    for event in demo_runtime_events() {
        apply_runtime_event(&mut snapshot, event);
    }
    snapshot
}

pub fn import_candidate_from_profile(profile: Profile) -> ImportCandidate {
    ImportCandidate {
        id: "candidate-fake-profile".to_string(),
        source: profile.sources[0].clone(),
        best_effort_preview: false,
        summary: ImportSummary {
            imported_keys: profile.physical_layout.keys.len(),
            imported_layers: profile.keymap.layers.len(),
            preserved_sections: vec![
                ":keyboard/physical-layout".to_string(),
                ":keyboard/keymap".to_string(),
                ":runtime/backends".to_string(),
            ],
        },
        preview_profile: profile,
        conflicts: Vec::new(),
    }
}

fn fake_keys() -> Vec<PhysicalKey> {
    let rows = [
        [("k-esc", 0, 0), ("k-q", 0, 1), ("k-w", 0, 2), ("k-e", 0, 3)],
        [("k-a", 1, 0), ("k-s", 1, 1), ("k-d", 1, 2), ("k-f", 1, 3)],
        [
            ("k-shift", 2, 0),
            ("k-z", 2, 1),
            ("k-space", 2, 2),
            ("k-fn", 2, 3),
        ],
    ];

    rows.into_iter()
        .flat_map(|row| row.into_iter())
        .map(|(id, row, col)| PhysicalKey {
            id: id.to_string(),
            matrix: Some(crate::domain::MatrixPosition {
                row: row as u16,
                col: col as u16,
            }),
            geometry: KeyGeometry {
                x: col as f32 * 1.08 + if row == 1 { 0.28 } else { 0.0 },
                y: row as f32 * 1.05,
                width: if id == "k-space" { 1.7 } else { 1.0 },
                height: 1.0,
                rotation: if row == 2 && col >= 2 { -4.0 } else { 0.0 },
            },
            provenance: SourceRef {
                source_id: FAKE_SOURCE_ID.to_string(),
                field_path: format!(":keyboard/physical-layout {}", id),
                raw: Some(format!("matrix:{},{}", row, col)),
            },
        })
        .collect()
}

fn ok_backend_health() -> BackendHealth {
    BackendHealth {
        backend_id: FAKE_SOURCE_ID.to_string(),
        state: HealthState::Ok,
        message: "Streaming deterministic layer stack events".to_string(),
    }
}

fn permission_missing_backend_health() -> BackendHealth {
    BackendHealth {
        backend_id: FAKE_SOURCE_ID.to_string(),
        state: HealthState::PermissionMissing,
        message: "Input monitoring permission is missing for host input fallback".to_string(),
    }
}

fn default_layer_activation() -> LayerActivation {
    LayerActivation {
        layer_id: "layer-0".to_string(),
        kind: ActivationKind::Default,
        confidence: StateConfidence {
            level: StateConfidenceLevel::High,
            reason: "Default layer from fake backend".to_string(),
        },
    }
}

fn nav_layer_activation() -> LayerActivation {
    LayerActivation {
        layer_id: "layer-1".to_string(),
        kind: ActivationKind::Momentary,
        confidence: StateConfidence {
            level: StateConfidenceLevel::High,
            reason: "Momentary layer from fake backend Runtime Event".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_backend_snapshot_has_authoritative_health_and_effective_keys() {
        let snapshot = initial_snapshot();

        assert_eq!(
            snapshot.runtime_state.backend_health[0].state,
            HealthState::Ok
        );
        assert!(snapshot
            .backends
            .iter()
            .flat_map(|backend| backend.capabilities.iter())
            .any(|capability| capability == &CapabilityFlag::StreamLayerStack));
        assert_eq!(
            snapshot.effective_keys.len(),
            snapshot.physical_layout.keys.len()
        );
        assert!(snapshot.source_provenance.iter().any(|source_ref| {
            source_ref.field_path == ":keyboard/physical-layout k-q"
                && source_ref.raw.as_deref() == Some("matrix:0,1")
        }));
        assert!(snapshot.backends.iter().any(|backend| {
            backend.id == overlay_backend::OVERLAY_WINDOW_BACKEND_ID
                && backend
                    .capabilities
                    .contains(&CapabilityFlag::ClickThroughOverlayWindow)
                && backend.health.state == HealthState::Ok
        }));
        assert!(snapshot.backends.iter().any(|backend| {
            backend.id == "keypeek-live" && backend.health.state == HealthState::Disconnected
        }));
        assert!(snapshot.backends.iter().any(|backend| {
            backend.id == kanata_backend::KANATA_BACKEND_ID
                && backend.health.state == HealthState::Disconnected
        }));
        assert!(snapshot.backends.iter().any(|backend| {
            backend.id == sentinel_backend::SENTINEL_BACKEND_ID
                && backend.health.state == HealthState::PermissionMissing
        }));
        assert_eq!(snapshot.sentinel_keys[0].host_input_code, "F24");
        assert_eq!(snapshot.sentinel_keys[0].layer_id, "layer-1");
        assert!(snapshot
            .runtime_state
            .backend_health
            .iter()
            .any(|health| health.backend_id == "keypeek-live"
                && health.state == HealthState::Disconnected));
        assert!(snapshot.source_precedence.iter().any(|rule| {
            rule.field_scope == ":runtime/state"
                && rule.source_order[0] == "keypeek-live"
                && rule.source_order[1] == kanata_backend::KANATA_BACKEND_ID
                && rule.source_order[3] == sentinel_backend::SENTINEL_BACKEND_ID
        }));
    }

    #[test]
    fn fake_backend_layer_event_recomputes_effective_inherited_legends() {
        let mut snapshot = initial_snapshot();
        apply_runtime_event(
            &mut snapshot,
            RuntimeEvent::LayerStackChanged {
                layer_stack: vec![nav_layer_activation(), default_layer_activation()],
            },
        );

        let key_q = snapshot
            .effective_keys
            .iter()
            .find(|key| key.key_id == "k-q")
            .expect("q key exists");

        assert_eq!(key_q.semantic.label, "Q");
        assert_eq!(key_q.source_layer_id, "layer-0");
        assert!(key_q.inherited);
    }

    #[test]
    fn demo_runtime_events_include_permission_health_and_recovery() {
        let health_states: Vec<HealthState> = demo_runtime_events()
            .into_iter()
            .filter_map(|event| match event {
                RuntimeEvent::BackendHealthChanged { health } => Some(health.state),
                _ => None,
            })
            .collect();

        assert!(health_states.contains(&HealthState::PermissionMissing));
        assert!(health_states.contains(&HealthState::Ok));

        let snapshot = snapshot_after_demo_events();
        assert_eq!(
            snapshot.runtime_state.backend_health[0].state,
            HealthState::Ok
        );
        assert_eq!(snapshot.backends[0].health.state, HealthState::Ok);
    }
}
