//! EDN ↔ [`Profile`] mapping (ADR 0035, 0042).
//!
//! Encoding emits sections in a fixed canonical order so saves are
//! deterministic. Decoding re-derives Semantic Actions from preserved Raw
//! Actions, keeping the Raw Action the single source of truth on disk.
//!
//! Source Precedence is currently owned in code ([`crate::precedence`]) rather
//! than serialized, and Source Provenance is stored per element rather than as a
//! separate top-level section. The other ADR 0035 sections are modeled here.

use super::{
    BackendConfig, DisplayTargeting, OverlayConfig, Profile, ProfileError, SourceRef, UserOverride,
    VisibilityPolicy, CURRENT_SCHEMA,
};
use crate::action::RawAction;
use crate::geometry::{KeyGeometry, MatrixPosition};
use crate::ids::{KeyId, LayerId, SourceId};
use crate::model::keymap::{Layer, LayerEntry, LogicalKeymap};
use crate::model::physical::{PhysicalKey, PhysicalLayout};
use crate::model::style::{StyleVariant, VisualStyle};
use crate::model::KeyboardModel;
use crate::profile::edn::Edn;
use crate::provenance::{Provenance, SourceKind};
use crate::resolve::semantic;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

fn kw(name: &str) -> Edn {
    Edn::keyword(name)
}

// ---- Encode ----------------------------------------------------------------

/// Encode a Profile into a canonical, ordered [`Edn`] map.
pub fn encode(profile: &Profile) -> Edn {
    let mut sections: Vec<(Edn, Edn)> = Vec::new();
    sections.push((kw("schema/version"), Edn::Int(profile.schema_version as i64)));
    sections.push((kw("profile/id"), Edn::string(&profile.id)));
    if let Some(name) = &profile.name {
        sections.push((kw("profile/name"), Edn::string(name)));
    }
    sections.push((kw("sources"), encode_sources(&profile.sources)));
    sections.push((
        kw("keyboard/physical-layout"),
        encode_physical(&profile.model.physical_layout),
    ));
    sections.push((kw("keyboard/keymap"), encode_keymap(&profile.model.keymap)));
    sections.push((kw("runtime/backends"), encode_backends(&profile.backends)));
    sections.push((kw("visual/style"), encode_style(&profile.model.style)));
    sections.push((kw("overlay/window"), encode_overlay(&profile.overlay)));
    sections.push((
        kw("user/overrides"),
        encode_overrides(&profile.user_overrides),
    ));
    Edn::Map(sections)
}

fn encode_sources(sources: &[SourceRef]) -> Edn {
    Edn::Vector(
        sources
            .iter()
            .map(|s| {
                let mut m = vec![
                    (kw("source/id"), Edn::string(s.id.as_str())),
                    (kw("source/kind"), kw(source_kind_name(s.kind))),
                ];
                if let Some(label) = &s.label {
                    m.push((kw("source/label"), Edn::string(label)));
                }
                Edn::Map(m)
            })
            .collect(),
    )
}

fn encode_backends(backends: &[BackendConfig]) -> Edn {
    Edn::Vector(
        backends
            .iter()
            .map(|b| {
                Edn::Map(vec![
                    (kw("backend/id"), Edn::string(&b.id)),
                    (kw("backend/kind"), kw(source_kind_name(b.kind))),
                    (kw("backend/enabled"), Edn::Bool(b.enabled)),
                ])
            })
            .collect(),
    )
}

fn encode_physical(layout: &PhysicalLayout) -> Edn {
    Edn::Map(vec![
        (kw("layout/fallback"), Edn::Bool(layout.fallback)),
        (
            kw("layout/keys"),
            Edn::Vector(layout.keys.iter().map(encode_physical_key).collect()),
        ),
    ])
}

fn encode_physical_key(key: &PhysicalKey) -> Edn {
    let mut m = vec![(kw("key/id"), Edn::string(key.id.as_str()))];
    if let Some(mx) = &key.matrix {
        m.push((
            kw("key/matrix"),
            Edn::Vector(vec![Edn::Int(mx.row as i64), Edn::Int(mx.col as i64)]),
        ));
    }
    m.push((kw("key/geometry"), encode_geometry(&key.geometry)));
    if let Some(p) = &key.provenance {
        m.push((kw("key/source"), encode_provenance(p)));
    }
    Edn::Map(m)
}

