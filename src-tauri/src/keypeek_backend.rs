use crate::domain::{
    ActivationKind, BackendHealth, BackendStatus, CapabilityFlag, HealthState, LayerActivation,
    RuntimeEvent, StateConfidence, StateConfidenceLevel,
};
pub use crate::keypeek_contract::{
    keypeek_subscribe_message, parse_keypeek_layer_packet, parse_keypeek_pressed_key_packet,
    KeyPeekLayerPacket, KeyPeekPressedKeyPacket, LAYER_STATE_PACKET_MARKER,
    PRESSED_KEY_PACKET_MARKER, SUBSCRIBE_ACTIVE, SUBSCRIBE_INACTIVE, SUBSCRIBE_MARKER,
};

pub const KEYPEEK_BACKEND_ID: &str = "keypeek-live";

pub fn keypeek_backend_status(health: HealthState, message: impl Into<String>) -> BackendStatus {
    BackendStatus {
        id: KEYPEEK_BACKEND_ID.to_string(),
        name: "KeyPeek Live".to_string(),
        capabilities: vec![
            CapabilityFlag::DiscoverDevices,
            CapabilityFlag::StreamLayerStack,
            CapabilityFlag::StreamPressedKeys,
        ],
        health: BackendHealth {
            backend_id: KEYPEEK_BACKEND_ID.to_string(),
            state: health,
            message: message.into(),
        },
        config: None,
    }
}

pub fn keypeek_discovery_backend_status(device_count: usize) -> BackendStatus {
    if device_count == 0 {
        return keypeek_backend_status(
            HealthState::Disconnected,
            "No KeyPeek-compatible VIA Raw HID devices discovered",
        );
    }

    keypeek_backend_status(
        HealthState::Ok,
        format!("Discovered {device_count} KeyPeek-compatible VIA Raw HID device(s)"),
    )
}

pub fn layer_stack_from_keypeek_masks(
    default_layer_state: u32,
    layer_state: u32,
    layer_count: usize,
) -> Vec<LayerActivation> {
    let layer_count = layer_count.min(32);
    let mut stack = Vec::new();

    for layer_index in (1..layer_count).rev() {
        let layer_mask = 1_u32 << layer_index;
        let momentary = (layer_state & layer_mask) != 0;
        let default = (default_layer_state & layer_mask) != 0;

        if momentary || default {
            stack.push(LayerActivation {
                layer_id: format!("layer-{}", layer_index),
                kind: if momentary {
                    ActivationKind::Momentary
                } else {
                    ActivationKind::Default
                },
                confidence: StateConfidence {
                    level: StateConfidenceLevel::High,
                    reason: "KeyPeek firmware-module layer packet".to_string(),
                },
            });
        }
    }

    stack.push(LayerActivation {
        layer_id: "layer-0".to_string(),
        kind: ActivationKind::Default,
        confidence: StateConfidence {
            level: StateConfidenceLevel::High,
            reason: "Base layer retained below KeyPeek active layers".to_string(),
        },
    });

    stack
}

pub fn runtime_event_from_keypeek_layer_packet(
    response: &[u8],
    layer_count: usize,
) -> Option<RuntimeEvent> {
    let packet = parse_keypeek_layer_packet(response)?;
    Some(RuntimeEvent::LayerStackChanged {
        layer_stack: layer_stack_from_keypeek_masks(
            packet.default_layer_state,
            packet.layer_state,
            layer_count,
        ),
        source_id: Some(KEYPEEK_BACKEND_ID.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keypeek_layer_packets_with_variable_state_size() {
        let packet = parse_keypeek_layer_packet(&[
            LAYER_STATE_PACKET_MARKER,
            2,
            0b0000_0001,
            0,
            0b0000_0100,
            0,
        ])
        .expect("valid layer packet");

        assert_eq!(
            packet,
            KeyPeekLayerPacket {
                default_layer_state: 1,
                layer_state: 4
            }
        );
    }

    #[test]
    fn rejects_non_layer_packets_and_echoed_subscribe_messages() {
        assert!(parse_keypeek_layer_packet(&[SUBSCRIBE_MARKER, SUBSCRIBE_ACTIVE]).is_none());
        assert!(parse_keypeek_layer_packet(&[LAYER_STATE_PACKET_MARKER, 9, 0, 0]).is_none());
    }

    #[test]
    fn converts_keypeek_layer_masks_to_top_first_layer_stack() {
        let stack = layer_stack_from_keypeek_masks(1, 0b0000_1010, 4);

        assert_eq!(stack[0].layer_id, "layer-3");
        assert_eq!(stack[1].layer_id, "layer-1");
        assert_eq!(stack[2].layer_id, "layer-0");
        assert_eq!(stack[0].kind, ActivationKind::Momentary);
    }

    #[test]
    fn parses_pressed_key_packets() {
        let pressed = parse_keypeek_pressed_key_packet(&[PRESSED_KEY_PACKET_MARKER, 2, 5, 1])
            .expect("valid pressed-key packet");

        assert_eq!(
            pressed,
            KeyPeekPressedKeyPacket {
                row: 2,
                col: 5,
                pressed: true
            }
        );
    }

    #[test]
    fn emits_keypeek_subscription_messages() {
        assert_eq!(
            keypeek_subscribe_message(true),
            vec![SUBSCRIBE_MARKER, SUBSCRIBE_ACTIVE]
        );
        assert_eq!(
            keypeek_subscribe_message(false),
            vec![SUBSCRIBE_MARKER, SUBSCRIBE_INACTIVE]
        );
    }

    #[test]
    fn discovery_status_reports_discovered_device_count() {
        let empty = keypeek_discovery_backend_status(0);
        assert_eq!(empty.health.state, HealthState::Disconnected);
        assert!(empty
            .health
            .message
            .contains("No KeyPeek-compatible VIA Raw HID devices"));

        let discovered = keypeek_discovery_backend_status(2);
        assert_eq!(discovered.health.state, HealthState::Ok);
        assert!(discovered.health.message.contains("Discovered 2"));
        assert!(discovered
            .capabilities
            .contains(&CapabilityFlag::DiscoverDevices));
    }

    #[test]
    fn live_backend_capabilities_only_advertise_discovery_and_runtime_streams() {
        let status = keypeek_backend_status(HealthState::Disconnected, "not connected");

        assert_eq!(
            status.capabilities,
            vec![
                CapabilityFlag::DiscoverDevices,
                CapabilityFlag::StreamLayerStack,
                CapabilityFlag::StreamPressedKeys,
            ]
        );
    }
}
