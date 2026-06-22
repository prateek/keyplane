use serde::{Deserialize, Serialize};

const USER_OVERRIDE_SOURCE_ID: &str = "user-overrides";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourceAuthority {
    Authoritative,
    BestEffortPreview,
    Inferred,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub authority: SourceAuthority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRef {
    pub source_id: String,
    pub field_path: String,
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixPosition {
    pub row: u16,
    pub col: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhysicalKey {
    pub id: String,
    pub matrix: Option<MatrixPosition>,
    pub geometry: KeyGeometry,
    pub provenance: SourceRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhysicalLayout {
    pub keys: Vec<PhysicalKey>,
    pub fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawAction {
    pub dialect: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticActionKind {
    Key,
    Modifier,
    LayerMomentary,
    LayerToggle,
    LayerTap,
    TapHold,
    Macro,
    Transparent,
    None,
    Mouse,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticAction {
    pub kind: SemanticActionKind,
    pub label: String,
    pub target_layer: Option<String>,
    pub hold_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LegendSlotKind {
    Primary,
    Shifted,
    TapRole,
    HoldRole,
    LayerHint,
    ActionHint,
    Icon,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegendSlot {
    pub slot: LegendSlotKind,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisplayLegend {
    pub slots: Vec<LegendSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyAction {
    pub key_id: String,
    pub raw: RawAction,
    pub semantic: SemanticAction,
    pub legend: DisplayLegend,
    pub provenance: SourceRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub actions: Vec<KeyAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogicalKeymap {
    pub layers: Vec<Layer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ActivationKind {
    Default,
    Momentary,
    Toggle,
    TapHold,
    Lock,
    RemapperState,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StateConfidenceLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateConfidence {
    pub level: StateConfidenceLevel,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerActivation {
    pub layer_id: String,
    pub kind: ActivationKind,
    pub confidence: StateConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostInputEvent {
    pub code: String,
    pub pressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SentinelKeyBinding {
    pub host_input_code: String,
    pub layer_id: String,
    pub activation: ActivationKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityFlag {
    DiscoverDevices,
    ImportGeometry,
    ImportKeymaps,
    StreamLayerStack,
    StreamPressedKeys,
    PollState,
    PreviewOnly,
    RenderOverlayWindow,
    TransparentOverlayWindow,
    ClickThroughOverlayWindow,
    PositionOverlayWindow,
    AllWorkspacesOverlayWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HealthState {
    Ok,
    PermissionMissing,
    Disconnected,
    Stale,
    Unsupported,
    ParseError,
    ProtocolError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendHealth {
    pub backend_id: String,
    pub state: HealthState,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum BackendConfig {
    KanataTcp { host: String, port: u16 },
}

pub fn default_kanata_tcp_config() -> BackendConfig {
    BackendConfig::KanataTcp {
        host: "127.0.0.1".to_string(),
        port: 7070,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendStatus {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<CapabilityFlag>,
    pub health: BackendHealth,
    pub config: Option<BackendConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyPeekDiscoveredDevice {
    pub vid: String,
    pub pid: String,
    pub usage_page: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyPeekDeviceDiscovery {
    pub devices: Vec<KeyPeekDiscoveredDevice>,
    pub snapshot: KeyboardSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeState {
    pub layer_stack: Vec<LayerActivation>,
    pub pressed_keys: Vec<String>,
    pub backend_health: Vec<BackendHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectiveKey {
    pub key_id: String,
    pub raw: RawAction,
    pub semantic: SemanticAction,
    pub legend: DisplayLegend,
    pub source_layer_id: String,
    pub inherited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceCandidate {
    pub source_id: String,
    pub value: String,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceConflict {
    pub field_path: String,
    pub selected_source_id: String,
    pub candidates: Vec<SourceCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VisibilityPolicy {
    Pinned,
    ManualToggle,
    Fade,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayTargeting {
    pub display_id: Option<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub opacity: f32,
}

pub fn global_display_fallback() -> DisplayTargeting {
    DisplayTargeting {
        display_id: None,
        x: 72.0,
        y: 72.0,
        width: 940.0,
        height: 320.0,
        opacity: 0.92,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayWindowConfig {
    pub visibility: VisibilityPolicy,
    pub visible: bool,
    pub click_through: bool,
    pub positioning_mode: bool,
    pub display_targeting: DisplayTargeting,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StyleDensity {
    Compact,
    Standard,
    Rich,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualStyle {
    pub id: String,
    pub variant_id: String,
    pub density: StyleDensity,
    pub colors: VisualStyleColors,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualStyleColors {
    pub keycap_background: Option<String>,
    pub keycap_text: Option<String>,
    pub keycap_border: Option<String>,
    pub modifier_accent: Option<String>,
    pub overlay_background: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourcePrecedenceRule {
    pub field_scope: String,
    pub source_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserOverride {
    pub field_path: String,
    pub value: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub schema_version: u32,
    pub id: String,
    pub keyboard_id: String,
    pub name: String,
    pub sources: Vec<Source>,
    pub physical_layout: PhysicalLayout,
    pub keymap: LogicalKeymap,
    pub runtime_backends: Vec<BackendStatus>,
    pub sentinel_keys: Vec<SentinelKeyBinding>,
    pub visual_style: VisualStyle,
    pub overlay_window: OverlayWindowConfig,
    pub source_precedence: Vec<SourcePrecedenceRule>,
    pub user_overrides: Vec<UserOverride>,
    pub source_provenance: Vec<SourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyboardSnapshot {
    pub profile_id: String,
    pub keyboard_id: String,
    pub profile_name: String,
    pub sources: Vec<Source>,
    pub physical_layout: PhysicalLayout,
    pub keymap: LogicalKeymap,
    pub runtime_state: RuntimeState,
    pub effective_keys: Vec<EffectiveKey>,
    pub backends: Vec<BackendStatus>,
    pub sentinel_keys: Vec<SentinelKeyBinding>,
    pub source_conflicts: Vec<SourceConflict>,
    pub source_provenance: Vec<SourceRef>,
    pub source_precedence: Vec<SourcePrecedenceRule>,
    pub user_overrides: Vec<UserOverride>,
    pub visual_style: VisualStyle,
    pub overlay_window: OverlayWindowConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RuntimeEvent {
    LayerStackChanged { layer_stack: Vec<LayerActivation> },
    BackendHealthChanged { health: BackendHealth },
    PressedKeysChanged { pressed_keys: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportSummary {
    pub imported_keys: usize,
    pub imported_layers: usize,
    pub preserved_sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportCandidate {
    pub id: String,
    pub source: Source,
    pub best_effort_preview: bool,
    pub preview_profile: Profile,
    pub conflicts: Vec<SourceConflict>,
    pub summary: ImportSummary,
}

pub fn compose_snapshot(
    profile: &Profile,
    runtime_state: RuntimeState,
    source_conflicts: Vec<SourceConflict>,
) -> KeyboardSnapshot {
    let source_conflicts = resolve_source_conflicts(
        source_conflicts,
        &profile.source_precedence,
        &profile.user_overrides,
    );
    let mut physical_layout = profile.physical_layout.clone();
    apply_physical_layout_conflict_selection(&mut physical_layout, &source_conflicts);
    let mut keymap = profile.keymap.clone();
    apply_keymap_conflict_selection(&mut keymap, &source_conflicts);
    let effective_keys = resolve_effective_keys(&keymap, &runtime_state);
    let mut visual_style = profile.visual_style.clone();
    apply_visual_conflict_selection(&mut visual_style, &source_conflicts);

    KeyboardSnapshot {
        profile_id: profile.id.clone(),
        keyboard_id: profile.keyboard_id.clone(),
        profile_name: profile.name.clone(),
        sources: profile.sources.clone(),
        physical_layout,
        keymap,
        runtime_state,
        effective_keys,
        backends: profile.runtime_backends.clone(),
        sentinel_keys: profile.sentinel_keys.clone(),
        source_conflicts,
        source_provenance: profile.source_provenance.clone(),
        source_precedence: profile.source_precedence.clone(),
        user_overrides: profile.user_overrides.clone(),
        visual_style,
        overlay_window: profile.overlay_window.clone(),
    }
}

pub fn resolve_source_conflicts(
    conflicts: Vec<SourceConflict>,
    precedence_rules: &[SourcePrecedenceRule],
    user_overrides: &[UserOverride],
) -> Vec<SourceConflict> {
    conflicts
        .into_iter()
        .map(|conflict| resolve_source_conflict(&conflict, precedence_rules, user_overrides))
        .collect()
}

pub fn resolve_source_conflict(
    conflict: &SourceConflict,
    precedence_rules: &[SourcePrecedenceRule],
    user_overrides: &[UserOverride],
) -> SourceConflict {
    let mut resolved = conflict.clone();

    if let Some(user_override) = user_overrides
        .iter()
        .rev()
        .find(|candidate| candidate.field_path == resolved.field_path)
    {
        upsert_user_override_candidate(&mut resolved, user_override);
        select_conflict_source(&mut resolved, USER_OVERRIDE_SOURCE_ID);
        return resolved;
    }

    if let Some(source_id) = selected_source_from_precedence(&resolved, precedence_rules) {
        select_conflict_source(&mut resolved, &source_id);
        return resolved;
    }

    if resolved
        .candidates
        .iter()
        .any(|candidate| candidate.source_id == resolved.selected_source_id)
    {
        let selected_source_id = resolved.selected_source_id.clone();
        select_conflict_source(&mut resolved, &selected_source_id);
        return resolved;
    }

    if let Some(source_id) = resolved
        .candidates
        .first()
        .map(|candidate| candidate.source_id.clone())
    {
        select_conflict_source(&mut resolved, &source_id);
    }

    resolved
}

fn selected_source_from_precedence(
    conflict: &SourceConflict,
    precedence_rules: &[SourcePrecedenceRule],
) -> Option<String> {
    let rule = precedence_rules
        .iter()
        .filter(|rule| field_path_matches_scope(&conflict.field_path, &rule.field_scope))
        .max_by_key(|rule| rule.field_scope.len())?;

    rule.source_order
        .iter()
        .find(|source_id| {
            conflict
                .candidates
                .iter()
                .any(|candidate| candidate.source_id == **source_id)
        })
        .cloned()
}

fn field_path_matches_scope(field_path: &str, field_scope: &str) -> bool {
    field_path == field_scope || field_path.starts_with(&format!("{field_scope} "))
}

fn upsert_user_override_candidate(conflict: &mut SourceConflict, user_override: &UserOverride) {
    if let Some(existing) = conflict
        .candidates
        .iter_mut()
        .find(|candidate| candidate.source_id == USER_OVERRIDE_SOURCE_ID)
    {
        existing.value = user_override.value.clone();
        return;
    }

    conflict.candidates.push(SourceCandidate {
        source_id: USER_OVERRIDE_SOURCE_ID.to_string(),
        value: user_override.value.clone(),
        selected: false,
    });
}

fn select_conflict_source(conflict: &mut SourceConflict, source_id: &str) {
    conflict.selected_source_id = source_id.to_string();
    for candidate in conflict.candidates.iter_mut() {
        candidate.selected = candidate.source_id == source_id;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhysicalLayoutField {
    GeometryX,
    GeometryY,
    GeometryWidth,
    GeometryHeight,
    GeometryRotation,
    MatrixRow,
    MatrixCol,
}

fn apply_physical_layout_conflict_selection(
    physical_layout: &mut PhysicalLayout,
    source_conflicts: &[SourceConflict],
) {
    for conflict in source_conflicts {
        let Some((key_id, field)) = physical_layout_field_path(&conflict.field_path) else {
            continue;
        };
        let Some(selected) = conflict
            .candidates
            .iter()
            .find(|candidate| candidate.selected)
        else {
            continue;
        };
        let Some(key) = physical_layout
            .keys
            .iter_mut()
            .find(|candidate| candidate.id == key_id)
        else {
            continue;
        };

        if !apply_physical_key_value(key, field, &selected.value) {
            continue;
        }
        key.provenance = SourceRef {
            source_id: selected.source_id.clone(),
            field_path: conflict.field_path.clone(),
            raw: Some(selected.value.clone()),
        };
    }
}

fn physical_layout_field_path(field_path: &str) -> Option<(String, PhysicalLayoutField)> {
    let parts: Vec<&str> = field_path.split_whitespace().collect();
    let [":keyboard/physical-layout", key_id, field] = parts.as_slice() else {
        return None;
    };

    let field = match *field {
        ":geometry/x" => PhysicalLayoutField::GeometryX,
        ":geometry/y" => PhysicalLayoutField::GeometryY,
        ":geometry/width" => PhysicalLayoutField::GeometryWidth,
        ":geometry/height" => PhysicalLayoutField::GeometryHeight,
        ":geometry/rotation" => PhysicalLayoutField::GeometryRotation,
        ":matrix/row" => PhysicalLayoutField::MatrixRow,
        ":matrix/col" => PhysicalLayoutField::MatrixCol,
        _ => return None,
    };

    Some(((*key_id).to_string(), field))
}

fn apply_physical_key_value(
    key: &mut PhysicalKey,
    field: PhysicalLayoutField,
    value: &str,
) -> bool {
    match field {
        PhysicalLayoutField::GeometryX => parse_f32(value)
            .map(|value| key.geometry.x = value)
            .is_some(),
        PhysicalLayoutField::GeometryY => parse_f32(value)
            .map(|value| key.geometry.y = value)
            .is_some(),
        PhysicalLayoutField::GeometryWidth => parse_f32(value)
            .map(|value| key.geometry.width = value)
            .is_some(),
        PhysicalLayoutField::GeometryHeight => parse_f32(value)
            .map(|value| key.geometry.height = value)
            .is_some(),
        PhysicalLayoutField::GeometryRotation => parse_f32(value)
            .map(|value| key.geometry.rotation = value)
            .is_some(),
        PhysicalLayoutField::MatrixRow => {
            let Some(value) = parse_u16(value) else {
                return false;
            };
            matrix_position(key).row = value;
            true
        }
        PhysicalLayoutField::MatrixCol => {
            let Some(value) = parse_u16(value) else {
                return false;
            };
            matrix_position(key).col = value;
            true
        }
    }
}

fn parse_f32(value: &str) -> Option<f32> {
    let value: f32 = value.trim().parse().ok()?;
    value.is_finite().then_some(value)
}

fn parse_u16(value: &str) -> Option<u16> {
    value.trim().parse().ok()
}

fn matrix_position(key: &mut PhysicalKey) -> &mut MatrixPosition {
    key.matrix.get_or_insert(MatrixPosition { row: 0, col: 0 })
}

fn apply_keymap_conflict_selection(
    keymap: &mut LogicalKeymap,
    source_conflicts: &[SourceConflict],
) {
    for conflict in source_conflicts {
        let Some((layer_id, key_id)) = keymap_action_field_path(&conflict.field_path) else {
            continue;
        };
        let Some(selected) = conflict
            .candidates
            .iter()
            .find(|candidate| candidate.selected)
        else {
            continue;
        };
        let Some(action) = keymap
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id)
            .and_then(|layer| {
                layer
                    .actions
                    .iter_mut()
                    .find(|action| action.key_id == key_id)
            })
        else {
            continue;
        };

        let dialect = action.raw.dialect.clone();
        *action = derive_action(
            &dialect,
            &selected.value,
            SourceRef {
                source_id: selected.source_id.clone(),
                field_path: conflict.field_path.clone(),
                raw: Some(selected.value.clone()),
            },
            &key_id,
        );
    }
}

fn keymap_action_field_path(field_path: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = field_path.split_whitespace().collect();
    match parts.as_slice() {
        [":keyboard/keymap", layer_id, key_id]
        | [":keyboard/keymap", layer_id, key_id, ":action/raw"] => Some((
            layer_id.trim_start_matches(':').to_string(),
            (*key_id).to_string(),
        )),
        _ => None,
    }
}

fn apply_visual_conflict_selection(
    visual_style: &mut VisualStyle,
    source_conflicts: &[SourceConflict],
) {
    for conflict in source_conflicts {
        let Some(value) = conflict
            .candidates
            .iter()
            .find(|candidate| candidate.selected)
            .map(|candidate| candidate.value.clone())
        else {
            continue;
        };

        match conflict.field_path.as_str() {
            ":visual/style :style/id" => visual_style.id = value,
            ":visual/style :style/variant-id" => visual_style.variant_id = value,
            ":visual/style :style/density" => {
                if let Some(density) = parse_style_density_label(&value) {
                    visual_style.density = density;
                }
            }
            ":visual/style :style/colors :color/keycap-background" => {
                visual_style.colors.keycap_background = source_conflict_optional_value(&value);
            }
            ":visual/style :style/colors :color/keycap-text" => {
                visual_style.colors.keycap_text = source_conflict_optional_value(&value);
            }
            ":visual/style :style/colors :color/keycap-border" => {
                visual_style.colors.keycap_border = source_conflict_optional_value(&value);
            }
            ":visual/style :style/colors :color/modifier-accent" => {
                visual_style.colors.modifier_accent = source_conflict_optional_value(&value);
            }
            ":visual/style :style/colors :color/overlay-background" => {
                visual_style.colors.overlay_background = source_conflict_optional_value(&value);
            }
            _ => {}
        }
    }
}

fn parse_style_density_label(value: &str) -> Option<StyleDensity> {
    match value {
        "compact" => Some(StyleDensity::Compact),
        "standard" => Some(StyleDensity::Standard),
        "rich" => Some(StyleDensity::Rich),
        _ => None,
    }
}

fn source_conflict_optional_value(value: &str) -> Option<String> {
    if value == "nil" {
        None
    } else {
        Some(value.to_string())
    }
}

pub fn apply_runtime_event(snapshot: &mut KeyboardSnapshot, event: RuntimeEvent) {
    match event {
        RuntimeEvent::LayerStackChanged { layer_stack } => {
            snapshot.runtime_state.layer_stack = layer_stack;
        }
        RuntimeEvent::BackendHealthChanged { health } => {
            if let Some(existing) = snapshot
                .runtime_state
                .backend_health
                .iter_mut()
                .find(|item| item.backend_id == health.backend_id)
            {
                *existing = health.clone();
            } else {
                snapshot.runtime_state.backend_health.push(health.clone());
            }

            if let Some(existing) = snapshot
                .backends
                .iter_mut()
                .find(|backend| backend.id == health.backend_id)
            {
                existing.health = health;
            }
        }
        RuntimeEvent::PressedKeysChanged { pressed_keys } => {
            snapshot.runtime_state.pressed_keys = pressed_keys;
        }
    }

    snapshot.effective_keys = resolve_effective_keys(&snapshot.keymap, &snapshot.runtime_state);
}

pub fn resolve_effective_keys(
    keymap: &LogicalKeymap,
    runtime_state: &RuntimeState,
) -> Vec<EffectiveKey> {
    let Some(base_layer) = keymap.layers.first() else {
        return Vec::new();
    };

    base_layer
        .actions
        .iter()
        .map(|base_action| {
            resolve_effective_key(&base_action.key_id, keymap, runtime_state)
                .unwrap_or_else(|| effective_from_action(base_action, &base_layer.id, false))
        })
        .collect()
}

fn resolve_effective_key(
    key_id: &str,
    keymap: &LogicalKeymap,
    runtime_state: &RuntimeState,
) -> Option<EffectiveKey> {
    let mut inherited = false;

    for activation in &runtime_state.layer_stack {
        let Some(layer) = keymap
            .layers
            .iter()
            .find(|candidate| candidate.id == activation.layer_id)
        else {
            continue;
        };
        let Some(action) = layer
            .actions
            .iter()
            .find(|candidate| candidate.key_id == key_id)
        else {
            inherited = true;
            continue;
        };

        if action.semantic.kind == SemanticActionKind::Transparent {
            inherited = true;
            continue;
        }

        return Some(effective_from_action(action, &layer.id, inherited));
    }

    None
}

fn effective_from_action(
    action: &KeyAction,
    source_layer_id: &str,
    inherited: bool,
) -> EffectiveKey {
    EffectiveKey {
        key_id: action.key_id.clone(),
        raw: action.raw.clone(),
        semantic: action.semantic.clone(),
        legend: action.legend.clone(),
        source_layer_id: source_layer_id.to_string(),
        inherited,
    }
}

pub fn derive_action(
    dialect: &str,
    raw_value: &str,
    provenance: SourceRef,
    key_id: &str,
) -> KeyAction {
    let semantic = derive_semantic_action(raw_value);
    let legend = legend_for_action(raw_value, &semantic);

    KeyAction {
        key_id: key_id.to_string(),
        raw: RawAction {
            dialect: dialect.to_string(),
            value: raw_value.to_string(),
        },
        semantic,
        legend,
        provenance,
    }
}

pub fn derive_semantic_action(raw_value: &str) -> SemanticAction {
    let value = raw_value.trim();
    let upper = value.to_ascii_uppercase();

    if matches!(
        upper.as_str(),
        "KC_TRNS" | "_______" | "TRANSPARENT" | "&TRANS"
    ) {
        return SemanticAction {
            kind: SemanticActionKind::Transparent,
            label: "trans".to_string(),
            target_layer: None,
            hold_label: None,
        };
    }

    if matches!(upper.as_str(), "KC_NO" | "XXXXXXX" | "NONE" | "&NONE") {
        return SemanticAction {
            kind: SemanticActionKind::None,
            label: "none".to_string(),
            target_layer: None,
            hold_label: None,
        };
    }

    if let Some(target) = parse_wrapped_arg(value, "MO") {
        return layer_action(SemanticActionKind::LayerMomentary, "hold", target);
    }

    if let Some(target) = parse_wrapped_arg(value, "TG") {
        return layer_action(SemanticActionKind::LayerToggle, "toggle", target);
    }

    if let Some((target, tap)) = parse_layer_tap(value) {
        let tap_label = key_label(tap);
        return SemanticAction {
            kind: SemanticActionKind::LayerTap,
            label: tap_label,
            target_layer: Some(format!("layer-{}", target.trim())),
            hold_label: None,
        };
    }

    if let Some(target) = parse_space_separated_arg(value, "&mo") {
        return layer_action(SemanticActionKind::LayerMomentary, "hold", target);
    }

    if let Some(target) = parse_space_separated_arg(value, "&tog") {
        return layer_action(SemanticActionKind::LayerToggle, "toggle", target);
    }

    if let Some((target, tap)) = parse_zmk_layer_tap(value) {
        return SemanticAction {
            kind: SemanticActionKind::LayerTap,
            label: key_label(tap),
            target_layer: Some(format!("layer-{}", target.trim())),
            hold_label: None,
        };
    }

    if let Some((hold_label, tap)) = parse_qmk_mod_tap(value) {
        return SemanticAction {
            kind: SemanticActionKind::TapHold,
            label: key_label(tap),
            target_layer: None,
            hold_label: Some(hold_label),
        };
    }

    if upper.starts_with("KC_MS_") || upper.starts_with("MS_") {
        return SemanticAction {
            kind: SemanticActionKind::Mouse,
            label: mouse_label(value),
            target_layer: None,
            hold_label: None,
        };
    }

    if upper.starts_with("MACRO") || upper.starts_with("DM_") {
        return SemanticAction {
            kind: SemanticActionKind::Macro,
            label: "macro".to_string(),
            target_layer: None,
            hold_label: None,
        };
    }

    if matches!(
        upper.as_str(),
        "KC_LCTL"
            | "KC_RCTL"
            | "KC_LSFT"
            | "KC_RSFT"
            | "KC_LALT"
            | "KC_RALT"
            | "KC_LGUI"
            | "KC_RGUI"
            | "KC_MEH"
            | "KC_HYPR"
    ) {
        return SemanticAction {
            kind: SemanticActionKind::Modifier,
            label: key_label(value),
            target_layer: None,
            hold_label: None,
        };
    }

    if upper.starts_with("KC_") || upper.starts_with("&KP ") {
        return SemanticAction {
            kind: SemanticActionKind::Key,
            label: key_label(value),
            target_layer: None,
            hold_label: None,
        };
    }

    SemanticAction {
        kind: SemanticActionKind::Unknown,
        label: value.to_string(),
        target_layer: None,
        hold_label: None,
    }
}

fn layer_action(kind: SemanticActionKind, label: &str, target: &str) -> SemanticAction {
    SemanticAction {
        kind,
        label: label.to_string(),
        target_layer: Some(format!("layer-{}", target.trim())),
        hold_label: None,
    }
}

fn parse_wrapped_arg<'a>(value: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{}(", name);
    value
        .strip_prefix(&prefix)
        .and_then(|rest| rest.strip_suffix(')'))
        .map(str::trim)
}

fn parse_layer_tap(value: &str) -> Option<(&str, &str)> {
    let rest = value.strip_prefix("LT(")?.strip_suffix(')')?;
    let (target, tap) = rest.split_once(',')?;
    Some((target.trim(), tap.trim()))
}

fn parse_space_separated_arg<'a>(value: &'a str, behavior: &str) -> Option<&'a str> {
    let mut parts = value.split_whitespace();
    let candidate = parts.next()?;
    if !candidate.eq_ignore_ascii_case(behavior) {
        return None;
    }
    parts.next()
}

fn parse_zmk_layer_tap(value: &str) -> Option<(&str, &str)> {
    let mut parts = value.split_whitespace();
    let behavior = parts.next()?;
    if !behavior.eq_ignore_ascii_case("&lt") {
        return None;
    }
    let target = parts.next()?;
    let tap = parts.next()?;
    Some((target, tap))
}

fn parse_qmk_mod_tap(value: &str) -> Option<(String, &str)> {
    if let Some((hold, tap)) = parse_mod_tap_args(value) {
        return Some((modifier_label(hold), tap));
    }

    let (alias, tap) = parse_qmk_mod_tap_alias(value)?;
    Some((qmk_mod_tap_alias_label(alias)?, tap))
}

fn parse_mod_tap_args(value: &str) -> Option<(&str, &str)> {
    let rest = value.strip_prefix("MT(")?.strip_suffix(')')?;
    let (hold, tap) = rest.split_once(',')?;
    Some((hold.trim(), tap.trim()))
}

fn parse_qmk_mod_tap_alias(value: &str) -> Option<(&str, &str)> {
    let (alias, rest) = value.split_once('(')?;
    let tap = rest.strip_suffix(')')?.trim();
    Some((alias.trim(), tap))
}

fn qmk_mod_tap_alias_label(alias: &str) -> Option<String> {
    let label = match alias.to_ascii_uppercase().as_str() {
        "LCTL_T" | "RCTL_T" | "CTL_T" => "Ctrl",
        "LSFT_T" | "RSFT_T" | "SFT_T" => "Shift",
        "LALT_T" | "RALT_T" | "ALT_T" => "Alt",
        "LGUI_T" | "RGUI_T" | "GUI_T" => "Cmd",
        "MEH_T" => "Meh",
        "HYPR_T" | "ALL_T" => "Hyper",
        _ => return None,
    };
    Some(label.to_string())
}

fn modifier_label(value: &str) -> String {
    let normalized = value
        .replace("MOD_", "")
        .replace("KC_", "")
        .replace(' ', "");
    let parts: Vec<String> = normalized
        .split('|')
        .filter(|part| !part.is_empty())
        .map(single_modifier_label)
        .collect();

    if parts.is_empty() {
        value.trim().to_string()
    } else {
        parts.join("+")
    }
}

fn single_modifier_label(value: &str) -> String {
    match value.to_ascii_uppercase().as_str() {
        "LCTL" | "RCTL" | "CTL" => "Ctrl".to_string(),
        "LSFT" | "RSFT" | "SFT" => "Shift".to_string(),
        "LALT" | "RALT" | "ALT" => "Alt".to_string(),
        "LGUI" | "RGUI" | "GUI" => "Cmd".to_string(),
        "MEH" => "Meh".to_string(),
        "HYPR" | "ALL" => "Hyper".to_string(),
        other => other.replace('_', " "),
    }
}

fn key_label(value: &str) -> String {
    match key_token(value).as_str() {
        "ESC" => "Esc".to_string(),
        "ENT" | "ENTER" => "Enter".to_string(),
        "BSPC" => "Backspace".to_string(),
        "SPC" | "SPACE" => "Space".to_string(),
        "TAB" => "Tab".to_string(),
        "DEL" => "Del".to_string(),
        "LCTL" | "RCTL" => "Ctrl".to_string(),
        "LSFT" | "RSFT" => "Shift".to_string(),
        "LALT" | "RALT" => "Alt".to_string(),
        "LGUI" | "RGUI" => "Cmd".to_string(),
        "LEFT" => "Left".to_string(),
        "RGHT" | "RIGHT" => "Right".to_string(),
        "UP" => "Up".to_string(),
        "DOWN" => "Down".to_string(),
        "N1" | "NUMBER_1" => "1".to_string(),
        "N2" | "NUMBER_2" => "2".to_string(),
        "N3" | "NUMBER_3" => "3".to_string(),
        "N4" | "NUMBER_4" => "4".to_string(),
        "N5" | "NUMBER_5" => "5".to_string(),
        "N6" | "NUMBER_6" => "6".to_string(),
        "N7" | "NUMBER_7" => "7".to_string(),
        "N8" | "NUMBER_8" => "8".to_string(),
        "N9" | "NUMBER_9" => "9".to_string(),
        "N0" | "NUMBER_0" => "0".to_string(),
        "MINS" | "MINUS" => "-".to_string(),
        "EQL" | "EQUAL" => "=".to_string(),
        "LBRC" | "LBRACKET" | "LEFT_BRACKET" => "[".to_string(),
        "RBRC" | "RBRACKET" | "RIGHT_BRACKET" => "]".to_string(),
        "BSLS" | "BSLASH" | "BACKSLASH" => "\\".to_string(),
        "SCLN" | "SEMICOLON" => ";".to_string(),
        "QUOT" | "QUOTE" | "SQT" => "'".to_string(),
        "GRV" | "GRAVE" | "BTICK" => "`".to_string(),
        "COMM" | "COMMA" => ",".to_string(),
        "DOT" | "PERIOD" => ".".to_string(),
        "SLSH" | "SLASH" | "FSLH" => "/".to_string(),
        "EXLM" | "EXCLAIM" => "!".to_string(),
        "AT" => "@".to_string(),
        "HASH" => "#".to_string(),
        "DLR" | "DOLLAR" => "$".to_string(),
        "PERC" | "PERCENT" => "%".to_string(),
        "CIRC" | "CIRCUMFLEX" => "^".to_string(),
        "AMPR" | "AMPERSAND" => "&".to_string(),
        "ASTR" | "ASTERISK" => "*".to_string(),
        "LPRN" | "LEFT_PAREN" => "(".to_string(),
        "RPRN" | "RIGHT_PAREN" => ")".to_string(),
        "UNDS" | "UNDERSCORE" => "_".to_string(),
        "PLUS" => "+".to_string(),
        "LCBR" | "LEFT_CURLY_BRACE" => "{".to_string(),
        "RCBR" | "RIGHT_CURLY_BRACE" => "}".to_string(),
        "PIPE" => "|".to_string(),
        "COLN" | "COLON" => ":".to_string(),
        "DQUO" | "DOUBLE_QUOTE" => "\"".to_string(),
        "TILD" | "TILDE" => "~".to_string(),
        "LT" => "<".to_string(),
        "GT" => ">".to_string(),
        "QUES" | "QUESTION" => "?".to_string(),
        other if other.len() == 1 => other.to_string(),
        other => other.replace('_', " "),
    }
}

fn key_token(value: &str) -> String {
    let upper = value.trim().to_ascii_uppercase();
    upper
        .strip_prefix("KC_")
        .or_else(|| upper.strip_prefix("&KP "))
        .unwrap_or(&upper)
        .trim()
        .to_string()
}

fn mouse_label(value: &str) -> String {
    key_label(value.trim_start_matches("KC_MS_").trim_start_matches("MS_"))
}

fn legend_for_action(raw_value: &str, semantic: &SemanticAction) -> DisplayLegend {
    let mut legend = legend_for_semantic(semantic);

    if semantic.kind == SemanticActionKind::Key {
        if let Some(shifted_label) = shifted_label_for_key(raw_value) {
            insert_after_primary(
                &mut legend.slots,
                LegendSlot {
                    slot: LegendSlotKind::Shifted,
                    text: shifted_label.to_string(),
                },
            );
        }
    }

    match semantic.kind {
        SemanticActionKind::Macro => insert_before_icon(
            &mut legend.slots,
            LegendSlot {
                slot: LegendSlotKind::ActionHint,
                text: raw_value.trim().to_string(),
            },
        ),
        SemanticActionKind::Mouse => insert_before_icon(
            &mut legend.slots,
            LegendSlot {
                slot: LegendSlotKind::ActionHint,
                text: "mouse".to_string(),
            },
        ),
        SemanticActionKind::Unknown => insert_before_icon(
            &mut legend.slots,
            LegendSlot {
                slot: LegendSlotKind::ActionHint,
                text: "unknown".to_string(),
            },
        ),
        _ => {}
    }

    legend
}

fn legend_for_semantic(semantic: &SemanticAction) -> DisplayLegend {
    let mut slots = vec![LegendSlot {
        slot: LegendSlotKind::Primary,
        text: semantic.label.clone(),
    }];

    match semantic.kind {
        SemanticActionKind::LayerMomentary | SemanticActionKind::LayerToggle => {
            if let Some(target_layer) = &semantic.target_layer {
                slots.push(LegendSlot {
                    slot: LegendSlotKind::LayerHint,
                    text: target_layer.clone(),
                });
            }
            push_icon_slot(&mut slots, "layer");
        }
        SemanticActionKind::LayerTap => {
            if let Some(target_layer) = &semantic.target_layer {
                slots.push(LegendSlot {
                    slot: LegendSlotKind::LayerHint,
                    text: target_layer.clone(),
                });
                slots.push(LegendSlot {
                    slot: LegendSlotKind::HoldRole,
                    text: format!("hold {target_layer}"),
                });
            }
            slots.push(LegendSlot {
                slot: LegendSlotKind::TapRole,
                text: format!("tap {}", semantic.label),
            });
            push_icon_slot(&mut slots, "layer");
        }
        SemanticActionKind::TapHold => {
            slots.push(LegendSlot {
                slot: LegendSlotKind::TapRole,
                text: format!("tap {}", semantic.label),
            });
            slots.push(LegendSlot {
                slot: LegendSlotKind::HoldRole,
                text: semantic
                    .hold_label
                    .as_ref()
                    .map(|label| format!("hold {label}"))
                    .unwrap_or_else(|| "hold".to_string()),
            });
            push_icon_slot(&mut slots, "tap-hold");
        }
        SemanticActionKind::Modifier => {
            push_icon_slot(&mut slots, "mod");
        }
        SemanticActionKind::Macro => {
            push_icon_slot(&mut slots, "macro");
        }
        SemanticActionKind::Mouse => {
            push_icon_slot(&mut slots, "mouse");
        }
        SemanticActionKind::Transparent => {
            slots.push(LegendSlot {
                slot: LegendSlotKind::ActionHint,
                text: "inherits".to_string(),
            });
            push_icon_slot(&mut slots, "inherit");
        }
        SemanticActionKind::None => {
            push_icon_slot(&mut slots, "none");
        }
        SemanticActionKind::Unknown => {
            push_icon_slot(&mut slots, "unknown");
        }
        _ => {}
    }

    DisplayLegend { slots }
}

fn insert_after_primary(slots: &mut Vec<LegendSlot>, slot: LegendSlot) {
    let index = slots
        .iter()
        .position(|candidate| candidate.slot != LegendSlotKind::Primary)
        .unwrap_or(slots.len());
    slots.insert(index, slot);
}

fn insert_before_icon(slots: &mut Vec<LegendSlot>, slot: LegendSlot) {
    let index = slots
        .iter()
        .position(|candidate| candidate.slot == LegendSlotKind::Icon)
        .unwrap_or(slots.len());
    slots.insert(index, slot);
}

fn shifted_label_for_key(value: &str) -> Option<&'static str> {
    match key_token(value).as_str() {
        "1" | "N1" | "NUMBER_1" => Some("!"),
        "2" | "N2" | "NUMBER_2" => Some("@"),
        "3" | "N3" | "NUMBER_3" => Some("#"),
        "4" | "N4" | "NUMBER_4" => Some("$"),
        "5" | "N5" | "NUMBER_5" => Some("%"),
        "6" | "N6" | "NUMBER_6" => Some("^"),
        "7" | "N7" | "NUMBER_7" => Some("&"),
        "8" | "N8" | "NUMBER_8" => Some("*"),
        "9" | "N9" | "NUMBER_9" => Some("("),
        "0" | "N0" | "NUMBER_0" => Some(")"),
        "MINS" | "MINUS" => Some("_"),
        "EQL" | "EQUAL" => Some("+"),
        "LBRC" | "LBRACKET" | "LEFT_BRACKET" => Some("{"),
        "RBRC" | "RBRACKET" | "RIGHT_BRACKET" => Some("}"),
        "BSLS" | "BSLASH" | "BACKSLASH" => Some("|"),
        "SCLN" | "SEMICOLON" => Some(":"),
        "QUOT" | "QUOTE" | "SQT" => Some("\""),
        "GRV" | "GRAVE" | "BTICK" => Some("~"),
        "COMM" | "COMMA" => Some("<"),
        "DOT" | "PERIOD" => Some(">"),
        "SLSH" | "SLASH" | "FSLH" => Some("?"),
        _ => None,
    }
}

fn push_icon_slot(slots: &mut Vec<LegendSlot>, text: &str) {
    slots.push(LegendSlot {
        slot: LegendSlotKind::Icon,
        text: text.to_string(),
    });
}

pub fn promote_conflict_to_override(
    profile: &mut Profile,
    conflict: &SourceConflict,
    source_id: &str,
) -> Option<UserOverride> {
    let selected = conflict
        .candidates
        .iter()
        .find(|candidate| candidate.source_id == source_id)?;

    let user_override = UserOverride {
        field_path: conflict.field_path.clone(),
        value: selected.value.clone(),
        reason: format!("Promoted from {}", source_id),
    };

    if let Some(existing) = profile
        .user_overrides
        .iter_mut()
        .find(|candidate| candidate.field_path == user_override.field_path)
    {
        *existing = user_override.clone();
    } else {
        profile.user_overrides.push(user_override.clone());
    }

    Some(user_override)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_ref() -> SourceRef {
        SourceRef {
            source_id: "fake".to_string(),
            field_path: ":keyboard/keymap".to_string(),
            raw: None,
        }
    }

    fn visual_style_conflict(selected_source_id: &str) -> SourceConflict {
        SourceConflict {
            field_path: ":visual/style :style/variant-id".to_string(),
            selected_source_id: selected_source_id.to_string(),
            candidates: vec![
                SourceCandidate {
                    source_id: "fake-backend".to_string(),
                    value: "keyplane-default".to_string(),
                    selected: selected_source_id == "fake-backend",
                },
                SourceCandidate {
                    source_id: "keyviz-import".to_string(),
                    value: "keyviz-minimal".to_string(),
                    selected: selected_source_id == "keyviz-import",
                },
            ],
        }
    }

    fn keymap_action_conflict(selected_source_id: &str) -> SourceConflict {
        SourceConflict {
            field_path: ":keyboard/keymap :layer-0 k-q".to_string(),
            selected_source_id: selected_source_id.to_string(),
            candidates: vec![
                SourceCandidate {
                    source_id: "fake-backend".to_string(),
                    value: "KC_Q".to_string(),
                    selected: selected_source_id == "fake-backend",
                },
                SourceCandidate {
                    source_id: "vial-import".to_string(),
                    value: "KC_TAB".to_string(),
                    selected: selected_source_id == "vial-import",
                },
            ],
        }
    }

    fn physical_layout_conflict(selected_source_id: &str) -> SourceConflict {
        SourceConflict {
            field_path: ":keyboard/physical-layout k-q :geometry/x".to_string(),
            selected_source_id: selected_source_id.to_string(),
            candidates: vec![
                SourceCandidate {
                    source_id: "fake-backend".to_string(),
                    value: "1.08".to_string(),
                    selected: selected_source_id == "fake-backend",
                },
                SourceCandidate {
                    source_id: "vial-import".to_string(),
                    value: "2.5".to_string(),
                    selected: selected_source_id == "vial-import",
                },
            ],
        }
    }

    fn physical_layout_matrix_conflict(selected_source_id: &str) -> SourceConflict {
        SourceConflict {
            field_path: ":keyboard/physical-layout k-q :matrix/row".to_string(),
            selected_source_id: selected_source_id.to_string(),
            candidates: vec![
                SourceCandidate {
                    source_id: "fake-backend".to_string(),
                    value: "0".to_string(),
                    selected: selected_source_id == "fake-backend",
                },
                SourceCandidate {
                    source_id: "vial-import".to_string(),
                    value: "9".to_string(),
                    selected: selected_source_id == "vial-import",
                },
            ],
        }
    }

    fn precedence_rule(field_scope: &str, source_order: Vec<&str>) -> SourcePrecedenceRule {
        SourcePrecedenceRule {
            field_scope: field_scope.to_string(),
            source_order: source_order
                .into_iter()
                .map(|source_id| source_id.to_string())
                .collect(),
        }
    }

    #[test]
    fn source_precedence_selects_the_first_matching_source_for_a_field() {
        let conflict = visual_style_conflict("fake-backend");
        let resolved = resolve_source_conflict(
            &conflict,
            &[precedence_rule(
                ":visual/style",
                vec!["keyviz-import", "fake-backend"],
            )],
            &[],
        );

        assert_eq!(resolved.selected_source_id, "keyviz-import");
        assert!(resolved
            .candidates
            .iter()
            .any(|candidate| candidate.source_id == "fake-backend"
                && candidate.value == "keyplane-default"
                && !candidate.selected));
        assert!(resolved
            .candidates
            .iter()
            .any(|candidate| candidate.source_id == "keyviz-import"
                && candidate.value == "keyviz-minimal"
                && candidate.selected));
    }

    #[test]
    fn user_override_wins_over_source_precedence_and_keeps_losing_candidates() {
        let conflict = visual_style_conflict("fake-backend");
        let resolved = resolve_source_conflict(
            &conflict,
            &[precedence_rule(
                ":visual/style",
                vec!["fake-backend", "keyviz-import"],
            )],
            &[UserOverride {
                field_path: ":visual/style :style/variant-id".to_string(),
                value: "keyviz-minimal".to_string(),
                reason: "Pinned by user".to_string(),
            }],
        );

        assert_eq!(resolved.selected_source_id, USER_OVERRIDE_SOURCE_ID);
        assert_eq!(resolved.candidates.len(), 3);
        assert!(resolved
            .candidates
            .iter()
            .any(|candidate| candidate.source_id == USER_OVERRIDE_SOURCE_ID
                && candidate.value == "keyviz-minimal"
                && candidate.selected));
        assert!(resolved
            .candidates
            .iter()
            .any(|candidate| candidate.source_id == "fake-backend"
                && candidate.value == "keyplane-default"
                && !candidate.selected));
    }

    #[test]
    fn source_conflict_resolution_falls_back_to_existing_selection_without_a_rule() {
        let conflict = visual_style_conflict("keyviz-import");
        let resolved = resolve_source_conflict(&conflict, &[], &[]);

        assert_eq!(resolved.selected_source_id, "keyviz-import");
        assert!(resolved
            .candidates
            .iter()
            .any(|candidate| candidate.source_id == "keyviz-import" && candidate.selected));
    }

    #[test]
    fn source_precedence_scope_does_not_match_accidental_prefixes() {
        let mut conflict = visual_style_conflict("fake-backend");
        conflict.field_path = ":visual/style-extra :style/variant-id".to_string();

        let resolved = resolve_source_conflict(
            &conflict,
            &[precedence_rule(
                ":visual/style",
                vec!["keyviz-import", "fake-backend"],
            )],
            &[],
        );

        assert_eq!(resolved.selected_source_id, "fake-backend");
    }

    #[test]
    fn promoted_conflict_candidate_replaces_the_field_user_override() {
        let mut profile = crate::fake_backend::fake_profile();
        let conflict = visual_style_conflict("fake-backend");

        let first = promote_conflict_to_override(&mut profile, &conflict, "keyviz-import")
            .expect("candidate should be promotable");
        let second = promote_conflict_to_override(&mut profile, &conflict, "fake-backend")
            .expect("candidate should replace the prior override");

        assert_eq!(first.value, "keyviz-minimal");
        assert_eq!(second.value, "keyplane-default");
        assert_eq!(profile.user_overrides.len(), 1);
        assert_eq!(profile.user_overrides[0].field_path, conflict.field_path);
        assert_eq!(profile.user_overrides[0].value, "keyplane-default");
    }

    #[test]
    fn composed_snapshot_exposes_resolved_source_metadata_and_selected_style() {
        let mut profile = crate::fake_backend::fake_profile();
        profile.source_precedence.push(precedence_rule(
            ":visual/style",
            vec!["keyviz-import", "fake-backend"],
        ));
        let runtime_state = crate::fake_backend::initial_runtime_state(&profile);

        let snapshot = compose_snapshot(
            &profile,
            runtime_state,
            vec![visual_style_conflict("fake-backend")],
        );

        assert_eq!(snapshot.visual_style.variant_id, "keyviz-minimal");
        assert_eq!(
            snapshot.source_conflicts[0].selected_source_id,
            "keyviz-import"
        );
        assert_eq!(snapshot.source_precedence, profile.source_precedence);
        assert!(snapshot.user_overrides.is_empty());
    }

    #[test]
    fn composed_snapshot_applies_selected_physical_layout_field_conflicts() {
        let mut profile = crate::fake_backend::fake_profile();
        profile.source_precedence = vec![precedence_rule(
            ":keyboard/physical-layout",
            vec!["vial-import", "fake-backend"],
        )];
        let runtime_state = crate::fake_backend::initial_runtime_state(&profile);

        let snapshot = compose_snapshot(
            &profile,
            runtime_state,
            vec![physical_layout_conflict("fake-backend")],
        );

        let key = snapshot
            .physical_layout
            .keys
            .iter()
            .find(|key| key.id == "k-q")
            .expect("k-q physical key exists");
        assert_eq!(key.geometry.x, 2.5);
        assert_eq!(key.provenance.source_id, "vial-import");
        assert_eq!(
            key.provenance.field_path,
            ":keyboard/physical-layout k-q :geometry/x"
        );
        assert_eq!(
            snapshot.source_conflicts[0].selected_source_id,
            "vial-import"
        );
    }

    #[test]
    fn composed_snapshot_applies_selected_physical_layout_matrix_conflicts() {
        let mut profile = crate::fake_backend::fake_profile();
        profile.source_precedence = vec![precedence_rule(
            ":keyboard/physical-layout",
            vec!["vial-import", "fake-backend"],
        )];
        let runtime_state = crate::fake_backend::initial_runtime_state(&profile);

        let snapshot = compose_snapshot(
            &profile,
            runtime_state,
            vec![physical_layout_matrix_conflict("fake-backend")],
        );

        let key = snapshot
            .physical_layout
            .keys
            .iter()
            .find(|key| key.id == "k-q")
            .expect("k-q physical key exists");
        assert_eq!(key.matrix.as_ref().expect("matrix is present").row, 9);
        assert_eq!(key.provenance.source_id, "vial-import");
        assert_eq!(
            key.provenance.field_path,
            ":keyboard/physical-layout k-q :matrix/row"
        );
        assert_eq!(
            snapshot.source_conflicts[0].selected_source_id,
            "vial-import"
        );
    }

    #[test]
    fn composed_snapshot_applies_physical_layout_user_overrides_over_source_precedence() {
        let mut profile = crate::fake_backend::fake_profile();
        profile.source_precedence = vec![precedence_rule(
            ":keyboard/physical-layout",
            vec!["vial-import", "fake-backend"],
        )];
        profile.user_overrides = vec![UserOverride {
            field_path: ":keyboard/physical-layout k-q :geometry/x".to_string(),
            value: "3.25".to_string(),
            reason: "Pinned by user".to_string(),
        }];
        let runtime_state = crate::fake_backend::initial_runtime_state(&profile);

        let snapshot = compose_snapshot(
            &profile,
            runtime_state,
            vec![physical_layout_conflict("fake-backend")],
        );

        let key = snapshot
            .physical_layout
            .keys
            .iter()
            .find(|key| key.id == "k-q")
            .expect("k-q physical key exists");
        assert_eq!(key.geometry.x, 3.25);
        assert_eq!(key.provenance.source_id, USER_OVERRIDE_SOURCE_ID);
        assert_eq!(
            snapshot.source_conflicts[0].selected_source_id,
            USER_OVERRIDE_SOURCE_ID
        );
        assert_eq!(snapshot.source_precedence, profile.source_precedence);
    }

    #[test]
    fn composed_snapshot_applies_selected_visual_style_field_conflicts() {
        let mut profile = crate::fake_backend::fake_profile();
        profile.source_precedence = vec![precedence_rule(
            ":visual/style",
            vec!["keyviz-import", "fake-backend"],
        )];
        let runtime_state = crate::fake_backend::initial_runtime_state(&profile);

        let snapshot = compose_snapshot(
            &profile,
            runtime_state,
            vec![
                SourceConflict {
                    field_path: ":visual/style :style/id".to_string(),
                    selected_source_id: "keyviz-import".to_string(),
                    candidates: vec![
                        SourceCandidate {
                            source_id: "fake-backend".to_string(),
                            value: "style-keyplane-default".to_string(),
                            selected: false,
                        },
                        SourceCandidate {
                            source_id: "keyviz-import".to_string(),
                            value: "style-keyviz-lowprofile".to_string(),
                            selected: true,
                        },
                    ],
                },
                SourceConflict {
                    field_path: ":visual/style :style/density".to_string(),
                    selected_source_id: "keyviz-import".to_string(),
                    candidates: vec![
                        SourceCandidate {
                            source_id: "fake-backend".to_string(),
                            value: "standard".to_string(),
                            selected: false,
                        },
                        SourceCandidate {
                            source_id: "keyviz-import".to_string(),
                            value: "rich".to_string(),
                            selected: true,
                        },
                    ],
                },
                SourceConflict {
                    field_path: ":visual/style :style/colors :color/keycap-background".to_string(),
                    selected_source_id: "keyviz-import".to_string(),
                    candidates: vec![
                        SourceCandidate {
                            source_id: "fake-backend".to_string(),
                            value: "nil".to_string(),
                            selected: false,
                        },
                        SourceCandidate {
                            source_id: "keyviz-import".to_string(),
                            value: "#ffffff".to_string(),
                            selected: true,
                        },
                    ],
                },
            ],
        );

        assert_eq!(snapshot.visual_style.id, "style-keyviz-lowprofile");
        assert_eq!(snapshot.visual_style.density, StyleDensity::Rich);
        assert_eq!(
            snapshot.visual_style.colors.keycap_background.as_deref(),
            Some("#ffffff")
        );
    }

    #[test]
    fn composed_snapshot_applies_selected_keymap_field_conflicts_before_effective_resolution() {
        let mut profile = crate::fake_backend::fake_profile();
        profile.source_precedence = vec![precedence_rule(
            ":keyboard/keymap",
            vec!["vial-import", "fake-backend"],
        )];
        let runtime_state = crate::fake_backend::initial_runtime_state(&profile);

        let snapshot = compose_snapshot(
            &profile,
            runtime_state,
            vec![keymap_action_conflict("fake-backend")],
        );

        let key_action = snapshot.keymap.layers[0]
            .actions
            .iter()
            .find(|action| action.key_id == "k-q")
            .expect("k-q action exists");
        assert_eq!(key_action.raw.value, "KC_TAB");
        assert_eq!(key_action.semantic.label, "Tab");
        assert_eq!(key_action.provenance.source_id, "vial-import");

        let effective_key = snapshot
            .effective_keys
            .iter()
            .find(|key| key.key_id == "k-q")
            .expect("k-q effective key exists");
        assert_eq!(effective_key.raw.value, "KC_TAB");
        assert_eq!(effective_key.semantic.label, "Tab");
        assert_eq!(
            snapshot.source_conflicts[0].selected_source_id,
            "vial-import"
        );
    }

    #[test]
    fn composed_snapshot_applies_keymap_user_overrides_over_source_precedence() {
        let mut profile = crate::fake_backend::fake_profile();
        profile.source_precedence = vec![precedence_rule(
            ":keyboard/keymap",
            vec!["vial-import", "fake-backend"],
        )];
        profile.user_overrides = vec![UserOverride {
            field_path: ":keyboard/keymap :layer-0 k-q".to_string(),
            value: "KC_ESC".to_string(),
            reason: "Pinned by user".to_string(),
        }];
        let runtime_state = crate::fake_backend::initial_runtime_state(&profile);

        let snapshot = compose_snapshot(
            &profile,
            runtime_state,
            vec![keymap_action_conflict("fake-backend")],
        );

        let effective_key = snapshot
            .effective_keys
            .iter()
            .find(|key| key.key_id == "k-q")
            .expect("k-q effective key exists");
        assert_eq!(effective_key.raw.value, "KC_ESC");
        assert_eq!(effective_key.semantic.label, "Esc");
        assert_eq!(
            snapshot.source_conflicts[0].selected_source_id,
            USER_OVERRIDE_SOURCE_ID
        );
    }

    #[test]
    fn transparent_entries_resolve_to_lower_layer_effective_actions() {
        let base = Layer {
            id: "layer-0".to_string(),
            name: "Base".to_string(),
            actions: vec![derive_action("qmk", "KC_A", source_ref(), "k-a")],
        };
        let nav = Layer {
            id: "layer-1".to_string(),
            name: "Nav".to_string(),
            actions: vec![derive_action("qmk", "KC_TRNS", source_ref(), "k-a")],
        };
        let runtime = RuntimeState {
            layer_stack: vec![
                LayerActivation {
                    layer_id: "layer-1".to_string(),
                    kind: ActivationKind::Momentary,
                    confidence: StateConfidence {
                        level: StateConfidenceLevel::High,
                        reason: "fake backend event".to_string(),
                    },
                },
                LayerActivation {
                    layer_id: "layer-0".to_string(),
                    kind: ActivationKind::Default,
                    confidence: StateConfidence {
                        level: StateConfidenceLevel::High,
                        reason: "default layer".to_string(),
                    },
                },
            ],
            pressed_keys: Vec::new(),
            backend_health: Vec::new(),
        };

        let resolved = resolve_effective_keys(
            &LogicalKeymap {
                layers: vec![base, nav],
            },
            &runtime,
        );

        assert_eq!(resolved[0].semantic.label, "A");
        assert_eq!(resolved[0].source_layer_id, "layer-0");
        assert!(resolved[0].inherited);
    }

    #[test]
    fn semantic_actions_keep_raw_actions_separate_from_display_labels() {
        let action = derive_action("qmk", "MO(1)", source_ref(), "k-fn");

        assert_eq!(action.raw.value, "MO(1)");
        assert_eq!(action.semantic.kind, SemanticActionKind::LayerMomentary);
        assert_eq!(action.semantic.target_layer, Some("layer-1".to_string()));
        assert_eq!(
            action.legend.slots,
            vec![
                LegendSlot {
                    slot: LegendSlotKind::Primary,
                    text: "hold".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::LayerHint,
                    text: "layer-1".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::Icon,
                    text: "layer".to_string()
                }
            ]
        );
    }

    #[test]
    fn zmk_layer_behaviors_derive_visual_layer_semantics() {
        let momentary = derive_action("zmk", "&mo 2", source_ref(), "k-fn");
        let toggle = derive_action("zmk", "&tog 3", source_ref(), "k-toggle");
        let layer_tap = derive_action("zmk", "&lt 1 SPACE", source_ref(), "k-space");
        let qmk_layer_tap = derive_action("qmk", "LT(2, KC_TAB)", source_ref(), "k-tab");

        assert_eq!(momentary.raw.value, "&mo 2");
        assert_eq!(momentary.semantic.kind, SemanticActionKind::LayerMomentary);
        assert_eq!(momentary.semantic.target_layer.as_deref(), Some("layer-2"));
        assert_eq!(toggle.semantic.kind, SemanticActionKind::LayerToggle);
        assert_eq!(toggle.semantic.target_layer.as_deref(), Some("layer-3"));
        assert_eq!(layer_tap.semantic.kind, SemanticActionKind::LayerTap);
        assert_eq!(layer_tap.semantic.label, "Space");
        assert_eq!(layer_tap.semantic.target_layer.as_deref(), Some("layer-1"));
        assert_eq!(
            layer_tap.legend.slots,
            vec![
                LegendSlot {
                    slot: LegendSlotKind::Primary,
                    text: "Space".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::LayerHint,
                    text: "layer-1".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::HoldRole,
                    text: "hold layer-1".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::TapRole,
                    text: "tap Space".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::Icon,
                    text: "layer".to_string()
                }
            ]
        );
        assert_eq!(qmk_layer_tap.semantic.kind, SemanticActionKind::LayerTap);
        assert_eq!(qmk_layer_tap.semantic.label, "Tab");
        assert_eq!(
            qmk_layer_tap.semantic.target_layer.as_deref(),
            Some("layer-2")
        );
        assert!(qmk_layer_tap
            .legend
            .slots
            .iter()
            .any(|slot| { slot.slot == LegendSlotKind::TapRole && slot.text == "tap Tab" }));
        assert!(qmk_layer_tap
            .legend
            .slots
            .iter()
            .any(|slot| { slot.slot == LegendSlotKind::HoldRole && slot.text == "hold layer-2" }));
    }

    #[test]
    fn qmk_mod_taps_derive_tap_hold_role_slots() {
        let shifted_space = derive_action("qmk", "LSFT_T(KC_SPC)", source_ref(), "k-space");
        let ctrl_escape = derive_action("qmk", "MT(MOD_LCTL, KC_ESC)", source_ref(), "k-esc");
        let hyper_tab = derive_action("qmk", "HYPR_T(KC_TAB)", source_ref(), "k-tab");

        assert_eq!(shifted_space.raw.value, "LSFT_T(KC_SPC)");
        assert_eq!(shifted_space.semantic.kind, SemanticActionKind::TapHold);
        assert_eq!(shifted_space.semantic.label, "Space");
        assert_eq!(shifted_space.semantic.hold_label.as_deref(), Some("Shift"));
        assert_eq!(
            shifted_space.legend.slots,
            vec![
                LegendSlot {
                    slot: LegendSlotKind::Primary,
                    text: "Space".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::TapRole,
                    text: "tap Space".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::HoldRole,
                    text: "hold Shift".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::Icon,
                    text: "tap-hold".to_string()
                }
            ]
        );
        assert_eq!(ctrl_escape.semantic.kind, SemanticActionKind::TapHold);
        assert_eq!(ctrl_escape.semantic.label, "Esc");
        assert_eq!(ctrl_escape.semantic.hold_label.as_deref(), Some("Ctrl"));
        assert!(ctrl_escape
            .legend
            .slots
            .iter()
            .any(|slot| { slot.slot == LegendSlotKind::HoldRole && slot.text == "hold Ctrl" }));
        assert_eq!(hyper_tab.semantic.kind, SemanticActionKind::TapHold);
        assert_eq!(hyper_tab.semantic.label, "Tab");
        assert_eq!(hyper_tab.semantic.hold_label.as_deref(), Some("Hyper"));
    }

    #[test]
    fn number_and_symbol_keys_derive_shifted_legend_slots() {
        let one = derive_action("qmk", "KC_1", source_ref(), "k-1");
        let minus = derive_action("qmk", "KC_MINS", source_ref(), "k-minus");
        let zmk_two = derive_action("zmk", "&kp N2", source_ref(), "k-2");
        let shifted_alias = derive_action("qmk", "KC_EXLM", source_ref(), "k-exclaim");

        assert_eq!(one.semantic.label, "1");
        assert_eq!(
            one.legend.slots,
            vec![
                LegendSlot {
                    slot: LegendSlotKind::Primary,
                    text: "1".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::Shifted,
                    text: "!".to_string()
                }
            ]
        );
        assert_eq!(minus.semantic.label, "-");
        assert!(minus
            .legend
            .slots
            .iter()
            .any(|slot| { slot.slot == LegendSlotKind::Shifted && slot.text == "_" }));
        assert_eq!(zmk_two.semantic.label, "2");
        assert!(zmk_two
            .legend
            .slots
            .iter()
            .any(|slot| { slot.slot == LegendSlotKind::Shifted && slot.text == "@" }));
        assert_eq!(shifted_alias.semantic.label, "!");
        assert!(!shifted_alias
            .legend
            .slots
            .iter()
            .any(|slot| { slot.slot == LegendSlotKind::Shifted }));
    }

    #[test]
    fn macro_mouse_and_unknown_actions_derive_action_hint_slots() {
        let macro_action = derive_action("qmk", "DM_PLY1", source_ref(), "k-macro");
        let mouse = derive_action("qmk", "KC_MS_UP", source_ref(), "k-mouse");
        let unknown = derive_action("qmk", "CUSTOM_BEHAVIOR", source_ref(), "k-custom");

        assert_eq!(
            macro_action.legend.slots,
            vec![
                LegendSlot {
                    slot: LegendSlotKind::Primary,
                    text: "macro".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::ActionHint,
                    text: "DM_PLY1".to_string()
                },
                LegendSlot {
                    slot: LegendSlotKind::Icon,
                    text: "macro".to_string()
                }
            ]
        );
        assert!(mouse
            .legend
            .slots
            .iter()
            .any(|slot| { slot.slot == LegendSlotKind::ActionHint && slot.text == "mouse" }));
        assert!(unknown
            .legend
            .slots
            .iter()
            .any(|slot| { slot.slot == LegendSlotKind::ActionHint && slot.text == "unknown" }));
    }

    #[test]
    fn semantic_action_legends_include_optional_icon_slots() {
        let modifier = derive_action("qmk", "KC_LCTL", source_ref(), "k-ctrl");
        let mouse = derive_action("qmk", "KC_MS_UP", source_ref(), "k-mouse");
        let macro_action = derive_action("qmk", "DM_PLY1", source_ref(), "k-macro");
        let transparent = derive_action("qmk", "KC_TRNS", source_ref(), "k-trans");
        let none = derive_action("qmk", "KC_NO", source_ref(), "k-none");
        let unknown = derive_action("qmk", "CUSTOM_BEHAVIOR", source_ref(), "k-custom");

        let icon_texts = [
            (&modifier, "mod"),
            (&mouse, "mouse"),
            (&macro_action, "macro"),
            (&transparent, "inherit"),
            (&none, "none"),
            (&unknown, "unknown"),
        ];

        for (action, expected_icon) in icon_texts {
            assert!(
                action.legend.slots.iter().any(|slot| {
                    slot.slot == LegendSlotKind::Icon && slot.text == expected_icon
                }),
                "expected {expected_icon} icon slot for {}",
                action.raw.value
            );
        }
    }
}
