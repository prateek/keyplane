//! The KeyPeek keycode-label bridge: raw VIA `u16` codes → Semantic Actions.
//!
//! These exercise KeyPeek's actual tables (the reason for reusing them): numeric
//! keycodes that the hand-rolled string parser in `keyplane-core` could not
//! classify now resolve to accurate Semantic Actions.

use keyplane_core::action::{LayerSwitch, SemanticAction};
use keyplane_core::ids::LayerId;
use keyplane_keypeek::bridge::via_code_to_semantic;

fn layer_of(n: u16) -> LayerId {
    LayerId::new(format!("layer-{n}"))
}

fn semantic(code: u16) -> SemanticAction {
    via_code_to_semantic(code, &layer_of)
}

// VIA/QMK keycode constants (protocol v12 numeric encoding).
const KC_NO: u16 = 0x0000;
const KC_TRANSPARENT: u16 = 0x0001;
const KC_A: u16 = 0x0004;
const KC_SPC: u16 = 0x002C;
const QK_MOMENTARY: u16 = 0x5220; // MO(n) = base + n
const QK_TOGGLE_LAYER: u16 = 0x5260; // TG(n)
const QK_LAYER_TAP: u16 = 0x4000; // LT(n, kc) = base | (n<<8) | kc
const QK_MOD_TAP: u16 = 0x2000; // MT(mod, kc) = base | (mod<<8) | kc
const MOD_LCTL: u16 = 0x01;

#[test]
fn basic_key_resolves_to_a_named_key() {
    assert_eq!(semantic(KC_A), SemanticAction::Key { label: "A".into() });
}

#[test]
fn transparent_and_none_are_distinguished() {
    assert_eq!(semantic(KC_TRANSPARENT), SemanticAction::Transparent);
    assert_eq!(semantic(KC_NO), SemanticAction::None);
}

#[test]
fn momentary_layer_switch_resolves_with_target_layer() {
    match semantic(QK_MOMENTARY + 1) {
        SemanticAction::Layer { switch, layer, tap } => {
            assert_eq!(switch, LayerSwitch::Momentary);
            assert_eq!(layer, LayerId::new("layer-1"));
            assert_eq!(tap, None);
        }
        other => panic!("expected momentary layer, got {other:?}"),
    }
}

#[test]
fn toggle_layer_switch_uses_solid_border_semantics() {
    match semantic(QK_TOGGLE_LAYER + 2) {
        SemanticAction::Layer { switch, layer, .. } => {
            assert_eq!(switch, LayerSwitch::Toggle);
            assert_eq!(layer, LayerId::new("layer-2"));
        }
        other => panic!("expected toggle layer, got {other:?}"),
    }
}

#[test]
fn layer_tap_keeps_both_the_tap_key_and_target_layer() {
    // LT(2, KC_SPC)
    let code = QK_LAYER_TAP | (2 << 8) | KC_SPC;
    match semantic(code) {
        SemanticAction::Layer { switch, layer, tap } => {
            assert_eq!(switch, LayerSwitch::Tap);
            assert_eq!(layer, LayerId::new("layer-2"));
            assert!(tap.is_some(), "tap role preserved");
        }
        other => panic!("expected layer-tap, got {other:?}"),
    }
}

#[test]
fn mod_tap_resolves_to_tap_hold() {
    // MT(MOD_LCTL, KC_A)
    let code = QK_MOD_TAP | (MOD_LCTL << 8) | KC_A;
    match semantic(code) {
        SemanticAction::TapHold { tap, hold } => {
            assert_eq!(tap, "A");
            assert_eq!(hold, "Ctrl");
        }
        other => panic!("expected tap-hold, got {other:?}"),
    }
}
