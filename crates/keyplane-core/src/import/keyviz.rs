//! keyviz style JSON import (ADR 0013, 0018).
//!
//! keyviz compatibility means importing its style JSON into the
//! [`VisualStyle`](crate::model::style::VisualStyle) model only. Per Source
//! Precedence (ADR 0018) keyviz affects Visual Style fields and nothing else, so
//! the candidate carries no Physical Layout or Keymap — only style.

use super::{ImportCandidate, ImportError, Importer};
use crate::ids::SourceId;
use crate::model::style::{StyleVariant, VisualStyle};
use crate::model::KeyboardModel;
use crate::profile::SourceRef;
use crate::provenance::SourceKind;
use serde_json::Value as JsonValue;

/// Imports keyviz style JSON into Visual Style fields.
pub struct KeyvizStyleImporter {
    id: String,
}

impl Default for KeyvizStyleImporter {
    fn default() -> Self {
        Self {
            id: "keyviz-style".to_string(),
        }
    }
}

impl KeyvizStyleImporter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Importer for KeyvizStyleImporter {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Keyviz
    }

    fn import(&self, input: &str) -> Result<ImportCandidate, ImportError> {
        let json: JsonValue = serde_json::from_str(input).map_err(|e| ImportError::Parse {
            format: "keyviz-style".to_string(),
            detail: e.to_string(),
        })?;

        let mut style = VisualStyle::default();
        // keyviz schemas vary; accept a few plausible field names per concept.
        if let Some(c) = first_str(&json, &["accentColor", "accent", "highlightColor"]) {
            style.accent = Some(c);
        }
        if let Some(c) = first_str(&json, &["keyCapColor", "keycapColor", "backgroundColor"]) {
            style.keycap_color = Some(c);
        }
        if let Some(c) = first_str(&json, &["textColor", "fontColor", "labelColor"]) {
            style.text_color = Some(c);
        }
        if let Some(o) = first_f64(&json, &["opacity", "windowOpacity"]) {
            style.opacity = o.clamp(0.0, 1.0);
        }
        if let Some(v) = first_str(&json, &["density", "style", "variant"]) {
            if v.eq_ignore_ascii_case("minimal") || v.eq_ignore_ascii_case("compact") {
                style.variant = StyleVariant::Minimal;
            }
        }

        let mut model = KeyboardModel::default();
        model.style = style;

        Ok(ImportCandidate {
            source: SourceRef {
                id: SourceId::new(&self.id),
                kind: SourceKind::Keyviz,
                label: Some("keyviz style".to_string()),
            },
            model,
            notes: vec!["Affects Visual Style only; no keyboard data imported.".to_string()],
            best_effort_preview: false,
        })
    }
}

fn first_str(json: &JsonValue, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| json.get(*k).and_then(JsonValue::as_str))
        .map(String::from)
}

fn first_f64(json: &JsonValue, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|k| json.get(*k).and_then(JsonValue::as_f64))
}
