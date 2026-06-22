//! ZMK `.keymap` file import (devicetree) as Best-Effort Preview.
//!
//! A ZMK `.keymap` file holds the Logical Keymap as devicetree `keymap` node
//! layers, each a `bindings = < &kp A &mo 1 ... >` list. It carries no per-key
//! geometry (that lives in the board `.dtsi`/`.json`), so — like Vial files —
//! the importer builds a Fallback Layout and marks the candidate Best-Effort
//! Preview: a `.keymap` file has no live layer state.
//!
//! Bindings are split on `&` so each `&behavior args...` token survives intact
//! as a Raw Action; the ZMK Semantic Action parser then classifies it. This
//! avoids a per-behavior arity table while keeping the raw form exact.

use super::{ImportCandidate, ImportError, Importer};
use crate::action::RawAction;
use crate::geometry::KeyGeometry;
use crate::ids::{KeyId, LayerId, SourceId};
use crate::model::keymap::{Layer, LayerEntry, LogicalKeymap};
use crate::model::physical::{PhysicalKey, PhysicalLayout};
use crate::model::KeyboardModel;
use crate::profile::SourceRef;
use crate::provenance::{Provenance, SourceKind};
use crate::resolve::semantic;

/// Columns used when laying out a ZMK Fallback Layout grid.
const FALLBACK_COLS: usize = 12;

/// Imports ZMK `.keymap` devicetree files.
pub struct ZmkKeymapImporter {
    id: String,
}

impl Default for ZmkKeymapImporter {
    fn default() -> Self {
        Self {
            id: "zmk-keymap".to_string(),
        }
    }
}

impl ZmkKeymapImporter {
    pub fn new() -> Self {
        Self::default()
    }
}

fn key_id(index: usize) -> KeyId {
    KeyId::new(format!("k{index}"))
}

fn layer_id(index: usize) -> LayerId {
    LayerId::new(format!("layer-{index}"))
}

impl Importer for ZmkKeymapImporter {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Zmk
    }

    fn import(&self, input: &str) -> Result<ImportCandidate, ImportError> {
        let parsed = parse_keymap(input);
        if parsed.is_empty() {
            return Err(ImportError::Unsupported {
                format: "zmk-keymap".to_string(),
                detail: "no keymap layers with `bindings` found".to_string(),
            });
        }

        let source_id = SourceId::new(&self.id);
        let prov = |raw: &str| Provenance::new(source_id.clone(), SourceKind::Zmk).with_raw(raw);
        let resolver = |n: u16| layer_id(n as usize);

        let mut max_keys = 0usize;
        let mut layers = Vec::new();
        for (index, (name, bindings)) in parsed.iter().enumerate() {
            max_keys = max_keys.max(bindings.len());
            let mut layer = Layer::new(layer_id(index), index as u16).with_name(name);
            for (i, binding) in bindings.iter().enumerate() {
                let raw = RawAction::Zmk(binding.clone());
                let semantic = semantic::derive(&raw, &resolver);
                layer
                    .entries
                    .insert(key_id(i), LayerEntry::new(raw, semantic).with_provenance(prov(binding)));
            }
            layers.push(layer);
        }

        // Fallback Layout: a grid wrapping at FALLBACK_COLS columns.
        let keys = (0..max_keys)
            .map(|i| {
                let (row, col) = (i / FALLBACK_COLS, i % FALLBACK_COLS);
                PhysicalKey::new(key_id(i), KeyGeometry::unit(col as f64, row as f64))
                    .with_provenance(prov("zmk-fallback"))
            })
            .collect();
        let physical = PhysicalLayout::new(keys).as_fallback();

        let keymap = LogicalKeymap::new(layers).with_default(layer_id(0));
        let model = KeyboardModel::new(physical, keymap).with_name("ZMK keymap");

        Ok(ImportCandidate {
            source: SourceRef {
                id: source_id,
                kind: SourceKind::Zmk,
                label: Some("ZMK .keymap".to_string()),
            },
            model,
            notes: vec![
                "Best-Effort Preview: a .keymap file has no live layer state.".to_string(),
                "Geometry is a Fallback Layout; real geometry lives in the board files.".to_string(),
            ],
            best_effort_preview: true,
        })
    }
}

/// Parse `(layer-name, bindings)` pairs from a ZMK `.keymap` file.
///
/// Strips comments, finds the `keymap` node, and for each child node with a
/// `bindings = < ... >` block, records the node name and the split bindings.
fn parse_keymap(input: &str) -> Vec<(String, Vec<String>)> {
    let src = strip_comments(input);
    let Some(keymap_body) = node_body(&src, "keymap") else {
        return Vec::new();
    };

    let mut layers = Vec::new();
    let bytes = keymap_body.as_bytes();
    let mut i = 0;
    while let Some(rel) = keymap_body[i..].find("bindings") {
        let pos = i + rel;
        // Find the enclosing node's name: walk back to the previous '{', then
        // read the identifier before it.
        if let (Some(name), Some(bindings)) =
            (node_name_before(keymap_body, pos), bindings_after(keymap_body, pos))
        {
            layers.push((name, bindings));
        }
        // Advance past this `bindings` occurrence.
        i = pos + "bindings".len();
        if i >= bytes.len() {
            break;
        }
    }
    layers
}

/// Return the body (between matching braces) of the first `name {` node.
fn node_body<'a>(src: &'a str, name: &str) -> Option<&'a str> {
    let start = src.find(name)?;
    let brace = src[start..].find('{')? + start;
    let mut depth = 0usize;
    for (offset, ch) in src[brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[brace + 1..brace + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

/// The identifier of the node enclosing `pos` (the layer name), if any.
fn node_name_before(body: &str, pos: usize) -> Option<String> {
    let open = body[..pos].rfind('{')?;
    // The identifier is the last word before `{`.
    let before = body[..open].trim_end();
    let name: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// The split bindings of the `bindings = < ... >` block starting at/after `pos`.
fn bindings_after(body: &str, pos: usize) -> Option<Vec<String>> {
    let lt = body[pos..].find('<')? + pos;
    let gt = body[lt..].find('>')? + lt;
    let inner = &body[lt + 1..gt];
    let bindings: Vec<String> = inner
        .split('&')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("&{}", s.split_whitespace().collect::<Vec<_>>().join(" ")))
        .collect();
    if bindings.is_empty() {
        None
    } else {
        Some(bindings)
    }
}

/// Remove `//` line comments, `/* */` block comments, and `#` preprocessor
/// lines so the brace/angle scanning is not fooled by them.
fn strip_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else if bytes[i] == b'#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}
