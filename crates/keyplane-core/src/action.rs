//! Raw and Semantic actions (ADR 0005, ADR 0030).
//!
//! Every keymap entry preserves its [`RawAction`] — the exact source
//! representation — while deriving a normalized [`SemanticAction`] for
//! visualization, Display Legends, layer hints, and State Confidence warnings.
//! The semantic layer never executes firmware behavior; it only explains it.

use crate::ids::LayerId;
use serde::{Deserialize, Serialize};

/// The original, source-specific action representation, preserved verbatim.
///
/// Keeping the raw form lets the app debug protocol-specific behavior and
/// improve importers later without losing fidelity (ADR 0005).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", content = "value", rename_all = "kebab-case")]
pub enum RawAction {
    /// A QMK/Vial keycode token such as `KC_A` or `LT(2,KC_SPC)`.
    Qmk(String),
    /// A Vial/VIA numeric keycode.
    ViaCode(u16),
    /// A ZMK behavior binding such as `&kp A` or `&mo 2`.
    Zmk(String),
    /// A Kanata action string.
    Kanata(String),
    /// A host input event name, used by sentinel/host-event backends.
    HostEvent(String),
    /// A KeyPeek-reported keycode token.
    KeyPeek(String),
    /// Free-form text that no importer could classify further.
    Opaque(String),
}

impl RawAction {
    /// The raw token as written by the source, for debugging and Source
    /// Inspector display.
    pub fn token(&self) -> String {
        match self {
            RawAction::Qmk(s)
            | RawAction::Zmk(s)
            | RawAction::Kanata(s)
            | RawAction::HostEvent(s)
            | RawAction::KeyPeek(s)
            | RawAction::Opaque(s) => s.clone(),
            RawAction::ViaCode(code) => format!("0x{code:04X}"),
        }
    }
}

/// The known reason a layer entry switches layers, when derivable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayerSwitch {
    /// Momentary: active only while held (QMK `MO`, ZMK `&mo`).
    Momentary,
    /// Toggle: flips the layer on/off (QMK `TG`, ZMK `&tog`).
    Toggle,
    /// Tap-hold layer: tap for a key, hold for the layer (QMK `LT`).
    Tap,
    /// One-shot layer (QMK `OSL`).
    OneShot,
    /// Switches the default/base layer (QMK `DF`).
    Default,
}

/// A normalized interpretation of a [`RawAction`] for visualization only.
///
/// Per ADR 0030 the app understands actions enough to explain them, not enough
/// to reimplement firmware. Unrecognized raw actions fall back to
/// [`SemanticAction::Unknown`] rather than guessing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SemanticAction {
    /// A plain key press, e.g. a letter, digit, or named key.
    Key { label: String },
    /// A modifier such as Shift, Ctrl, Alt, or GUI.
    Modifier { label: String },
    /// A layer switch targeting `layer` with a known [`LayerSwitch`] kind.
    Layer {
        switch: LayerSwitch,
        layer: LayerId,
        /// The tap key for tap-hold/`LT` style actions, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tap: Option<String>,
    },
    /// A tap-hold (mod-tap) that is not a layer switch, e.g. QMK `MT`.
    TapHold { tap: String, hold: String },
    /// A macro invocation.
    Macro { label: String },
    /// A transparent entry that defers to a lower layer (`KC_TRNS`, `&trans`).
    Transparent,
    /// An explicit no-op (`KC_NO`, `&none`).
    None,
    /// A mouse action (movement, button, or wheel).
    Mouse { label: String },
    /// Recognized as an action, but the app has no richer interpretation.
    Unknown { raw: String },
}

impl SemanticAction {
    pub fn is_transparent(&self) -> bool {
        matches!(self, SemanticAction::Transparent)
    }

    pub fn is_none(&self) -> bool {
        matches!(self, SemanticAction::None)
    }

    /// A short human label for compact Style Variants and tests.
    pub fn short_label(&self) -> String {
        match self {
            SemanticAction::Key { label }
            | SemanticAction::Modifier { label }
            | SemanticAction::Macro { label }
            | SemanticAction::Mouse { label } => label.clone(),
            SemanticAction::Layer { layer, switch, .. } => match switch {
                LayerSwitch::Default => format!("DF {layer}"),
                LayerSwitch::Toggle => format!("TG {layer}"),
                LayerSwitch::OneShot => format!("OSL {layer}"),
                LayerSwitch::Momentary => format!("L{layer}"),
                LayerSwitch::Tap => format!("L{layer}"),
            },
            SemanticAction::TapHold { tap, .. } => tap.clone(),
            SemanticAction::Transparent => "▽".to_string(),
            SemanticAction::None => String::new(),
            SemanticAction::Unknown { raw } => raw.clone(),
        }
    }
}
