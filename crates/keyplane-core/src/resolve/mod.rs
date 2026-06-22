//! Effective Action resolution (ADR 0031, ADR 0032, ADR 0036).
//!
//! Given a [`KeyboardModel`] and a live [`LayerStack`], resolve each Physical
//! Key to the Effective Action the user will get, walking the stack top-down
//! and inheriting through transparent entries. Rust owns this so the overlay,
//! Source Inspector, and tests all see identical results.

pub mod semantic;

use crate::action::SemanticAction;
use crate::ids::{KeyId, LayerId};
use crate::legend::DisplayLegend;
use crate::model::{KeyboardModel, LayerStack};
use serde::{Deserialize, Serialize};

/// The resolved view of one Physical Key under the active Layer Stack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResolvedKey {
    pub key: KeyId,
    pub geometry: crate::geometry::KeyGeometry,
    /// The Effective Action after walking the stack and inheriting.
    pub effective: SemanticAction,
    pub legend: DisplayLegend,
    /// The layer that actually supplied the rendered value.
    pub source_layer: LayerId,
    /// True when the value came from a layer below the topmost active layer —
    /// i.e. a transparent key resolved through inheritance (ADR 0031).
    pub inherited: bool,
}

/// Resolve every Physical Key in the model under `stack`.
pub fn resolve_layout(model: &KeyboardModel, stack: &LayerStack) -> Vec<ResolvedKey> {
    model
        .physical_layout
        .keys
        .iter()
        .map(|pk| resolve_key(model, stack, &pk.id))
        .collect()
}

/// Resolve a single Physical Key.
///
/// Walks the active layers top-down. The first non-transparent, non-absent
/// entry wins. If every active layer is transparent for this key, the value
/// falls through to the base layer's entry; absent everywhere yields
/// [`SemanticAction::None`].
pub fn resolve_key(model: &KeyboardModel, stack: &LayerStack, key: &KeyId) -> ResolvedKey {
    let geometry = model
        .physical_layout
        .key(key)
        .map(|pk| pk.geometry)
        .unwrap_or_else(|| crate::geometry::KeyGeometry::unit(0.0, 0.0));

    let top_layer = stack.top().map(|a| a.layer.clone());

    let mut chosen: Option<(LayerId, SemanticAction)> = None;
    for active in stack.top_down() {
        let Some(layer) = model.keymap.layer(&active.layer) else {
            continue;
        };
        match layer.entry(key) {
            Some(entry) if !entry.semantic.is_transparent() => {
                chosen = Some((active.layer.clone(), entry.semantic.clone()));
                break;
            }
            // Transparent or absent: keep walking down the stack.
            _ => {}
        }
    }

    // Fall through to the base layer if the stack was all-transparent.
    if chosen.is_none() {
        if let Some(base_id) = model.base_layer() {
            if let Some(entry) = model
                .keymap
                .layer(&base_id)
                .and_then(|layer| layer.entry(key))
            {
                if !entry.semantic.is_transparent() {
                    chosen = Some((base_id, entry.semantic.clone()));
                }
            }
        }
    }

    let (source_layer, effective) = chosen.unwrap_or_else(|| {
        let base = model
            .base_layer()
            .unwrap_or_else(|| LayerId::new("__none__"));
        (base, SemanticAction::None)
    });

    let inherited = top_layer
        .as_ref()
        .map(|top| top != &source_layer)
        .unwrap_or(false);

    let legend = legend_for(&effective);

    ResolvedKey {
        key: key.clone(),
        geometry,
        effective,
        legend,
        source_layer,
        inherited,
    }
}

/// Build a structured [`DisplayLegend`] from a Semantic Action.
pub fn legend_for(action: &SemanticAction) -> DisplayLegend {
    use crate::action::LayerSwitch;
    let mut legend = DisplayLegend::default();
    match action {
        SemanticAction::Key { label } => legend.primary = Some(label.clone()),
        SemanticAction::Modifier { label } => {
            legend.primary = Some(label.clone());
            legend.action = Some("mod".to_string());
        }
        SemanticAction::Layer { switch, layer, tap } => {
            legend.layer = Some(layer.to_string());
            legend.action = Some(
                match switch {
                    LayerSwitch::Momentary => "mo",
                    LayerSwitch::Toggle => "tg",
                    LayerSwitch::Tap => "lt",
                    LayerSwitch::OneShot => "osl",
                    LayerSwitch::Default => "df",
                }
                .to_string(),
            );
            if let Some(tap) = tap {
                legend.tap = Some(tap.clone());
                legend.hold = Some(format!("L{layer}"));
            } else {
                legend.primary = Some(format!("L{layer}"));
            }
        }
        SemanticAction::TapHold { tap, hold } => {
            legend.tap = Some(tap.clone());
            legend.hold = Some(hold.clone());
        }
        SemanticAction::Macro { label } => {
            legend.primary = Some(label.clone());
            legend.action = Some("macro".to_string());
        }
        SemanticAction::Mouse { label } => {
            legend.primary = Some(label.clone());
            legend.action = Some("mouse".to_string());
        }
        SemanticAction::Transparent => {}
        SemanticAction::None => {}
        SemanticAction::Unknown { raw } => {
            legend.primary = Some(raw.clone());
            legend.action = Some("?".to_string());
        }
    }
    legend
}