fn encode_geometry(g: &KeyGeometry) -> Edn {
    let mut m = vec![
        (kw("x"), Edn::Float(g.x)),
        (kw("y"), Edn::Float(g.y)),
        (kw("w"), Edn::Float(g.w)),
        (kw("h"), Edn::Float(g.h)),
    ];
    if g.rotation != 0.0 {
        m.push((kw("rotation"), Edn::Float(g.rotation)));
    }
    if let Some((ox, oy)) = g.rotation_origin {
        m.push((
            kw("rotation-origin"),
            Edn::Vector(vec![Edn::Float(ox), Edn::Float(oy)]),
        ));
    }
    Edn::Map(m)
}

fn encode_keymap(keymap: &LogicalKeymap) -> Edn {
    let mut m = Vec::new();
    if let Some(default) = &keymap.default_layer {
        m.push((kw("keymap/default"), Edn::string(default.as_str())));
    }
    m.push((
        kw("keymap/layers"),
        Edn::Vector(keymap.layers.iter().map(encode_layer).collect()),
    ));
    Edn::Map(m)
}

fn encode_layer(layer: &Layer) -> Edn {
    let mut m = vec![
        (kw("layer/id"), Edn::string(layer.id.as_str())),
        (kw("layer/index"), Edn::Int(layer.index as i64)),
    ];
    if let Some(name) = &layer.name {
        m.push((kw("layer/name"), Edn::string(name)));
    }
    // entries is a BTreeMap, so iteration is sorted and deterministic.
    let entries = layer
        .entries
        .iter()
        .map(|(key, entry)| {
            let mut em = vec![
                (kw("entry/key"), Edn::string(key.as_str())),
                (kw("entry/raw"), encode_raw(&entry.raw)),
            ];
            if let Some(p) = &entry.provenance {
                em.push((kw("entry/source"), encode_provenance(p)));
            }
            Edn::Map(em)
        })
        .collect();
    m.push((kw("layer/entries"), Edn::Vector(entries)));
    Edn::Map(m)
}

fn encode_raw(raw: &RawAction) -> Edn {
    let (kind, value) = match raw {
        RawAction::Qmk(s) => ("qmk", Edn::string(s)),
        RawAction::ViaCode(c) => ("via-code", Edn::Int(*c as i64)),
        RawAction::Zmk(s) => ("zmk", Edn::string(s)),
        RawAction::Kanata(s) => ("kanata", Edn::string(s)),
        RawAction::HostEvent(s) => ("host-event", Edn::string(s)),
        RawAction::KeyPeek(s) => ("key-peek", Edn::string(s)),
        RawAction::Opaque(s) => ("opaque", Edn::string(s)),
    };
    Edn::Map(vec![(kw("raw/source"), kw(kind)), (kw("raw/value"), value)])
}

fn encode_provenance(p: &Provenance) -> Edn {
    let mut m = vec![
        (kw("source/id"), Edn::string(p.source.as_str())),
        (kw("source/kind"), kw(source_kind_name(p.kind))),
    ];
    if let Some(raw) = &p.raw {
        m.push((kw("source/raw"), Edn::string(raw)));
    }
    Edn::Map(m)
}

fn encode_style(style: &VisualStyle) -> Edn {
    let mut m = vec![
        (
            kw("style/variant"),
            kw(match style.variant {
                StyleVariant::Detailed => "detailed",
                StyleVariant::Minimal => "minimal",
            }),
        ),
        (
            kw("style/show-inherited"),
            Edn::Bool(style.show_inherited_indicator),
        ),
        (kw("style/opacity"), Edn::Float(style.opacity)),
    ];
    if let Some(c) = &style.accent {
        m.push((kw("style/accent"), Edn::string(c)));
    }
    if let Some(c) = &style.keycap_color {
        m.push((kw("style/keycap-color"), Edn::string(c)));
    }
    if let Some(c) = &style.text_color {
        m.push((kw("style/text-color"), Edn::string(c)));
    }
    Edn::Map(m)
}

