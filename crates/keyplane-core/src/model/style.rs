//! Visual Style (ADR 0011, ADR 0013).
//!
//! Presentation applied after keyboard data is known. keyviz style JSON imports
//! affect only these fields (ADR 0018). One overlay mode with Style Variants
//! keeps the product focused while allowing different looks.

use serde::{Deserialize, Serialize};

/// A reusable visual treatment for the overlay surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StyleVariant {
    /// Rich keycaps preserving structured Legend Slots.
    Detailed,
    /// Compact keycaps that collapse legends into a single label.
    Minimal,
}

impl Default for StyleVariant {
    fn default() -> Self {
        StyleVariant::Detailed
    }
}

/// The Visual Style configuration for the overlay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisualStyle {
    pub variant: StyleVariant,
    /// Whether to draw the subtle inherited indicator on transparent keys that
    /// resolve through inheritance (ADR 0031).
    pub show_inherited_indicator: bool,
    /// Overlay opacity in `0.0..=1.0`.
    pub opacity: f64,
    /// Optional accent color (CSS string) imported from keyviz style JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    /// Optional keycap background color (CSS string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keycap_color: Option<String>,
    /// Optional legend text color (CSS string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
}

impl Default for VisualStyle {
    fn default() -> Self {
        Self {
            variant: StyleVariant::default(),
            show_inherited_indicator: true,
            opacity: 0.92,
            accent: None,
            keycap_color: None,
            text_color: None,
        }
    }
}

impl VisualStyle {
    /// Whether legends should be collapsed to a single label for this variant.
    pub fn collapse_legends(&self) -> bool {
        matches!(self.variant, StyleVariant::Minimal)
    }
}
