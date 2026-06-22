//! The KeyPeek backend's pure adapters: device definition → Keyplane model, and
//! firmware layer packets → Layer Stack. The live HID transport is
//! hardware-gated, but these adapters are tested with synthetic device data.

use keyplane_core::action::{RawAction, SemanticAction};
use keyplane_core::ids::{KeyId, LayerId};
use keyplane_core::model::ActivationKind;
use keyplane_keypeek::backend::{build_model, decode_layer_packet, layer_stack_from_masks};
use keyplane_keypeek::vendor::protocols::{Key, KeyboardLayout};

fn key(row: usize, col: usize, x: f32, y: f32) -> Key {
    Key {
        row,
        col,
        x,
        y,
        w: 1.0,
        h: 1.0,
        r: 0.0,
    }
}

/// A 2-key, 2-layer synthetic device: base = [A, MO(1)], layer 1 = [1, transparent].
fn synthetic() -> (KeyboardLayout, Vec<Vec<Vec<u16>>>) {
    let layout = KeyboardLayout {
        name: "Default".into(),
        keys: vec![key(0, 0, 0.0, 0.0), key(0, 1, 1.0, 0.0)],
    };
    // raw[layer][row][col]; KC_A=0x04, MO(1)=0x5221, KC_1=0x1E, KC_TRNS=0x01
    let raw = vec![
        vec![vec![0x0004, 0x5221]],
        vec![vec![0x001E, 0x0001]],
    ];
    (layout, raw)
}

#[test]
fn build_model_uses_geometry_keymap_and_keypeek_labels() {
    let (layout, raw) = synthetic();
    let model = build_model(&layout, &raw);

    assert_eq!(model.physical_layout.keys.len(), 2);
    assert_eq!(model.keymap.layers.len(), 2);

    // Raw VIA codes are preserved (ADR 0005)...
    let base = &model.keymap.layers[0];
    let a = base.entry(&KeyId::new("r0c0")).unwrap();
    assert_eq!(a.raw, RawAction::ViaCode(0x0004));
    // ...and labeled via KeyPeek's tables.
    assert_eq!(a.semantic, SemanticAction::Key { label: "A".into() });

    // MO(1) classifies as a momentary layer switch to layer-1.
    let mo = base.entry(&KeyId::new("r0c1")).unwrap();
    assert!(matches!(
        mo.semantic,
        SemanticAction::Layer { layer: ref l, .. } if *l == LayerId::new("layer-1")
    ));

    // Transparent keycode resolves to a transparent entry.
    let trns = model.keymap.layers[1].entry(&KeyId::new("r0c1")).unwrap();
    assert_eq!(trns.semantic, SemanticAction::Transparent);
}

#[test]
fn layer_masks_build_an_ordered_stack() {
    // default layer 0, momentary layer 2 active.
    let stack = layer_stack_from_masks(0b001, 0b100, 4);
    assert_eq!(stack.active.len(), 2);
    assert_eq!(stack.active[0].activation, ActivationKind::Default);
    assert_eq!(stack.active[0].layer, LayerId::new("layer-0"));
    assert_eq!(stack.top().unwrap().layer, LayerId::new("layer-2"));
    assert_eq!(stack.top().unwrap().activation, ActivationKind::Momentary);
}

#[test]
fn empty_masks_fall_back_to_base_layer() {
    let stack = layer_stack_from_masks(0, 0, 4);
    assert_eq!(stack.active.len(), 1);
    assert_eq!(stack.active[0].layer, LayerId::new("layer-0"));
}

#[test]
fn decode_layer_packet_reads_default_and_active_masks() {
    // 0xff, size=1, default=0x01, active=0x04
    let packet = vec![0xff, 0x01, 0x01, 0x04];
    assert_eq!(decode_layer_packet(&packet), Some((0x01, 0x04)));
}

#[test]
fn decode_ignores_non_layer_packets() {
    // Firmware echoing the subscribe command, or a key-press packet.
    assert_eq!(decode_layer_packet(&[0xC0, 0xA1]), None);
    assert_eq!(decode_layer_packet(&[0xF1, 0, 0, 1]), None);
}
