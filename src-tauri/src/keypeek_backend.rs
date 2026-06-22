use crate::domain::{
    ActivationKind, BackendHealth, BackendStatus, CapabilityFlag, HealthState, LayerActivation,
    RuntimeEvent, StateConfidence, StateConfidenceLevel,
};

/// KeyPeek firmware-module packets mark layer-state reports with `0xff`.
pub const LAYER_STATE_PACKET_MARKER: u8 = 0xff;
/// KeyPeek firmware-module packets mark physical key press reports with `0xf1`.
pub const PRESSED_KEY_PACKET_MARKER: u8 = 0xf1;
/// Raw HID keepalive marker used by the KeyPeek firmware module.
pub const SUBSCRIBE_MARKER: u8 = 0xc0;
pub const SUBSCRIBE_ACTIVE: u8 = 0xa1;
pub const SUBSCRIBE_INACTIVE: u8 = 0xa0;

const MAX_LAYER_STATE_BYTES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPeekLayerPacket {
    pub default_layer_state: u32,
    pub layer_state: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPeekPressedKeyPacket {
    pub row: u16,
    pub col: u16,
    pub pressed: bool,
}

pub fn keypeek_backend_status(health: HealthState, message: impl Into<String>) -> BackendStatus {
    BackendStatus {
        id: "keypeek-live".to_string(),
        name: "KeyPeek Live".to_string(),
        capabilities: vec![
            CapabilityFlag::DiscoverDevices,
            CapabilityFlag::ImportGeometry,
            CapabilityFlag::ImportKeymaps,
            CapabilityFlag::StreamLayerStack,
            CapabilityFlag::StreamPressedKeys,
        ],
        health: BackendHealth {
            backend_id: "keypeek-live".to_string(),
            state: health,
            message: message.into(),
        },
    }
}

pub fn keypeek_subscribe_message(active: bool) -> Vec<u8> {
    vec![
        SUBSCRIBE_MARKER,
        if active {
            SUBSCRIBE_ACTIVE
        } else {
            SUBSCRIBE_INACTIVE
        },
    ]
}

pub fn parse_keypeek_layer_packet(response: &[u8]) -> Option<KeyPeekLayerPacket> {
    if response.first().copied()? != LAYER_STATE_PACKET_MARKER {
        return None;
    }

    let size = *response.get(1)? as usize;
    if size == 0 || size > MAX_LAYER_STATE_BYTES || 2 + 2 * size > response.len() {
        return None;
    }

    let mut default_bytes = [0_u8; 4];
    default_bytes[..size].copy_from_slice(&response[2..2 + size]);
    let default_layer_state = u32::from_le_bytes(default_bytes);

    let mut layer_bytes = [0_u8; 4];
    layer_bytes[..size].copy_from_slice(&response[2 + size..2 + 2 * size]);
    let layer_state = u32::from_le_bytes(layer_bytes);

    Some(KeyPeekLayerPacket {
        default_layer_state,
        layer_state,
    })
}

pub fn parse_keypeek_pressed_key_packet(response: &[u8]) -> Option<KeyPeekPressedKeyPacket> {
    if response.first().copied()? != PRESSED_KEY_PACKET_MARKER {
        return None;
    }

    Some(KeyPeekPressedKeyPacket {
        row: *response.get(1)? as u16,
        col: *response.get(2)? as u16,
        pressed: *response.get(3)? != 0,
    })
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
}
