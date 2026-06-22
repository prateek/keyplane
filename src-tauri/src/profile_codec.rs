use crate::domain::{
    ActivationKind, BackendHealth, BackendStatus, CapabilityFlag, DisplayLegend, DisplayTargeting,
    HealthState, KeyAction, KeyGeometry, Layer, LegendSlot, LegendSlotKind, LogicalKeymap,
    MatrixPosition, OverlayWindowConfig, PhysicalKey, PhysicalLayout, Profile, RawAction,
    SemanticAction, SemanticActionKind, SentinelKeyBinding, Source, SourceAuthority,
    SourcePrecedenceRule, SourceRef, StateConfidence, StateConfidenceLevel, StyleDensity,
    UserOverride, VisibilityPolicy, VisualStyle,
};
use edn_format::{emit_str, parse_str, Keyword, Value};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ProfileCodecError {
    #[error("profile EDN parse failed: {0}")]
    Parse(String),
    #[error("profile EDN is missing {0}")]
    Missing(&'static str),
    #[error("profile EDN has invalid {0}")]
    Invalid(&'static str),
}

pub fn save_profile(profile: &Profile) -> String {
    emit_str(&profile_to_value(profile))
}

pub fn load_profile(input: &str) -> Result<Profile, ProfileCodecError> {
    let value = parse_str(input).map_err(|err| ProfileCodecError::Parse(err.to_string()))?;
    let map = as_map(&value, "top-level profile map")?;

    Ok(Profile {
        schema_version: as_u32(get(map, "schema", "version")?, ":schema/version")?,
        id: as_string(get(map, "profile", "id")?, ":profile/id")?,
        name: as_string(get(map, "profile", "name")?, ":profile/name")?,
        sources: parse_sources(get(map, "sources", "items")?)?,
        physical_layout: parse_physical_layout(get(map, "keyboard", "physical-layout")?)?,
        keymap: parse_keymap(get(map, "keyboard", "keymap")?)?,
        runtime_backends: parse_backends(get(map, "runtime", "backends")?)?,
        sentinel_keys: match get_optional(map, "runtime", "sentinel-keys") {
            Some(value) => parse_sentinel_keys(value)?,
            None => Vec::new(),
        },
        visual_style: parse_visual_style(get(map, "visual", "style")?)?,
        overlay_window: parse_overlay_window(get(map, "overlay", "window")?)?,
        source_precedence: parse_precedence(get(map, "source", "precedence")?)?,
        user_overrides: parse_user_overrides(get(map, "user", "overrides")?)?,
        source_provenance: parse_source_refs(get(map, "source", "provenance")?)?,
    })
}

fn profile_to_value(profile: &Profile) -> Value {
    map([
        (kw("keyboard", "keymap"), keymap_to_value(&profile.keymap)),
        (
            kw("keyboard", "physical-layout"),
            physical_layout_to_value(&profile.physical_layout),
        ),
        (
            kw("overlay", "window"),
            overlay_window_to_value(&profile.overlay_window),
        ),
        (kw("profile", "id"), Value::from(profile.id.clone())),
        (kw("profile", "name"), Value::from(profile.name.clone())),
        (
            kw("runtime", "backends"),
            vector(profile.runtime_backends.iter().map(backend_to_value)),
        ),
        (
            kw("runtime", "sentinel-keys"),
            vector(profile.sentinel_keys.iter().map(sentinel_key_to_value)),
        ),
        (kw("schema", "version"), Value::from(profile.schema_version)),
        (
            kw("source", "precedence"),
            vector(profile.source_precedence.iter().map(precedence_to_value)),
        ),
        (
            kw("source", "provenance"),
            vector(profile.source_provenance.iter().map(source_ref_to_value)),
        ),
        (
            kw("sources", "items"),
            vector(profile.sources.iter().map(source_to_value)),
        ),
        (
            kw("user", "overrides"),
            vector(profile.user_overrides.iter().map(user_override_to_value)),
        ),
        (
            kw("visual", "style"),
            visual_style_to_value(&profile.visual_style),
        ),
    ])
}

fn source_to_value(source: &Source) -> Value {
    map([
        (
            kw("source", "authority"),
            authority_to_value(&source.authority),
        ),
        (kw("source", "id"), Value::from(source.id.clone())),
        (kw("source", "kind"), Value::from(source.kind.clone())),
        (kw("source", "name"), Value::from(source.name.clone())),
    ])
}

fn physical_layout_to_value(layout: &PhysicalLayout) -> Value {
    map([
        (kw("layout", "fallback?"), Value::from(layout.fallback)),
        (
            kw("layout", "keys"),
            vector(layout.keys.iter().map(physical_key_to_value)),
        ),
    ])
}

fn physical_key_to_value(key: &PhysicalKey) -> Value {
    map([
        (kw("key", "geometry"), geometry_to_value(&key.geometry)),
        (kw("key", "id"), Value::from(key.id.clone())),
        (
            kw("key", "matrix"),
            key.matrix
                .as_ref()
                .map(matrix_to_value)
                .unwrap_or(Value::Nil),
        ),
        (
            kw("source", "provenance"),
            source_ref_to_value(&key.provenance),
        ),
    ])
}

fn geometry_to_value(geometry: &KeyGeometry) -> Value {
    map([
        (
            kw("geometry", "height"),
            Value::from(geometry.height as f64),
        ),
        (
            kw("geometry", "rotation"),
            Value::from(geometry.rotation as f64),
        ),
        (kw("geometry", "width"), Value::from(geometry.width as f64)),
        (kw("geometry", "x"), Value::from(geometry.x as f64)),
        (kw("geometry", "y"), Value::from(geometry.y as f64)),
    ])
}

fn matrix_to_value(matrix: &MatrixPosition) -> Value {
    map([
        (kw("matrix", "col"), Value::from(matrix.col as u32)),
        (kw("matrix", "row"), Value::from(matrix.row as u32)),
    ])
}

fn keymap_to_value(keymap: &LogicalKeymap) -> Value {
    map([(
        kw("keymap", "layers"),
        vector(keymap.layers.iter().map(layer_to_value)),
    )])
}

fn layer_to_value(layer: &Layer) -> Value {
    map([
        (
            kw("layer", "actions"),
            vector(layer.actions.iter().map(action_to_value)),
        ),
        (kw("layer", "id"), Value::from(layer.id.clone())),
        (kw("layer", "name"), Value::from(layer.name.clone())),
    ])
}

fn action_to_value(action: &KeyAction) -> Value {
    map([
        (kw("action", "key-id"), Value::from(action.key_id.clone())),
        (kw("action", "legend"), legend_to_value(&action.legend)),
        (kw("action", "raw"), raw_action_to_value(&action.raw)),
        (
            kw("action", "semantic"),
            semantic_action_to_value(&action.semantic),
        ),
        (
            kw("source", "provenance"),
            source_ref_to_value(&action.provenance),
        ),
    ])
}

fn raw_action_to_value(raw: &RawAction) -> Value {
    map([
        (kw("raw", "dialect"), Value::from(raw.dialect.clone())),
        (kw("raw", "value"), Value::from(raw.value.clone())),
    ])
}

fn semantic_action_to_value(semantic: &SemanticAction) -> Value {
    map([
        (
            kw("semantic", "kind"),
            semantic_kind_to_value(&semantic.kind),
        ),
        (kw("semantic", "label"), Value::from(semantic.label.clone())),
        (
            kw("semantic", "target-layer"),
            semantic
                .target_layer
                .as_ref()
                .map(|target| Value::from(target.clone()))
                .unwrap_or(Value::Nil),
        ),
    ])
}

fn legend_to_value(legend: &DisplayLegend) -> Value {
    map([(
        kw("legend", "slots"),
        vector(legend.slots.iter().map(legend_slot_to_value)),
    )])
}

fn legend_slot_to_value(slot: &LegendSlot) -> Value {
    map([
        (kw("legend", "slot"), legend_slot_to_value_kind(&slot.slot)),
        (kw("legend", "text"), Value::from(slot.text.clone())),
    ])
}

fn backend_to_value(backend: &BackendStatus) -> Value {
    map([
        (
            kw("backend", "capabilities"),
            vector(backend.capabilities.iter().map(capability_to_value)),
        ),
        (
            kw("backend", "health"),
            backend_health_to_value(&backend.health),
        ),
        (kw("backend", "id"), Value::from(backend.id.clone())),
        (kw("backend", "name"), Value::from(backend.name.clone())),
    ])
}

fn backend_health_to_value(health: &BackendHealth) -> Value {
    map([
        (kw("backend", "id"), Value::from(health.backend_id.clone())),
        (kw("health", "message"), Value::from(health.message.clone())),
        (kw("health", "state"), health_state_to_value(&health.state)),
    ])
}

fn sentinel_key_to_value(binding: &SentinelKeyBinding) -> Value {
    map([
        (
            kw("sentinel", "activation"),
            activation_kind_to_value(&binding.activation),
        ),
        (
            kw("sentinel", "host-input-code"),
            Value::from(binding.host_input_code.clone()),
        ),
        (
            kw("sentinel", "layer-id"),
            Value::from(binding.layer_id.clone()),
        ),
    ])
}

fn visual_style_to_value(style: &VisualStyle) -> Value {
    map([
        (
            kw("style", "density"),
            style_density_to_value(&style.density),
        ),
        (
            kw("style", "variant-id"),
            Value::from(style.variant_id.clone()),
        ),
    ])
}

fn overlay_window_to_value(overlay: &OverlayWindowConfig) -> Value {
    map([
        (
            kw("overlay", "click-through?"),
            Value::from(overlay.click_through),
        ),
        (
            kw("overlay", "display-targeting"),
            display_targeting_to_value(&overlay.display_targeting),
        ),
        (
            kw("overlay", "positioning-mode?"),
            Value::from(overlay.positioning_mode),
        ),
        (
            kw("overlay", "visibility"),
            visibility_policy_to_value(&overlay.visibility),
        ),
    ])
}

fn display_targeting_to_value(targeting: &DisplayTargeting) -> Value {
    map([
        (
            kw("display", "id"),
            targeting
                .display_id
                .as_ref()
                .map(|id| Value::from(id.clone()))
                .unwrap_or(Value::Nil),
        ),
        (
            kw("display", "height"),
            Value::from(targeting.height as f64),
        ),
        (
            kw("display", "opacity"),
            Value::from(targeting.opacity as f64),
        ),
        (kw("display", "width"), Value::from(targeting.width as f64)),
        (kw("display", "x"), Value::from(targeting.x as f64)),
        (kw("display", "y"), Value::from(targeting.y as f64)),
    ])
}

fn precedence_to_value(precedence: &SourcePrecedenceRule) -> Value {
    map([
        (
            kw("precedence", "field-scope"),
            Value::from(precedence.field_scope.clone()),
        ),
        (
            kw("precedence", "source-order"),
            vector(precedence.source_order.iter().cloned().map(Value::from)),
        ),
    ])
}

fn user_override_to_value(user_override: &UserOverride) -> Value {
    map([
        (
            kw("override", "field-path"),
            Value::from(user_override.field_path.clone()),
        ),
        (
            kw("override", "reason"),
            Value::from(user_override.reason.clone()),
        ),
        (
            kw("override", "value"),
            Value::from(user_override.value.clone()),
        ),
    ])
}

fn source_ref_to_value(source_ref: &SourceRef) -> Value {
    map([
        (
            kw("source", "field-path"),
            Value::from(source_ref.field_path.clone()),
        ),
        (
            kw("source", "raw"),
            source_ref
                .raw
                .as_ref()
                .map(|raw| Value::from(raw.clone()))
                .unwrap_or(Value::Nil),
        ),
        (
            kw("source", "id"),
            Value::from(source_ref.source_id.clone()),
        ),
    ])
}

fn parse_sources(value: &Value) -> Result<Vec<Source>, ProfileCodecError> {
    as_vector(value, ":sources/items")?
        .iter()
        .map(|item| {
            let map = as_map(item, ":sources/items entry")?;
            Ok(Source {
                id: as_string(get(map, "source", "id")?, ":source/id")?,
                name: as_string(get(map, "source", "name")?, ":source/name")?,
                kind: as_string(get(map, "source", "kind")?, ":source/kind")?,
                authority: parse_authority(get(map, "source", "authority")?)?,
            })
        })
        .collect()
}

fn parse_physical_layout(value: &Value) -> Result<PhysicalLayout, ProfileCodecError> {
    let map = as_map(value, ":keyboard/physical-layout")?;
    Ok(PhysicalLayout {
        fallback: as_bool(get(map, "layout", "fallback?")?, ":layout/fallback?")?,
        keys: as_vector(get(map, "layout", "keys")?, ":layout/keys")?
            .iter()
            .map(parse_physical_key)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_physical_key(value: &Value) -> Result<PhysicalKey, ProfileCodecError> {
    let map = as_map(value, ":layout/keys entry")?;
    Ok(PhysicalKey {
        id: as_string(get(map, "key", "id")?, ":key/id")?,
        matrix: match get(map, "key", "matrix")? {
            Value::Nil => None,
            value => Some(parse_matrix(value)?),
        },
        geometry: parse_geometry(get(map, "key", "geometry")?)?,
        provenance: parse_source_ref(get(map, "source", "provenance")?)?,
    })
}

fn parse_matrix(value: &Value) -> Result<MatrixPosition, ProfileCodecError> {
    let map = as_map(value, ":key/matrix")?;
    Ok(MatrixPosition {
        row: as_u32(get(map, "matrix", "row")?, ":matrix/row")? as u16,
        col: as_u32(get(map, "matrix", "col")?, ":matrix/col")? as u16,
    })
}

fn parse_geometry(value: &Value) -> Result<KeyGeometry, ProfileCodecError> {
    let map = as_map(value, ":key/geometry")?;
    Ok(KeyGeometry {
        x: as_f32(get(map, "geometry", "x")?, ":geometry/x")?,
        y: as_f32(get(map, "geometry", "y")?, ":geometry/y")?,
        width: as_f32(get(map, "geometry", "width")?, ":geometry/width")?,
        height: as_f32(get(map, "geometry", "height")?, ":geometry/height")?,
        rotation: as_f32(get(map, "geometry", "rotation")?, ":geometry/rotation")?,
    })
}

fn parse_keymap(value: &Value) -> Result<LogicalKeymap, ProfileCodecError> {
    let map = as_map(value, ":keyboard/keymap")?;
    Ok(LogicalKeymap {
        layers: as_vector(get(map, "keymap", "layers")?, ":keymap/layers")?
            .iter()
            .map(parse_layer)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_layer(value: &Value) -> Result<Layer, ProfileCodecError> {
    let map = as_map(value, ":keymap/layers entry")?;
    Ok(Layer {
        id: as_string(get(map, "layer", "id")?, ":layer/id")?,
        name: as_string(get(map, "layer", "name")?, ":layer/name")?,
        actions: as_vector(get(map, "layer", "actions")?, ":layer/actions")?
            .iter()
            .map(parse_action)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_action(value: &Value) -> Result<KeyAction, ProfileCodecError> {
    let map = as_map(value, ":layer/actions entry")?;
    Ok(KeyAction {
        key_id: as_string(get(map, "action", "key-id")?, ":action/key-id")?,
        raw: parse_raw_action(get(map, "action", "raw")?)?,
        semantic: parse_semantic_action(get(map, "action", "semantic")?)?,
        legend: parse_legend(get(map, "action", "legend")?)?,
        provenance: parse_source_ref(get(map, "source", "provenance")?)?,
    })
}

fn parse_raw_action(value: &Value) -> Result<RawAction, ProfileCodecError> {
    let map = as_map(value, ":action/raw")?;
    Ok(RawAction {
        dialect: as_string(get(map, "raw", "dialect")?, ":raw/dialect")?,
        value: as_string(get(map, "raw", "value")?, ":raw/value")?,
    })
}

fn parse_semantic_action(value: &Value) -> Result<SemanticAction, ProfileCodecError> {
    let map = as_map(value, ":action/semantic")?;
    Ok(SemanticAction {
        kind: parse_semantic_kind(get(map, "semantic", "kind")?)?,
        label: as_string(get(map, "semantic", "label")?, ":semantic/label")?,
        target_layer: match get(map, "semantic", "target-layer")? {
            Value::Nil => None,
            value => Some(as_string(value, ":semantic/target-layer")?),
        },
    })
}

fn parse_legend(value: &Value) -> Result<DisplayLegend, ProfileCodecError> {
    let map = as_map(value, ":action/legend")?;
    Ok(DisplayLegend {
        slots: as_vector(get(map, "legend", "slots")?, ":legend/slots")?
            .iter()
            .map(parse_legend_slot)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_legend_slot(value: &Value) -> Result<LegendSlot, ProfileCodecError> {
    let map = as_map(value, ":legend/slots entry")?;
    Ok(LegendSlot {
        slot: parse_legend_slot_kind(get(map, "legend", "slot")?)?,
        text: as_string(get(map, "legend", "text")?, ":legend/text")?,
    })
}

fn parse_backends(value: &Value) -> Result<Vec<BackendStatus>, ProfileCodecError> {
    as_vector(value, ":runtime/backends")?
        .iter()
        .map(|item| {
            let map = as_map(item, ":runtime/backends entry")?;
            Ok(BackendStatus {
                id: as_string(get(map, "backend", "id")?, ":backend/id")?,
                name: as_string(get(map, "backend", "name")?, ":backend/name")?,
                capabilities: as_vector(
                    get(map, "backend", "capabilities")?,
                    ":backend/capabilities",
                )?
                .iter()
                .map(parse_capability)
                .collect::<Result<Vec<_>, _>>()?,
                health: parse_backend_health(get(map, "backend", "health")?)?,
            })
        })
        .collect()
}

fn parse_backend_health(value: &Value) -> Result<BackendHealth, ProfileCodecError> {
    let map = as_map(value, ":backend/health")?;
    Ok(BackendHealth {
        backend_id: as_string(get(map, "backend", "id")?, ":backend/id")?,
        state: parse_health_state(get(map, "health", "state")?)?,
        message: as_string(get(map, "health", "message")?, ":health/message")?,
    })
}

fn parse_sentinel_keys(value: &Value) -> Result<Vec<SentinelKeyBinding>, ProfileCodecError> {
    as_vector(value, ":runtime/sentinel-keys")?
        .iter()
        .map(|item| {
            let map = as_map(item, ":runtime/sentinel-keys entry")?;
            Ok(SentinelKeyBinding {
                host_input_code: as_string(
                    get(map, "sentinel", "host-input-code")?,
                    ":sentinel/host-input-code",
                )?,
                layer_id: as_string(get(map, "sentinel", "layer-id")?, ":sentinel/layer-id")?,
                activation: parse_activation_kind(get(map, "sentinel", "activation")?)?,
            })
        })
        .collect()
}

fn parse_visual_style(value: &Value) -> Result<VisualStyle, ProfileCodecError> {
    let map = as_map(value, ":visual/style")?;
    Ok(VisualStyle {
        variant_id: as_string(get(map, "style", "variant-id")?, ":style/variant-id")?,
        density: parse_style_density(get(map, "style", "density")?)?,
    })
}

fn parse_overlay_window(value: &Value) -> Result<OverlayWindowConfig, ProfileCodecError> {
    let map = as_map(value, ":overlay/window")?;
    Ok(OverlayWindowConfig {
        visibility: parse_visibility_policy(get(map, "overlay", "visibility")?)?,
        click_through: as_bool(
            get(map, "overlay", "click-through?")?,
            ":overlay/click-through?",
        )?,
        positioning_mode: as_bool(
            get(map, "overlay", "positioning-mode?")?,
            ":overlay/positioning-mode?",
        )?,
        display_targeting: parse_display_targeting(get(map, "overlay", "display-targeting")?)?,
    })
}

fn parse_display_targeting(value: &Value) -> Result<DisplayTargeting, ProfileCodecError> {
    let map = as_map(value, ":overlay/display-targeting")?;
    Ok(DisplayTargeting {
        display_id: match get(map, "display", "id")? {
            Value::Nil => None,
            value => Some(as_string(value, ":display/id")?),
        },
        x: as_f32(get(map, "display", "x")?, ":display/x")?,
        y: as_f32(get(map, "display", "y")?, ":display/y")?,
        width: as_f32(get(map, "display", "width")?, ":display/width")?,
        height: as_f32(get(map, "display", "height")?, ":display/height")?,
        opacity: as_f32(get(map, "display", "opacity")?, ":display/opacity")?,
    })
}

fn parse_precedence(value: &Value) -> Result<Vec<SourcePrecedenceRule>, ProfileCodecError> {
    as_vector(value, ":source/precedence")?
        .iter()
        .map(|item| {
            let map = as_map(item, ":source/precedence entry")?;
            Ok(SourcePrecedenceRule {
                field_scope: as_string(
                    get(map, "precedence", "field-scope")?,
                    ":precedence/field-scope",
                )?,
                source_order: as_vector(
                    get(map, "precedence", "source-order")?,
                    ":precedence/source-order",
                )?
                .iter()
                .map(|item| as_string(item, ":precedence/source-order item"))
                .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect()
}

fn parse_user_overrides(value: &Value) -> Result<Vec<UserOverride>, ProfileCodecError> {
    as_vector(value, ":user/overrides")?
        .iter()
        .map(|item| {
            let map = as_map(item, ":user/overrides entry")?;
            Ok(UserOverride {
                field_path: as_string(get(map, "override", "field-path")?, ":override/field-path")?,
                value: as_string(get(map, "override", "value")?, ":override/value")?,
                reason: as_string(get(map, "override", "reason")?, ":override/reason")?,
            })
        })
        .collect()
}

fn parse_source_refs(value: &Value) -> Result<Vec<SourceRef>, ProfileCodecError> {
    as_vector(value, ":source/provenance")?
        .iter()
        .map(parse_source_ref)
        .collect()
}

fn parse_source_ref(value: &Value) -> Result<SourceRef, ProfileCodecError> {
    let map = as_map(value, ":source/provenance entry")?;
    Ok(SourceRef {
        source_id: as_string(get(map, "source", "id")?, ":source/id")?,
        field_path: as_string(get(map, "source", "field-path")?, ":source/field-path")?,
        raw: match get(map, "source", "raw")? {
            Value::Nil => None,
            value => Some(as_string(value, ":source/raw")?),
        },
    })
}

fn map<const N: usize>(entries: [(Value, Value); N]) -> Value {
    Value::Map(BTreeMap::from(entries))
}

fn vector(values: impl Iterator<Item = Value>) -> Value {
    Value::Vector(values.collect())
}

fn kw(namespace: &str, name: &str) -> Value {
    Value::from(Keyword::from_namespace_and_name(namespace, name))
}

fn enum_kw(namespace: &str, name: &str) -> Value {
    kw(namespace, name)
}

fn get<'a>(
    map: &'a BTreeMap<Value, Value>,
    namespace: &'static str,
    name: &'static str,
) -> Result<&'a Value, ProfileCodecError> {
    map.get(&kw(namespace, name))
        .ok_or(ProfileCodecError::Missing(match (namespace, name) {
            ("schema", "version") => ":schema/version",
            ("profile", "id") => ":profile/id",
            ("profile", "name") => ":profile/name",
            ("sources", "items") => ":sources/items",
            ("keyboard", "physical-layout") => ":keyboard/physical-layout",
            ("keyboard", "keymap") => ":keyboard/keymap",
            ("runtime", "backends") => ":runtime/backends",
            ("runtime", "sentinel-keys") => ":runtime/sentinel-keys",
            ("visual", "style") => ":visual/style",
            ("overlay", "window") => ":overlay/window",
            ("source", "precedence") => ":source/precedence",
            ("user", "overrides") => ":user/overrides",
            ("source", "provenance") => ":source/provenance",
            _ => "nested profile field",
        }))
}

fn get_optional<'a>(
    map: &'a BTreeMap<Value, Value>,
    namespace: &'static str,
    name: &'static str,
) -> Option<&'a Value> {
    map.get(&kw(namespace, name))
}

fn as_map<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<&'a BTreeMap<Value, Value>, ProfileCodecError> {
    match value {
        Value::Map(map) => Ok(map),
        _ => Err(ProfileCodecError::Invalid(field)),
    }
}

fn as_vector<'a>(value: &'a Value, field: &'static str) -> Result<&'a [Value], ProfileCodecError> {
    match value {
        Value::Vector(items) => Ok(items),
        _ => Err(ProfileCodecError::Invalid(field)),
    }
}

fn as_string(value: &Value, field: &'static str) -> Result<String, ProfileCodecError> {
    match value {
        Value::String(text) => Ok(text.clone()),
        _ => Err(ProfileCodecError::Invalid(field)),
    }
}

fn as_bool(value: &Value, field: &'static str) -> Result<bool, ProfileCodecError> {
    match value {
        Value::Boolean(value) => Ok(*value),
        _ => Err(ProfileCodecError::Invalid(field)),
    }
}

fn as_u32(value: &Value, field: &'static str) -> Result<u32, ProfileCodecError> {
    match value {
        Value::Integer(value) if *value >= 0 => Ok(*value as u32),
        _ => Err(ProfileCodecError::Invalid(field)),
    }
}

fn as_f32(value: &Value, field: &'static str) -> Result<f32, ProfileCodecError> {
    match value {
        Value::Float(value) => Ok(value.into_inner() as f32),
        Value::Integer(value) => Ok(*value as f32),
        _ => Err(ProfileCodecError::Invalid(field)),
    }
}

fn enum_name(value: &Value, namespace: &'static str) -> Result<String, ProfileCodecError> {
    match value {
        Value::Keyword(keyword) if keyword.namespace() == Some(namespace) => {
            Ok(keyword.name().to_string())
        }
        _ => Err(ProfileCodecError::Invalid("enum keyword")),
    }
}

fn authority_to_value(value: &SourceAuthority) -> Value {
    enum_kw(
        "authority",
        match value {
            SourceAuthority::Authoritative => "authoritative",
            SourceAuthority::BestEffortPreview => "best-effort-preview",
            SourceAuthority::Inferred => "inferred",
        },
    )
}

fn parse_authority(value: &Value) -> Result<SourceAuthority, ProfileCodecError> {
    match enum_name(value, "authority")?.as_str() {
        "authoritative" => Ok(SourceAuthority::Authoritative),
        "best-effort-preview" => Ok(SourceAuthority::BestEffortPreview),
        "inferred" => Ok(SourceAuthority::Inferred),
        _ => Err(ProfileCodecError::Invalid(":source/authority")),
    }
}

fn semantic_kind_to_value(value: &SemanticActionKind) -> Value {
    enum_kw(
        "semantic",
        match value {
            SemanticActionKind::Key => "key",
            SemanticActionKind::Modifier => "modifier",
            SemanticActionKind::LayerMomentary => "layer-momentary",
            SemanticActionKind::LayerToggle => "layer-toggle",
            SemanticActionKind::LayerTap => "layer-tap",
            SemanticActionKind::TapHold => "tap-hold",
            SemanticActionKind::Macro => "macro",
            SemanticActionKind::Transparent => "transparent",
            SemanticActionKind::None => "none",
            SemanticActionKind::Mouse => "mouse",
            SemanticActionKind::Unknown => "unknown",
        },
    )
}

fn parse_semantic_kind(value: &Value) -> Result<SemanticActionKind, ProfileCodecError> {
    match enum_name(value, "semantic")?.as_str() {
        "key" => Ok(SemanticActionKind::Key),
        "modifier" => Ok(SemanticActionKind::Modifier),
        "layer-momentary" => Ok(SemanticActionKind::LayerMomentary),
        "layer-toggle" => Ok(SemanticActionKind::LayerToggle),
        "layer-tap" => Ok(SemanticActionKind::LayerTap),
        "tap-hold" => Ok(SemanticActionKind::TapHold),
        "macro" => Ok(SemanticActionKind::Macro),
        "transparent" => Ok(SemanticActionKind::Transparent),
        "none" => Ok(SemanticActionKind::None),
        "mouse" => Ok(SemanticActionKind::Mouse),
        "unknown" => Ok(SemanticActionKind::Unknown),
        _ => Err(ProfileCodecError::Invalid(":semantic/kind")),
    }
}

fn legend_slot_to_value_kind(value: &LegendSlotKind) -> Value {
    enum_kw(
        "legend-slot",
        match value {
            LegendSlotKind::Primary => "primary",
            LegendSlotKind::Shifted => "shifted",
            LegendSlotKind::TapRole => "tap-role",
            LegendSlotKind::HoldRole => "hold-role",
            LegendSlotKind::LayerHint => "layer-hint",
            LegendSlotKind::ActionHint => "action-hint",
            LegendSlotKind::Icon => "icon",
        },
    )
}

fn parse_legend_slot_kind(value: &Value) -> Result<LegendSlotKind, ProfileCodecError> {
    match enum_name(value, "legend-slot")?.as_str() {
        "primary" => Ok(LegendSlotKind::Primary),
        "shifted" => Ok(LegendSlotKind::Shifted),
        "tap-role" => Ok(LegendSlotKind::TapRole),
        "hold-role" => Ok(LegendSlotKind::HoldRole),
        "layer-hint" => Ok(LegendSlotKind::LayerHint),
        "action-hint" => Ok(LegendSlotKind::ActionHint),
        "icon" => Ok(LegendSlotKind::Icon),
        _ => Err(ProfileCodecError::Invalid(":legend/slot")),
    }
}

fn capability_to_value(value: &CapabilityFlag) -> Value {
    enum_kw(
        "capability",
        match value {
            CapabilityFlag::DiscoverDevices => "discover-devices",
            CapabilityFlag::ImportGeometry => "import-geometry",
            CapabilityFlag::ImportKeymaps => "import-keymaps",
            CapabilityFlag::StreamLayerStack => "stream-layer-stack",
            CapabilityFlag::StreamPressedKeys => "stream-pressed-keys",
            CapabilityFlag::PollState => "poll-state",
            CapabilityFlag::PreviewOnly => "preview-only",
        },
    )
}

fn parse_capability(value: &Value) -> Result<CapabilityFlag, ProfileCodecError> {
    match enum_name(value, "capability")?.as_str() {
        "discover-devices" => Ok(CapabilityFlag::DiscoverDevices),
        "import-geometry" => Ok(CapabilityFlag::ImportGeometry),
        "import-keymaps" => Ok(CapabilityFlag::ImportKeymaps),
        "stream-layer-stack" => Ok(CapabilityFlag::StreamLayerStack),
        "stream-pressed-keys" => Ok(CapabilityFlag::StreamPressedKeys),
        "poll-state" => Ok(CapabilityFlag::PollState),
        "preview-only" => Ok(CapabilityFlag::PreviewOnly),
        _ => Err(ProfileCodecError::Invalid(":backend/capabilities")),
    }
}

fn health_state_to_value(value: &HealthState) -> Value {
    enum_kw(
        "health",
        match value {
            HealthState::Ok => "ok",
            HealthState::PermissionMissing => "permission-missing",
            HealthState::Disconnected => "disconnected",
            HealthState::Stale => "stale",
            HealthState::Unsupported => "unsupported",
            HealthState::ParseError => "parse-error",
            HealthState::ProtocolError => "protocol-error",
        },
    )
}

fn parse_health_state(value: &Value) -> Result<HealthState, ProfileCodecError> {
    match enum_name(value, "health")?.as_str() {
        "ok" => Ok(HealthState::Ok),
        "permission-missing" => Ok(HealthState::PermissionMissing),
        "disconnected" => Ok(HealthState::Disconnected),
        "stale" => Ok(HealthState::Stale),
        "unsupported" => Ok(HealthState::Unsupported),
        "parse-error" => Ok(HealthState::ParseError),
        "protocol-error" => Ok(HealthState::ProtocolError),
        _ => Err(ProfileCodecError::Invalid(":health/state")),
    }
}

fn style_density_to_value(value: &StyleDensity) -> Value {
    enum_kw(
        "style-density",
        match value {
            StyleDensity::Compact => "compact",
            StyleDensity::Standard => "standard",
            StyleDensity::Rich => "rich",
        },
    )
}

fn parse_style_density(value: &Value) -> Result<StyleDensity, ProfileCodecError> {
    match enum_name(value, "style-density")?.as_str() {
        "compact" => Ok(StyleDensity::Compact),
        "standard" => Ok(StyleDensity::Standard),
        "rich" => Ok(StyleDensity::Rich),
        _ => Err(ProfileCodecError::Invalid(":style/density")),
    }
}

fn visibility_policy_to_value(value: &VisibilityPolicy) -> Value {
    enum_kw(
        "visibility",
        match value {
            VisibilityPolicy::Pinned => "pinned",
            VisibilityPolicy::ManualToggle => "manual-toggle",
            VisibilityPolicy::Fade => "fade",
        },
    )
}

fn parse_visibility_policy(value: &Value) -> Result<VisibilityPolicy, ProfileCodecError> {
    match enum_name(value, "visibility")?.as_str() {
        "pinned" => Ok(VisibilityPolicy::Pinned),
        "manual-toggle" => Ok(VisibilityPolicy::ManualToggle),
        "fade" => Ok(VisibilityPolicy::Fade),
        _ => Err(ProfileCodecError::Invalid(":overlay/visibility")),
    }
}

fn activation_kind_to_value(value: &ActivationKind) -> Value {
    enum_kw(
        "activation",
        match value {
            ActivationKind::Default => "default",
            ActivationKind::Momentary => "momentary",
            ActivationKind::Toggle => "toggle",
            ActivationKind::TapHold => "tap-hold",
            ActivationKind::Lock => "lock",
            ActivationKind::RemapperState => "remapper-state",
            ActivationKind::Unknown => "unknown",
        },
    )
}

fn parse_activation_kind(value: &Value) -> Result<ActivationKind, ProfileCodecError> {
    match enum_name(value, "activation")?.as_str() {
        "default" => Ok(ActivationKind::Default),
        "momentary" => Ok(ActivationKind::Momentary),
        "toggle" => Ok(ActivationKind::Toggle),
        "tap-hold" => Ok(ActivationKind::TapHold),
        "lock" => Ok(ActivationKind::Lock),
        "remapper-state" => Ok(ActivationKind::RemapperState),
        "unknown" => Ok(ActivationKind::Unknown),
        _ => Err(ProfileCodecError::Invalid(":sentinel/activation")),
    }
}

#[allow(dead_code)]
fn confidence_to_value(value: &StateConfidence) -> Value {
    map([
        (
            kw("confidence", "level"),
            enum_kw(
                "confidence-level",
                match value.level {
                    StateConfidenceLevel::High => "high",
                    StateConfidenceLevel::Medium => "medium",
                    StateConfidenceLevel::Low => "low",
                },
            ),
        ),
        (
            kw("confidence", "reason"),
            Value::from(value.reason.clone()),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake_backend::fake_profile;

    #[test]
    fn profile_codec_round_trips_the_public_edn_contract() {
        let profile = fake_profile();
        let saved = save_profile(&profile);
        let loaded = load_profile(&saved).expect("profile should load");

        assert_eq!(loaded.schema_version, 1);
        assert_eq!(loaded.id, profile.id);
        assert_eq!(
            loaded.physical_layout.keys.len(),
            profile.physical_layout.keys.len()
        );
        assert_eq!(loaded.keymap.layers[1].actions[1].raw.value, "KC_TRNS");
        assert_eq!(loaded.sentinel_keys.len(), 1);
        assert_eq!(loaded.sentinel_keys[0].host_input_code, "F24");
        assert!(saved.contains(":schema/version"));
        assert!(saved.contains(":keyboard/physical-layout"));
        assert!(saved.contains(":runtime/sentinel-keys"));
        assert!(saved.contains(":source/provenance"));
    }

    #[test]
    fn profile_codec_is_deterministic_on_save() {
        let profile = fake_profile();

        assert_eq!(save_profile(&profile), save_profile(&profile));
    }

    #[test]
    fn profile_codec_loads_profiles_without_sentinel_keys_as_empty_bindings() {
        let profile = fake_profile();
        let Value::Map(mut map) = profile_to_value(&profile) else {
            panic!("profile should serialize as map");
        };
        map.remove(&kw("runtime", "sentinel-keys"));
        let saved_without_sentinel_keys = emit_str(&Value::Map(map));

        let loaded =
            load_profile(&saved_without_sentinel_keys).expect("legacy profile should load");

        assert!(loaded.sentinel_keys.is_empty());
    }
}
