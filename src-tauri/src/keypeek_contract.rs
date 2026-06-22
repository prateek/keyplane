/// KeyPeek-compatible firmware packet contract.
///
/// This module is adapted from the vendored KeyPeek source slice in
/// `third_party/keypeek`, upstream commit
/// `9c8d4b3f7c30e088367ba361a52eb597e146a276`.
/// See `third_party/keypeek/src/keyboard.rs` and
/// `third_party/keypeek/src/protocols/mod.rs`.

/// KeyPeek firmware-module packets mark layer-state reports with `0xff`.
pub const LAYER_STATE_PACKET_MARKER: u8 = 0xff;
/// KeyPeek firmware-module packets mark physical key press reports with `0xf1`.
pub const PRESSED_KEY_PACKET_MARKER: u8 = 0xf1;
/// Raw HID keepalive marker used by the KeyPeek firmware module.
pub const SUBSCRIBE_MARKER: u8 = 0xc0;
pub const SUBSCRIBE_ACTIVE: u8 = 0xa1;
pub const SUBSCRIBE_INACTIVE: u8 = 0xa0;

/// KeyPeek treats the packet size field as `sizeof(layer_state_t)`, capped at 4 bytes.
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

#[cfg(test)]
mod tests {
    use super::*;

    const UPSTREAM_KEYBOARD_RS: &str = include_str!("../../third_party/keypeek/src/keyboard.rs");
    const UPSTREAM_PROTOCOLS_MOD_RS: &str =
        include_str!("../../third_party/keypeek/src/protocols/mod.rs");

    #[test]
    fn adapted_packet_markers_match_vendored_keypeek_source() {
        assert!(UPSTREAM_KEYBOARD_RS.contains("if response[0] == 0xff"));
        assert!(UPSTREAM_KEYBOARD_RS.contains("} else if response[0] == 0xF1 {"));
        assert!(UPSTREAM_PROTOCOLS_MOD_RS.contains("KEYPEEK_SUBSCRIBE_MARKER: u8 = 0xC0"));
        assert!(UPSTREAM_PROTOCOLS_MOD_RS.contains("KEYPEEK_SUBSCRIBE_ACTIVE: u8 = 0xA1"));
        assert!(UPSTREAM_PROTOCOLS_MOD_RS.contains("KEYPEEK_SUBSCRIBE_INACTIVE: u8 = 0xA0"));

        assert_eq!(LAYER_STATE_PACKET_MARKER, 0xff);
        assert_eq!(PRESSED_KEY_PACKET_MARKER, 0xf1);
        assert_eq!(SUBSCRIBE_MARKER, 0xc0);
        assert_eq!(SUBSCRIBE_ACTIVE, 0xa1);
        assert_eq!(SUBSCRIBE_INACTIVE, 0xa0);
    }

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