fn encode_overlay(overlay: &OverlayConfig) -> Edn {
    Edn::Map(vec![
        (
            kw("overlay/visibility"),
            kw(match overlay.visibility {
                VisibilityPolicy::Pinned => "pinned",
                VisibilityPolicy::ManualToggle => "manual-toggle",
                VisibilityPolicy::Fade => "fade",
            }),
        ),
        (kw("overlay/click-through"), Edn::Bool(overlay.click_through)),
        (kw("overlay/always-on-top"), Edn::Bool(overlay.always_on_top)),
        (kw("overlay/display"), encode_display(&overlay.display)),
    ])
}

fn encode_display(d: &DisplayTargeting) -> Edn {
    let mut m = Vec::new();
    if let Some(v) = &d.monitor {
        m.push((kw("display/monitor"), Edn::string(v)));
    }
    if let Some(v) = d.x {
        m.push((kw("display/x"), Edn::Float(v)));
    }
    if let Some(v) = d.y {
        m.push((kw("display/y"), Edn::Float(v)));
    }
    if let Some(v) = d.width {
        m.push((kw("display/width"), Edn::Float(v)));
    }
    if let Some(v) = d.height {
        m.push((kw("display/height"), Edn::Float(v)));
    }
    Edn::Map(m)
}

fn encode_overrides(overrides: &[UserOverride]) -> Edn {
    Edn::Vector(
        overrides
            .iter()
            .map(|o| {
                let mut m = vec![
                    (kw("override/field"), Edn::string(&o.field)),
                    (kw("override/value"), json_to_edn(&o.value)),
                ];
                if let Some(note) = &o.note {
                    m.push((kw("override/note"), Edn::string(note)));
                }
                Edn::Map(m)
            })
            .collect(),
    )
}

// ---- Decode ----------------------------------------------------------------

/// Decode a Profile from a (migrated) [`Edn`] value.
pub fn decode(edn: &Edn) -> Result<Profile, ProfileError> {
    let schema_version = edn
        .get("schema/version")
        .and_then(Edn::as_i64)
        .ok_or_else(|| ProfileError::schema("missing :schema/version"))? as u32;
    if schema_version != CURRENT_SCHEMA {
        return Err(ProfileError::UnsupportedVersion(schema_version));
    }
    let id = edn
        .get("profile/id")
        .and_then(Edn::as_str)
        .ok_or_else(|| ProfileError::schema("missing :profile/id"))?
        .to_string();
    let name = edn
        .get("profile/name")
        .and_then(Edn::as_str)
        .map(|s| s.to_string());

    let sources = decode_sources(edn.get("sources"))?;
    let physical_layout = decode_physical(edn.get("keyboard/physical-layout"))?;
    let keymap = decode_keymap(edn.get("keyboard/keymap"))?;
    let style = decode_style(edn.get("visual/style"));
    let backends = decode_backends(edn.get("runtime/backends"))?;
    let overlay = decode_overlay(edn.get("overlay/window"));
    let user_overrides = decode_overrides(edn.get("user/overrides"))?;

    let mut model = KeyboardModel::new(physical_layout, keymap);
    model.name = name.clone();
    model.style = style;

    Ok(Profile {
        schema_version,
        id,
        name,
        sources,
        model,
        backends,
        overlay,
        user_overrides,
    })
}

fn decode_sources(edn: Option<&Edn>) -> Result<Vec<SourceRef>, ProfileError> {
    let Some(items) = edn.and_then(Edn::as_vec) else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|item| {
            Ok(SourceRef {
                id: SourceId::new(req_str(item, "source/id")?),
                kind: req_source_kind(item, "source/kind")?,
                label: item.get("source/label").and_then(Edn::as_str).map(String::from),
            })
        })
        .collect()
}

