//! Convert KeyPeek's [`LayoutKey`] into a Keyplane
//! [`SemanticAction`](keyplane_core::SemanticAction).
//!
//! KeyPeek's tables operate on raw VIA `u16` keycodes and produce a
//! presentation-shaped `LayoutKey` (tap/behavior/argument/layer_ref/border).
//! Keyplane's domain wants a normalized Semantic Action, so this bridge maps the
//! one onto the other. The classification mirrors KeyPeek's own encoding:
//! `layer_ref` + a matching `L{n}` tap is a layer switch (border tells how it
//! activates), `layer_ref` + a real tap key is a layer-tap, the `Mod-Tap`
//! behavior is a tap-hold, and so on.

use crate::vendor::keycode_labels::get_layout_key;
use crate::vendor::layout_key::{BorderStyle, KeycodeKind, LayoutKey};
use keyplane_core::action::{LayerSwitch, SemanticAction};
use keyplane_core::ids::LayerId;

/// Maps a firmware layer index to its Stable Element ID (same contract as
/// `keyplane_core::resolve::semantic`).
pub type LayerIndexResolver<'a> = dyn Fn(u16) -> LayerId + 'a;

/// Derive a [`SemanticAction`] from a raw VIA `u16` keycode using KeyPeek's
/// keycode-label tables. `KC_TRANSPARENT` maps to
/// [`SemanticAction::Transparent`]; unrecognized codes fall back to
/// [`SemanticAction::Unknown`] via KeyPeek's hex label.
pub fn via_code_to_semantic(code: u16, layer_of: &LayerIndexResolver<'_>) -> SemanticAction {
    // KeyPeek returns None only for KC_TRANSPARENT.
    let Some(lk) = get_layout_key(code) else {
        return SemanticAction::Transparent;
    };
    classify(&lk, layer_of)
}

fn classify(lk: &LayoutKey, layer_of: &LayerIndexResolver<'_>) -> SemanticAction {
    let tap = lk.tap.full.clone();
    let behavior = lk.behavior.as_ref().map(|b| b.full.as_str());

    // Layer-referencing keys: switch vs. layer-tap.
    if let Some(layer_index) = lk.layer_ref {
        let layer = layer_of(layer_index as u16);
        if tap == format!("L{layer_index}") {
            // Pure layer switch; the border encodes how it activates.
            return SemanticAction::Layer {
                switch: border_to_switch(lk.border),
                layer,
                tap: None,
            };
        }
        // Layer-tap: tap the key, hold to enter the layer.
        return SemanticAction::Layer {
            switch: LayerSwitch::Tap,
            layer,
            tap: Some(tap),
        };
    }

    // Default-layer (DF) has no layer_ref but tap "L{n}" with a solid border.
    if lk.border != BorderStyle::None {
        if let Some(n) = tap.strip_prefix('L').and_then(|s| s.parse::<u16>().ok()) {
            return SemanticAction::Layer {
                switch: border_to_switch(lk.border),
                layer: layer_of(n),
                tap: None,
            };
        }
    }

    match behavior {
        Some("Mod-Tap") => SemanticAction::TapHold {
            tap,
            hold: lk.argument.as_ref().map(|a| a.full.clone()).unwrap_or_default(),
        },
        Some("Macro") | Some("Custom") | Some("Tap Dance") => SemanticAction::Macro {
            label: behavior.map(str::to_string).unwrap_or(tap),
        },
        _ if lk.kind == KeycodeKind::Modifier => SemanticAction::Modifier { label: tap },
        _ if tap.is_empty() && lk.symbol.is_none() => SemanticAction::None,
        _ if tap.starts_with("0x") => SemanticAction::Unknown { raw: tap },
        _ => SemanticAction::Key { label: tap },
    }
}

fn border_to_switch(border: BorderStyle) -> LayerSwitch {
    match border {
        // Persistent: toggle / to-layer / default-layer.
        BorderStyle::Solid => LayerSwitch::Toggle,
        // Sticky / one-shot.
        BorderStyle::Dashed => LayerSwitch::OneShot,
        // Momentary / while-held.
        BorderStyle::None => LayerSwitch::Momentary,
    }
}
