//! Runtime State: the live, ordered Layer Stack (ADR 0032).
//!
//! Active layers are modeled as an ordered [`LayerStack`], not a single current
//! layer, so momentary layers, toggles, tap-holds, and the default layer all
//! resolve coherently. Each active layer carries its [`ActivationKind`] and the
//! stack carries a [`StateConfidence`].

use crate::ids::{KeyId, LayerId};
use serde::{Deserialize, Serialize};

/// The known reason a layer is active (ADR 0032).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActivationKind {
    /// The base layer; always present at the bottom of the stack.
    Default,
    /// Active while a key is held.
    Momentary,
    /// Toggled on until toggled off.
    Toggle,
    /// Active via a tap-hold hold.
    TapHold,
    /// Locked on (e.g. one-shot lock).
    Lock,
    /// Active because a remapper (Kanata) reports it.
    Remapper,
    /// The source did not report why the layer is active.
    Unknown,
}

/// How trustworthy a Runtime State value is (ADR 0016, 0032).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StateConfidence {
    /// Reported by a firmware-aware or authoritative remapper source.
    Authoritative,
    /// Inferred (e.g. sentinel keys, polling) and may be wrong at startup.
    Inferred,
    /// The app cannot vouch for this value at all.
    Unknown,
}

/// One active layer in the stack.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveLayer {
    pub layer: LayerId,
    pub activation: ActivationKind,
}

impl ActiveLayer {
    pub fn new(layer: impl Into<LayerId>, activation: ActivationKind) -> Self {
        Self {
            layer: layer.into(),
            activation,
        }
    }
}

/// The ordered set of active layers used to resolve Effective Actions.
///
/// Order is bottom-to-top: the first entry is the base layer and the last entry
/// is the topmost active layer. Resolution walks the stack top-down, so later
/// entries take precedence (Layer Precedence).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerStack {
    pub active: Vec<ActiveLayer>,
}

impl LayerStack {
    pub fn new(active: Vec<ActiveLayer>) -> Self {
        Self { active }
    }

    /// A stack with only the base/default layer active.
    pub fn base(layer: impl Into<LayerId>) -> Self {
        Self {
            active: vec![ActiveLayer::new(layer, ActivationKind::Default)],
        }
    }

    /// The topmost active layer, which the overlay may highlight (ADR 0032).
    pub fn top(&self) -> Option<&ActiveLayer> {
        self.active.last()
    }

    /// Active layer ids ordered top-down (highest precedence first).
    pub fn top_down(&self) -> impl Iterator<Item = &ActiveLayer> {
        self.active.iter().rev()
    }

    pub fn contains(&self, layer: &LayerId) -> bool {
        self.active.iter().any(|a| &a.layer == layer)
    }
}

/// The live runtime values: the Layer Stack, pressed keys, and confidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeState {
    pub layer_stack: LayerStack,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pressed: Vec<KeyId>,
    pub confidence: StateConfidence,
}

impl RuntimeState {
    pub fn new(layer_stack: LayerStack, confidence: StateConfidence) -> Self {
        Self {
            layer_stack,
            pressed: Vec::new(),
            confidence,
        }
    }
}
