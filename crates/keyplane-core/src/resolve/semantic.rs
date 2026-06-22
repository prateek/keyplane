//! Semantic Action derivation (ADR 0030).
//!
//! Parses source-specific Raw Action tokens into normalized [`SemanticAction`]s
//! for visualization. This understands actions enough to explain them, never to
//! execute firmware behavior; unrecognized tokens become
//! [`SemanticAction::Unknown`] rather than a guess.

use crate::action::{LayerSwitch, RawAction, SemanticAction};
use crate::ids::LayerId;

/// Maps a firmware layer index to its Stable Element ID, supplied by the keymap
/// being built. Returns a synthetic id when the index is unknown so derivation
/// never fails outright.
pub type LayerIndexResolver<'a> = dyn Fn(u16) -> LayerId + 'a;

/// Derive a [`SemanticAction`] from a [`RawAction`].
///
/// `layer_of` maps a numeric layer index to a [`LayerId`] for layer-switch
/// actions. QMK and ZMK tokens are parsed; other raw kinds fall back to a best
/// effort that preserves the token.
pub fn derive(raw: &RawAction, layer_of: &LayerIndexResolver<'_>) -> SemanticAction {
    match raw {
        RawAction::Qmk(token) | RawAction::KeyPeek(token) => derive_qmk(token, layer_of),
        RawAction::Zmk(token) => derive_zmk(token, layer_of),
        RawAction::Kanata(token) => derive_kanata(token, layer_of),
        RawAction::HostEvent(name) => SemanticAction::Key {
            label: prettify_key(name),
        },
        RawAction::ViaCode(code) => SemanticAction::Unknown {
            raw: format!("0x{code:04X}"),
        },
        RawAction::Opaque(text) => SemanticAction::Unknown { raw: text.clone() },
    }
}

/// Parse a QMK/Vial keycode token (also used for KeyPeek-reported tokens).
pub fn derive_qmk(token: &str, layer_of: &LayerIndexResolver<'_>) -> SemanticAction {
    let t = token.trim();
    match t {
        "KC_TRNS" | "KC_TRANSPARENT" | "_______" | "TRNS" => return SemanticAction::Transparent,
        "KC_NO" | "XXXXXXX" | "NO" => return SemanticAction::None,
        _ => {}
    }

    if let Some(inner) = call_args(t, &["MO", "TT"]) {
        if let Some(n) = inner.first().and_then(|s| s.parse::<u16>().ok()) {
            return SemanticAction::Layer {
                switch: if t.starts_with("TT") {
                    LayerSwitch::Toggle
                } else {
                    LayerSwitch::Momentary
                },
                layer: layer_of(n),
                tap: None,
            };
        }
    }
    if let Some(inner) = call_args(t, &["TG", "TO"]) {
        if let Some(n) = inner.first().and_then(|s| s.parse::<u16>().ok()) {
            return SemanticAction::Layer {
                switch: LayerSwitch::Toggle,
                layer: layer_of(n),
                tap: None,
            };
        }
    }
    if let Some(inner) = call_args(t, &["OSL"]) {
        if let Some(n) = inner.first().and_then(|s| s.parse::<u16>().ok()) {
            return SemanticAction::Layer {
                switch: LayerSwitch::OneShot,
                layer: layer_of(n),
                tap: None,
            };
        }
    }
    if let Some(inner) = call_args(t, &["DF"]) {
        if let Some(n) = inner.first().and_then(|s| s.parse::<u16>().ok()) {
            return SemanticAction::Layer {
                switch: LayerSwitch::Default,
                layer: layer_of(n),
                tap: None,
            };
        }
    }
    if let Some(inner) = call_args(t, &["LT"]) {
        if inner.len() == 2 {
            if let Ok(n) = inner[0].parse::<u16>() {
                return SemanticAction::Layer {
                    switch: LayerSwitch::Tap,
                    layer: layer_of(n),
                    tap: Some(prettify_key(&inner[1])),
                };
            }
        }
    }
    if let Some(inner) = call_args(t, &["MT"]) {
        if inner.len() == 2 {
            return SemanticAction::TapHold {
                tap: prettify_key(&inner[1]),
                hold: prettify_mod(&inner[0]),
            };
        }
    }
    if let Some(inner) = call_args(t, &["MACRO", "DM_PLY"]) {
        let label = inner.first().map(|s| s.to_string()).unwrap_or_default();
        return SemanticAction::Macro {
            label: if label.is_empty() {
                "Macro".to_string()
            } else {
                label
            },
        };
    }

    if is_modifier_token(t) {
        return SemanticAction::Modifier {
            label: prettify_mod(t),
        };
    }
    if t.starts_with("KC_MS_") || t.starts_with("MS_") || t.starts_with("KC_WH_") {
        return SemanticAction::Mouse {
            label: prettify_key(t),
        };
    }
    if t.starts_with("KC_") || is_bare_key(t) {
        return SemanticAction::Key {
            label: prettify_key(t),
        };
    }
    SemanticAction::Unknown { raw: t.to_string() }
}

