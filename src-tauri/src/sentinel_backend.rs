use crate::domain::{
    ActivationKind, BackendHealth, BackendStatus, CapabilityFlag, HealthState, HostInputEvent,
    LayerActivation, RuntimeEvent, SentinelKeyBinding, StateConfidence, StateConfidenceLevel,
};

pub const SENTINEL_BACKEND_ID: &str = "sentinel-keys";

pub fn sentinel_backend_status(health: HealthState, message: impl Into<String>) -> BackendStatus {
    BackendStatus {
        id: SENTINEL_BACKEND_ID.to_string(),
        name: "Sentinel Keys".to_string(),
        capabilities: vec![CapabilityFlag::StreamLayerStack],
        health: BackendHealth {
            backend_id: SENTINEL_BACKEND_ID.to_string(),
            state: health,
            message: message.into(),
        },
        config: None,
    }
}

pub fn runtime_event_from_host_input_event(
    active_layers: &mut Vec<String>,
    bindings: &[SentinelKeyBinding],
    base_layer_id: &str,
    event: &HostInputEvent,
) -> Option<RuntimeEvent> {
    let binding = bindings
        .iter()
        .find(|binding| binding.host_input_code == event.code)?;

    match binding.activation {
        ActivationKind::Toggle | ActivationKind::Lock if !event.pressed => return None,
        ActivationKind::Toggle | ActivationKind::Lock => {
            if active_layers.contains(&binding.layer_id) {
                remove_active_layer(active_layers, &binding.layer_id);
            } else {
                activate_layer(active_layers, &binding.layer_id);
            }
        }
        _ if event.pressed => activate_layer(active_layers, &binding.layer_id),
        _ => remove_active_layer(active_layers, &binding.layer_id),
    }

    Some(RuntimeEvent::LayerStackChanged {
        layer_stack: sentinel_layer_stack(active_layers, bindings, base_layer_id),
        source_id: Some(SENTINEL_BACKEND_ID.to_string()),
    })
}

fn activate_layer(active_layers: &mut Vec<String>, layer_id: &str) {
    remove_active_layer(active_layers, layer_id);
    active_layers.insert(0, layer_id.to_string());
}

fn remove_active_layer(active_layers: &mut Vec<String>, layer_id: &str) {
    active_layers.retain(|active| active != layer_id);
}

fn sentinel_layer_stack(
    active_layers: &[String],
    bindings: &[SentinelKeyBinding],
    base_layer_id: &str,
) -> Vec<LayerActivation> {
    let mut stack: Vec<LayerActivation> = active_layers
        .iter()
        .map(|layer_id| LayerActivation {
            layer_id: layer_id.clone(),
            kind: bindings
                .iter()
                .find(|binding| binding.layer_id == *layer_id)
                .map(|binding| binding.activation.clone())
                .unwrap_or(ActivationKind::Unknown),
            confidence: sentinel_confidence(),
        })
        .collect();

    stack.push(LayerActivation {
        layer_id: base_layer_id.to_string(),
        kind: ActivationKind::Default,
        confidence: sentinel_confidence(),
    });
    stack
}

fn sentinel_confidence() -> StateConfidence {
    StateConfidence {
        level: StateConfidenceLevel::Low,
        reason: "Sentinel Key inferred from Host Input Event; startup state may be stale"
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(activation: ActivationKind) -> SentinelKeyBinding {
        SentinelKeyBinding {
            host_input_code: "F24".to_string(),
            layer_id: "layer-1".to_string(),
            activation,
        }
    }

    #[test]
    fn sentinel_backend_status_reports_lower_authority_runtime_capability() {
        let status = sentinel_backend_status(
            HealthState::PermissionMissing,
            "Input monitoring unavailable",
        );

        assert_eq!(status.id, SENTINEL_BACKEND_ID);
        assert!(status
            .capabilities
            .contains(&CapabilityFlag::StreamLayerStack));
        assert_eq!(status.health.state, HealthState::PermissionMissing);
    }

    #[test]
    fn momentary_sentinel_key_maps_host_input_press_and_release_to_layer_stack() {
        let mut active_layers = Vec::new();
        let bindings = vec![binding(ActivationKind::Momentary)];

        let pressed = runtime_event_from_host_input_event(
            &mut active_layers,
            &bindings,
            "layer-0",
            &HostInputEvent {
                code: "F24".to_string(),
                pressed: true,
            },
        )
        .expect("sentinel event");

        match pressed {
            RuntimeEvent::LayerStackChanged {
                layer_stack,
                source_id,
            } => {
                assert_eq!(source_id.as_deref(), Some(SENTINEL_BACKEND_ID));
                assert_eq!(layer_stack[0].layer_id, "layer-1");
                assert_eq!(layer_stack[0].kind, ActivationKind::Momentary);
                assert_eq!(layer_stack[0].confidence.level, StateConfidenceLevel::Low);
                assert_eq!(layer_stack[1].layer_id, "layer-0");
            }
            _ => panic!("expected layer stack event"),
        }

        let released = runtime_event_from_host_input_event(
            &mut active_layers,
            &bindings,
            "layer-0",
            &HostInputEvent {
                code: "F24".to_string(),
                pressed: false,
            },
        )
        .expect("sentinel release event");

        match released {
            RuntimeEvent::LayerStackChanged { layer_stack, .. } => {
                assert_eq!(layer_stack.len(), 1);
                assert_eq!(layer_stack[0].layer_id, "layer-0");
            }
            _ => panic!("expected layer stack event"),
        }
    }

    #[test]
    fn toggle_sentinel_key_ignores_release_events() {
        let mut active_layers = Vec::new();
        let bindings = vec![binding(ActivationKind::Toggle)];

        assert!(runtime_event_from_host_input_event(
            &mut active_layers,
            &bindings,
            "layer-0",
            &HostInputEvent {
                code: "F24".to_string(),
                pressed: true,
            },
        )
        .is_some());
        assert_eq!(active_layers, vec!["layer-1".to_string()]);

        assert!(runtime_event_from_host_input_event(
            &mut active_layers,
            &bindings,
            "layer-0",
            &HostInputEvent {
                code: "F24".to_string(),
                pressed: false,
            },
        )
        .is_none());
        assert_eq!(active_layers, vec!["layer-1".to_string()]);
    }

    #[test]
    fn unrelated_host_input_events_do_not_change_sentinel_state() {
        let mut active_layers = Vec::new();
        let bindings = vec![binding(ActivationKind::Momentary)];

        assert!(runtime_event_from_host_input_event(
            &mut active_layers,
            &bindings,
            "layer-0",
            &HostInputEvent {
                code: "Escape".to_string(),
                pressed: true,
            },
        )
        .is_none());
        assert!(active_layers.is_empty());
    }
}