fn decode_backends(edn: Option<&Edn>) -> Result<Vec<BackendConfig>, ProfileError> {
    let Some(items) = edn.and_then(Edn::as_vec) else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|item| {
            Ok(BackendConfig {
                id: req_str(item, "backend/id")?.to_string(),
                kind: req_source_kind(item, "backend/kind")?,
                enabled: item
                    .get("backend/enabled")
                    .and_then(Edn::as_bool)
                    .unwrap_or(true),
            })
        })
        .collect()
}

fn decode_physical(edn: Option<&Edn>) -> Result<PhysicalLayout, ProfileError> {
    let Some(map) = edn else {
        return Ok(PhysicalLayout::default());
    };
    let fallback = map
        .get("layout/fallback")
        .and_then(Edn::as_bool)
        .unwrap_or(false);
    let keys = map
        .get("layout/keys")
        .and_then(Edn::as_vec)
        .map(|items| items.iter().map(decode_physical_key).collect::<Result<Vec<_>, _>>())
        .transpose()?
        .unwrap_or_default();
    Ok(PhysicalLayout { keys, fallback })
}

fn decode_physical_key(edn: &Edn) -> Result<PhysicalKey, ProfileError> {
    let id = KeyId::new(req_str(edn, "key/id")?);
    let matrix = match edn.get("key/matrix").and_then(Edn::as_vec) {
        Some([r, c]) => Some(MatrixPosition::new(
            r.as_i64().unwrap_or(0) as u16,
            c.as_i64().unwrap_or(0) as u16,
        )),
        _ => None,
    };
    let geometry = decode_geometry(edn.get("key/geometry"))?;
    let provenance = edn.get("key/source").map(decode_provenance).transpose()?;
    Ok(PhysicalKey {
        id,
        matrix,
        geometry,
        provenance,
    })
}

fn decode_geometry(edn: Option<&Edn>) -> Result<KeyGeometry, ProfileError> {
    let map = edn.ok_or_else(|| ProfileError::schema("missing :key/geometry"))?;
    let f = |name: &str| map.get(name).and_then(Edn::as_f64);
    let rotation_origin = match map.get("rotation-origin").and_then(Edn::as_vec) {
        Some([ox, oy]) => Some((ox.as_f64().unwrap_or(0.0), oy.as_f64().unwrap_or(0.0))),
        _ => None,
    };
    Ok(KeyGeometry {
        x: f("x").unwrap_or(0.0),
        y: f("y").unwrap_or(0.0),
        w: f("w").unwrap_or(1.0),
        h: f("h").unwrap_or(1.0),
        rotation: f("rotation").unwrap_or(0.0),
        rotation_origin,
    })
}

fn decode_keymap(edn: Option<&Edn>) -> Result<LogicalKeymap, ProfileError> {
    let Some(map) = edn else {
        return Ok(LogicalKeymap::default());
    };
    let default_layer = map
        .get("keymap/default")
        .and_then(Edn::as_str)
        .map(LayerId::new);

    let layer_edns = map.get("keymap/layers").and_then(Edn::as_vec).unwrap_or(&[]);

    // First pass: build an index → LayerId map so semantics resolve layer
    // switches to the right id.
    let mut index_to_id: BTreeMap<u16, LayerId> = BTreeMap::new();
    for layer in layer_edns {
        if let (Some(id), Some(idx)) = (
            layer.get("layer/id").and_then(Edn::as_str),
            layer.get("layer/index").and_then(Edn::as_i64),
        ) {
            index_to_id.insert(idx as u16, LayerId::new(id));
        }
    }
    let resolver = |n: u16| {
        index_to_id
            .get(&n)
            .cloned()
            .unwrap_or_else(|| LayerId::new(format!("layer-{n}")))
    };

    let mut layers = Vec::new();
    for layer_edn in layer_edns {
        layers.push(decode_layer(layer_edn, &resolver)?);
    }
    Ok(LogicalKeymap {
        layers,
        default_layer,
    })
}