/// Parse a ZMK behavior binding such as `&mo 2` or `&kp A`.
pub fn derive_zmk(token: &str, layer_of: &LayerIndexResolver<'_>) -> SemanticAction {
    let t = token.trim();
    let mut parts = t.split_whitespace();
    let behavior = parts.next().unwrap_or("");
    let args: Vec<&str> = parts.collect();
    match behavior {
        "&trans" => SemanticAction::Transparent,
        "&none" => SemanticAction::None,
        "&kp" => SemanticAction::Key {
            label: prettify_key(args.first().copied().unwrap_or("")),
        },
        "&mo" => layer_switch(LayerSwitch::Momentary, args.first(), layer_of),
        "&to" => layer_switch(LayerSwitch::Default, args.first(), layer_of),
        "&tog" => layer_switch(LayerSwitch::Toggle, args.first(), layer_of),
        "&sl" => layer_switch(LayerSwitch::OneShot, args.first(), layer_of),
        "&lt" => {
            if args.len() == 2 {
                if let Ok(n) = args[0].parse::<u16>() {
                    return SemanticAction::Layer {
                        switch: LayerSwitch::Tap,
                        layer: layer_of(n),
                        tap: Some(prettify_key(args[1])),
                    };
                }
            }
            SemanticAction::Unknown { raw: t.to_string() }
        }
        "&mt" => {
            if args.len() == 2 {
                SemanticAction::TapHold {
                    tap: prettify_key(args[1]),
                    hold: prettify_mod(args[0]),
                }
            } else {
                SemanticAction::Unknown { raw: t.to_string() }
            }
        }
        _ => SemanticAction::Unknown { raw: t.to_string() },
    }
}

/// Parse a small subset of Kanata action strings (`@layer`, `(layer-while-held
/// n)`-style tokens are out of scope; companion profiles supply real keymaps).
pub fn derive_kanata(token: &str, _layer_of: &LayerIndexResolver<'_>) -> SemanticAction {
    let t = token.trim();
    match t {
        "_" | "transparent" => SemanticAction::Transparent,
        "XX" | "none" => SemanticAction::None,
        _ => SemanticAction::Key {
            label: prettify_key(t),
        },
    }
}

fn layer_switch(
    switch: LayerSwitch,
    arg: Option<&&str>,
    layer_of: &LayerIndexResolver<'_>,
) -> SemanticAction {
    match arg.and_then(|s| s.parse::<u16>().ok()) {
        Some(n) => SemanticAction::Layer {
            switch,
            layer: layer_of(n),
            tap: None,
        },
        None => SemanticAction::Unknown {
            raw: arg.map(|s| s.to_string()).unwrap_or_default(),
        },
    }
}

/// If `token` is `NAME(a,b,...)` for one of `names`, return the args.
fn call_args(token: &str, names: &[&str]) -> Option<Vec<String>> {
    let open = token.find('(')?;
    if !token.ends_with(')') {
        return None;
    }
    let name = &token[..open];
    if !names.contains(&name) {
        return None;
    }
    let inner = &token[open + 1..token.len() - 1];
    Some(inner.split(',').map(|s| s.trim().to_string()).collect())
}

fn is_bare_key(t: &str) -> bool {
    // A single character or digit written without the KC_ prefix.
    t.chars().count() == 1 && t.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_modifier_token(t: &str) -> bool {
    matches!(
        t,
        "KC_LSFT"
            | "KC_RSFT"
            | "KC_LCTL"
            | "KC_RCTL"
            | "KC_LALT"
            | "KC_RALT"
            | "KC_LGUI"
            | "KC_RGUI"
            | "KC_LSHIFT"
            | "KC_RSHIFT"
            | "LSFT"
            | "RSFT"
            | "LCTL"
            | "RCTL"
            | "LALT"
            | "RALT"
            | "LGUI"
            | "RGUI"
    )
}

fn prettify_mod(t: &str) -> String {
    match t {
        "KC_LSFT" | "KC_RSFT" | "KC_LSHIFT" | "KC_RSHIFT" | "LSFT" | "RSFT" | "MOD_LSFT" => {
            "Shift".to_string()
        }
        "KC_LCTL" | "KC_RCTL" | "LCTL" | "RCTL" | "MOD_LCTL" => "Ctrl".to_string(),
        "KC_LALT" | "KC_RALT" | "LALT" | "RALT" | "MOD_LALT" => "Alt".to_string(),
        "KC_LGUI" | "KC_RGUI" | "LGUI" | "RGUI" | "MOD_LGUI" => "Cmd".to_string(),
        other => prettify_key(other),
    }
}

/// Turn a keycode token into a short human label.
fn prettify_key(t: &str) -> String {
    let core = t.strip_prefix("KC_").unwrap_or(t);
    let named = match core {
        "SPC" | "SPACE" => "Space",
        "ENT" | "ENTER" => "Enter",
        "BSPC" => "Bksp",
        "DEL" => "Del",
        "ESC" => "Esc",
        "TAB" => "Tab",
        "LSFT" | "RSFT" => "Shift",
        "LCTL" | "RCTL" => "Ctrl",
        "LALT" | "RALT" => "Alt",
        "LGUI" | "RGUI" => "Cmd",
        "CAPS" => "Caps",
        "MINS" => "-",
        "EQL" => "=",
        "LBRC" => "[",
        "RBRC" => "]",
        "BSLS" => "\\",
        "SCLN" => ";",
        "QUOT" => "'",
        "GRV" => "`",
        "COMM" => ",",
        "DOT" => ".",
        "SLSH" => "/",
        "LEFT" => "←",
        "RGHT" | "RIGHT" => "→",
        "UP" => "↑",
        "DOWN" => "↓",
        "" => "",
        other => other,
    };
    named.to_string()
}
