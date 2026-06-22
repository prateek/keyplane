//! Structured Display Legends (ADR 0029).
//!
//! A keycap legend is modeled as named [`LegendSlot`]s rather than one string,
//! so primary, shifted, tap, hold, layer, and action meanings render cleanly.
//! Minimal Style Variants collapse the slots into a single label via
//! [`DisplayLegend::collapse`].

use serde::{Deserialize, Serialize};

/// A named part of a [`DisplayLegend`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LegendSlot {
    /// The main label (the unmodified key).
    Primary,
    /// The shifted / secondary label.
    Shifted,
    /// The tap role of a tap-hold key.
    Tap,
    /// The hold role of a tap-hold key.
    Hold,
    /// A layer hint, e.g. the target layer of a momentary switch.
    Layer,
    /// An action hint, e.g. "macro" or "mouse".
    Action,
    /// An icon name the renderer can map to a glyph.
    Icon,
}

/// A structured, multi-slot keycap legend.
///
/// Slots are stored in a fixed field layout (not a map) so equality and
/// serialization are stable and the renderer can address slots directly.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayLegend {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shifted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tap: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl DisplayLegend {
    /// A legend with only the primary slot filled.
    pub fn primary(label: impl Into<String>) -> Self {
        Self {
            primary: Some(label.into()),
            ..Default::default()
        }
    }

    pub fn get(&self, slot: LegendSlot) -> Option<&str> {
        let value = match slot {
            LegendSlot::Primary => &self.primary,
            LegendSlot::Shifted => &self.shifted,
            LegendSlot::Tap => &self.tap,
            LegendSlot::Hold => &self.hold,
            LegendSlot::Layer => &self.layer,
            LegendSlot::Action => &self.action,
            LegendSlot::Icon => &self.icon,
        };
        value.as_deref()
    }

    pub fn is_empty(&self) -> bool {
        self.primary.is_none()
            && self.shifted.is_none()
            && self.tap.is_none()
            && self.hold.is_none()
            && self.layer.is_none()
            && self.action.is_none()
            && self.icon.is_none()
    }

    /// Collapse the structured legend into a single label for minimal Style
    /// Variants (ADR 0029). Prefers the tap role, then primary, then the most
    /// informative remaining slot, so a tap-hold key reads as its tap action.
    pub fn collapse(&self) -> String {
        if let Some(tap) = &self.tap {
            return tap.clone();
        }
        if let Some(primary) = &self.primary {
            return primary.clone();
        }
        for slot in [
            LegendSlot::Layer,
            LegendSlot::Action,
            LegendSlot::Hold,
            LegendSlot::Shifted,
            LegendSlot::Icon,
        ] {
            if let Some(value) = self.get(slot) {
                return value.to_string();
            }
        }
        String::new()
    }
}
