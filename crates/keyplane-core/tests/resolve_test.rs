//! Layer resolution, Effective Actions, inheritance, and Display Legends.
//!
//! These assert observable rendering outcomes through the public resolve seam,
//! not private helper structure (PRD Testing Decisions).

use keyplane_core::action::{LayerSwitch, RawAction, SemanticAction};
use keyplane_core::backend::fake::demo_model;
use keyplane_core::ids::{KeyId, LayerId};
use keyplane_core::model::{ActivationKind, ActiveLayer, LayerStack};
use keyplane_core::resolve::{resolve_key, resolve_layout, semantic};

fn nav_stack() -> LayerStack {
    LayerStack::new(vec![
        ActiveLayer::new(LayerId::new("layer-0"), ActivationKind::Default),
        ActiveLayer::new(LayerId::new("layer-1"), ActivationKind::Momentary),
    ])
}

#[test]
fn momentary_layer_overrides_base_for_defined_keys() {
    let model = demo_model();
    let resolved = resolve_key(&model, &nav_stack(), &KeyId::new("k0"));
    assert_eq!(
        resolved.effective,
        SemanticAction::Key { label: "1".into() }
    );
    assert!(!resolved.inherited, "k0 is defined on the top layer");
    assert_eq!(resolved.source_layer, LayerId::new("layer-1"));
}

#[test]
fn transparent_key_inherits_from_base_and_marks_inherited() {
    let model = demo_model();
    // k8 is transparent on layer-1, so it inherits "I" from the base layer.
    let resolved = resolve_key(&model, &nav_stack(), &KeyId::new("k8"));
    assert_eq!(
        resolved.effective,
        SemanticAction::Key { label: "I".into() }
    );
    assert!(
        resolved.inherited,
        "value came from a lower layer than the top active layer"
    );
    assert_eq!(resolved.source_layer, LayerId::new("layer-0"));
}

#[test]
fn base_stack_shows_base_actions_without_inheritance() {
    let model = demo_model();
    let base = LayerStack::base(LayerId::new("layer-0"));
    let resolved = resolve_key(&model, &base, &KeyId::new("k0"));
    assert_eq!(
        resolved.effective,
        SemanticAction::Key { label: "A".into() }
    );
    assert!(!resolved.inherited);
}

#[test]
fn resolve_layout_covers_every_physical_key() {
    let model = demo_model();
    let resolved = resolve_layout(&model, &nav_stack());
    assert_eq!(resolved.len(), model.physical_layout.keys.len());
}

#[test]
fn momentary_layer_key_derives_layer_semantic() {
    let model = demo_model();
    let base = LayerStack::base(LayerId::new("layer-0"));
    let resolved = resolve_key(&model, &base, &KeyId::new("k12")); // MO(1)
    match resolved.effective {
        SemanticAction::Layer { switch, layer, .. } => {
            assert_eq!(switch, LayerSwitch::Momentary);
            assert_eq!(layer, LayerId::new("layer-1"));
        }
        other => panic!("expected a layer action, got {other:?}"),
    }
    // The legend exposes the target layer as a structured slot.
    assert_eq!(resolved.legend.layer.as_deref(), Some("layer-1"));
}

#[test]
fn unknown_action_falls_back_without_guessing() {
    let raw = RawAction::Qmk("QK_TOTALLY_MADE_UP".into());
    let action = semantic::derive(&raw, &|n| LayerId::new(format!("layer-{n}")));
    assert!(matches!(action, SemanticAction::Unknown { .. }));
}

#[test]
fn tap_hold_layer_tap_keeps_both_roles() {
    // LT(2, KC_SPC): tap is Space, hold enters layer 2.
    let raw = RawAction::Qmk("LT(2,KC_SPC)".into());
    let action = semantic::derive(&raw, &|n| LayerId::new(format!("layer-{n}")));
    match action {
        SemanticAction::Layer { switch, layer, tap } => {
            assert_eq!(switch, LayerSwitch::Tap);
            assert_eq!(layer, LayerId::new("layer-2"));
            assert_eq!(tap.as_deref(), Some("Space"));
        }
        other => panic!("expected layer-tap, got {other:?}"),
    }
}

#[test]
fn zmk_bindings_parse() {
    let resolver = |n: u16| LayerId::new(format!("layer-{n}"));
    assert_eq!(
        semantic::derive(&RawAction::Zmk("&trans".into()), &resolver),
        SemanticAction::Transparent
    );
    assert!(matches!(
        semantic::derive(&RawAction::Zmk("&mo 3".into()), &resolver),
        SemanticAction::Layer {
            switch: LayerSwitch::Momentary,
            ..
        }
    ));
}