fn decode_layer(
    edn: &Edn,
    resolver: &semantic::LayerIndexResolver<'_>,
) -> Result<Layer, ProfileError> {
    let id = LayerId::new(req_str(edn, "layer/id")?);
    let index = edn
        .get("layer/index")
        .and_then(Edn::as_i64)
        .ok_or_else(|| ProfileError::schema("missing :layer/index"))? as u16;
    let name = edn.get("layer/name").and_then(Edn::as_str).map(String::from);
    let mut entries = BTreeMap::new();
    if let Some(items) = edn.get("layer/entries").and_then(Edn::as_vec) {
        for item in items {
            let key = KeyId::new(req_str(item, "entry/key")?);
            let raw = decode_raw(item.get("entry/raw"))?;
            let semantic = semantic::derive(&raw, resolver);
            let provenance = item.get("entry/source").map(decode_provenance).transpose()?;
            entries.insert(
                key,
                LayerEntry {
                    raw,
                    semantic,
                    provenance,
                },
            );
        }
    }
    Ok(Layer {
        id,
        index,
        name,
        entries,
    })
}

fn decode_raw(edn: Option<&Edn>) -> Result<RawAction, ProfileError> {
    let map = edn.ok_or_else(|| ProfileError::schema("missing :entry/raw"))?;
    let kind = map
        .get("raw/source")
        .and_then(Edn::as_keyword)
        .ok_or_else(|| ProfileError::schema("missing :raw/source"))?;
    let value = map.get("raw/value");
    let s = || value.and_then(Edn::as_str).unwrap_or("").to_string();
    Ok(match kind {
        "qmk" => RawAction::Qmk(s()),
        "via-code" => RawAction::ViaCode(value.and_then(Edn::as_i64).unwrap_or(0) as u16),
        "zmk" => RawAction::Zmk(s()),
        "kanata" => RawAction::Kanata(s()),
        "host-event" => RawAction::HostEvent(s()),
        "key-peek" => RawAction::KeyPeek(s()),
        "opaque" => RawAction::Opaque(s()),
        other => return Err(ProfileError::schema(format!("unknown :raw/source {other}"))),
    })
}

fn decode_provenance(edn: &Edn) -> Result<Provenance, ProfileError> {
    Ok(Provenance {
        source: SourceId::new(req_str(edn, "source/id")?),
        kind: req_source_kind(edn, "source/kind")?,
        raw: edn.get("source/raw").and_then(Edn::as_str).map(String::from),
    })
}

fn decode_style(edn: Option<&Edn>) -> VisualStyle {
    let mut style = VisualStyle::default();
    let Some(map) = edn else {
        return style;
    };
    if let Some(v) = map.get("style/variant").and_then(Edn::as_keyword) {
        style.variant = match v {
            "minimal" => StyleVariant::Minimal,
            _ => StyleVariant::Detailed,
        };
    }
    if let Some(v) = map.get("style/show-inherited").and_then(Edn::as_bool) {
        style.show_inherited_indicator = v;
    }
    if let Some(v) = map.get("style/opacity").and_then(Edn::as_f64) {
        style.opacity = v;
    }
    style.accent = map.get("style/accent").and_then(Edn::as_str).map(String::from);
    style.keycap_color = map
        .get("style/keycap-color")
        .and_then(Edn::as_str)
        .map(String::from);
    style.text_color = map
        .get("style/text-color")
        .and_then(Edn::as_str)
        .map(String::from);
    style
}

fn decode_overlay(edn: Option<&Edn>) -> OverlayConfig {
    let mut overlay = OverlayConfig::default();
    let Some(map) = edn else {
        return overlay;
    };
    if let Some(v) = map.get("overlay/visibility").and_then(Edn::as_keyword) {
        overlay.visibility = match v {
            "manual-toggle" => VisibilityPolicy::ManualToggle,
            "fade" => VisibilityPolicy::Fade,
            _ => VisibilityPolicy::Pinned,
        };
    }
    if let Some(v) = map.get("overlay/click-through").and_then(Edn::as_bool) {
        overlay.click_through = v;
    }
    if let Some(v) = map.get("overlay/always-on-top").and_then(Edn::as_bool) {
        overlay.always_on_top = v;
    }
    if let Some(d) = map.get("overlay/display") {
        overlay.display = DisplayTargeting {
            monitor: d.get("display/monitor").and_then(Edn::as_str).map(String::from),
            x: d.get("display/x").and_then(Edn::as_f64),
            y: d.get("display/y").and_then(Edn::as_f64),
            width: d.get("display/width").and_then(Edn::as_f64),
            height: d.get("display/height").and_then(Edn::as_f64),
        };
    }
    overlay
}

fn decode_overrides(edn: Option<&Edn>) -> Result<Vec<UserOverride>, ProfileError> {
    let Some(items) = edn.and_then(Edn::as_vec) else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|item| {
            Ok(UserOverride {
                field: req_str(item, "override/field")?.to_string(),
                value: item
                    .get("override/value")
                    .map(edn_to_json)
                    .unwrap_or(JsonValue::Null),
                note: item.get("override/note").and_then(Edn::as_str).map(String::from),
            })
        })
        .collect()
}

// ---- Helpers ---------------------------------------------------------------

fn req_str<'a>(edn: &'a Edn, key: &str) -> Result<&'a str, ProfileError> {
    edn.get(key)
        .and_then(Edn::as_str)
        .ok_or_else(|| ProfileError::schema(format!("missing :{key}")))
}

fn req_source_kind(edn: &Edn, key: &str) -> Result<SourceKind, ProfileError> {
    let name = edn
        .get(key)
        .and_then(Edn::as_keyword)
        .ok_or_else(|| ProfileError::schema(format!("missing :{key}")))?;
    source_kind_from_name(name)
        .ok_or_else(|| ProfileError::schema(format!("unknown source kind :{name}")))
}

/// Stable keyword name for a [`SourceKind`] in EDN.
pub fn source_kind_name(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::KeyPeek => "key-peek",
        SourceKind::Vial => "vial",
        SourceKind::Via => "via",
        SourceKind::Zmk => "zmk",
        SourceKind::OverKeys => "over-keys",
        SourceKind::Keyviz => "keyviz",
        SourceKind::Kanata => "kanata",
        SourceKind::Sentinel => "sentinel",
        SourceKind::Fake => "fake",
        SourceKind::User => "user",
    }
}

fn source_kind_from_name(name: &str) -> Option<SourceKind> {
    Some(match name {
        "key-peek" => SourceKind::KeyPeek,
        "vial" => SourceKind::Vial,
        "via" => SourceKind::Via,
        "zmk" => SourceKind::Zmk,
        "over-keys" => SourceKind::OverKeys,
        "keyviz" => SourceKind::Keyviz,
        "kanata" => SourceKind::Kanata,
        "sentinel" => SourceKind::Sentinel,
        "fake" => SourceKind::Fake,
        "user" => SourceKind::User,
        _ => return None,
    })
}

// ---- JSON <-> EDN bridge (for free-form user override values) --------------

fn json_to_edn(value: &JsonValue) -> Edn {
    match value {
        JsonValue::Null => Edn::Nil,
        JsonValue::Bool(b) => Edn::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Edn::Int(i)
            } else {
                Edn::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        JsonValue::String(s) => Edn::Str(s.clone()),
        JsonValue::Array(a) => Edn::Vector(a.iter().map(json_to_edn).collect()),
        JsonValue::Object(o) => Edn::Map(
            o.iter()
                .map(|(k, v)| (Edn::Str(k.clone()), json_to_edn(v)))
                .collect(),
        ),
    }
}

fn edn_to_json(value: &Edn) -> JsonValue {
    match value {
        Edn::Nil => JsonValue::Null,
        Edn::Bool(b) => JsonValue::Bool(*b),
        Edn::Int(i) => JsonValue::Number((*i).into()),
        Edn::Float(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        Edn::Str(s) => JsonValue::String(s.clone()),
        Edn::Keyword(k) => JsonValue::String(format!(":{k}")),
        Edn::Vector(v) => JsonValue::Array(v.iter().map(edn_to_json).collect()),
        Edn::Map(m) => {
            let obj = m
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        Edn::Str(s) => s.clone(),
                        Edn::Keyword(s) => s.clone(),
                        other => format!("{other:?}"),
                    };
                    (key, edn_to_json(v))
                })
                .collect();
            JsonValue::Object(obj)
        }
    }
}
