export type SourceAuthority = "authoritative" | "best-effort-preview" | "inferred";

export interface Source {
  id: string;
  name: string;
  kind: string;
  authority: SourceAuthority;
}

export interface SourceRef {
  source_id: string;
  field_path: string;
  raw: string | null;
}

export interface MatrixPosition {
  row: number;
  col: number;
}

export interface KeyGeometry {
  x: number;
  y: number;
  width: number;
  height: number;
  rotation: number;
}

export interface PhysicalKey {
  id: string;
  matrix: MatrixPosition | null;
  geometry: KeyGeometry;
  provenance: SourceRef;
}

export interface PhysicalLayout {
  keys: PhysicalKey[];
  fallback: boolean;
}

export interface RawAction {
  dialect: string;
  value: string;
}

export type SemanticActionKind =
  | "key"
  | "modifier"
  | "layer-momentary"
  | "layer-toggle"
  | "layer-tap"
  | "tap-hold"
  | "macro"
  | "transparent"
  | "none"
  | "mouse"
  | "unknown";

export interface SemanticAction {
  kind: SemanticActionKind;
  label: string;
  target_layer: string | null;
}

export type LegendSlotKind =
  | "primary"
  | "shifted"
  | "tap-role"
  | "hold-role"
  | "layer-hint"
  | "action-hint"
  | "icon";

export interface LegendSlot {
  slot: LegendSlotKind;
  text: string;
}

export interface DisplayLegend {
  slots: LegendSlot[];
}

export interface KeyAction {
  key_id: string;
  raw: RawAction;
  semantic: SemanticAction;
  legend: DisplayLegend;
  provenance: SourceRef;
}

export interface Layer {
  id: string;
  name: string;
  actions: KeyAction[];
}

export interface LogicalKeymap {
  layers: Layer[];
}

export type ActivationKind =
  | "default"
  | "momentary"
  | "toggle"
  | "tap-hold"
  | "lock"
  | "remapper-state"
  | "unknown";

export type StateConfidenceLevel = "high" | "medium" | "low";

export interface StateConfidence {
  level: StateConfidenceLevel;
  reason: string;
}

export interface LayerActivation {
  layer_id: string;
  kind: ActivationKind;
  confidence: StateConfidence;
}

export interface HostInputEvent {
  code: string;
  pressed: boolean;
}

export interface SentinelKeyBinding {
  host_input_code: string;
  layer_id: string;
  activation: ActivationKind;
}

export type CapabilityFlag =
  | "discover-devices"
  | "import-geometry"
  | "import-keymaps"
  | "stream-layer-stack"
  | "stream-pressed-keys"
  | "poll-state"
  | "preview-only";

export type HealthState =
  | "ok"
  | "permission-missing"
  | "disconnected"
  | "stale"
  | "unsupported"
  | "parse-error"
  | "protocol-error";

export interface BackendHealth {
  backend_id: string;
  state: HealthState;
  message: string;
}

export interface BackendStatus {
  id: string;
  name: string;
  capabilities: CapabilityFlag[];
  health: BackendHealth;
}

export interface RuntimeState {
  layer_stack: LayerActivation[];
  pressed_keys: string[];
  backend_health: BackendHealth[];
}

export interface EffectiveKey {
  key_id: string;
  raw: RawAction;
  semantic: SemanticAction;
  legend: DisplayLegend;
  source_layer_id: string;
  inherited: boolean;
}

export interface SourceCandidate {
  source_id: string;
  value: string;
  selected: boolean;
}

export interface SourceConflict {
  field_path: string;
  selected_source_id: string;
  candidates: SourceCandidate[];
}

export type VisibilityPolicy = "pinned" | "manual-toggle" | "fade";

export interface DisplayTargeting {
  display_id: string | null;
  x: number;
  y: number;
  width: number;
  height: number;
  opacity: number;
}

export interface OverlayWindowConfig {
  visibility: VisibilityPolicy;
  click_through: boolean;
  positioning_mode: boolean;
  display_targeting: DisplayTargeting;
}

export type StyleDensity = "compact" | "standard" | "rich";

export interface VisualStyle {
  variant_id: string;
  density: StyleDensity;
}

export interface SourcePrecedenceRule {
  field_scope: string;
  source_order: string[];
}

export interface UserOverride {
  field_path: string;
  value: string;
  reason: string;
}

export interface Profile {
  schema_version: number;
  id: string;
  name: string;
  sources: Source[];
  physical_layout: PhysicalLayout;
  keymap: LogicalKeymap;
  runtime_backends: BackendStatus[];
  sentinel_keys: SentinelKeyBinding[];
  visual_style: VisualStyle;
  overlay_window: OverlayWindowConfig;
  source_precedence: SourcePrecedenceRule[];
  user_overrides: UserOverride[];
  source_provenance: SourceRef[];
}

export interface KeyboardSnapshot {
  profile_id: string;
  profile_name: string;
  sources: Source[];
  physical_layout: PhysicalLayout;
  keymap: LogicalKeymap;
  runtime_state: RuntimeState;
  effective_keys: EffectiveKey[];
  backends: BackendStatus[];
  sentinel_keys: SentinelKeyBinding[];
  source_conflicts: SourceConflict[];
  source_provenance: SourceRef[];
  source_precedence: SourcePrecedenceRule[];
  user_overrides: UserOverride[];
  visual_style: VisualStyle;
  overlay_window: OverlayWindowConfig;
}

export type RuntimeEvent =
  | { type: "layer-stack-changed"; layer_stack: LayerActivation[] }
  | { type: "backend-health-changed"; health: BackendHealth }
  | { type: "pressed-keys-changed"; pressed_keys: string[] };

export interface ImportSummary {
  imported_keys: number;
  imported_layers: number;
  preserved_sections: string[];
}

export interface ImportCandidate {
  id: string;
  source: Source;
  best_effort_preview: boolean;
  preview_profile: Profile;
  conflicts: SourceConflict[];
  summary: ImportSummary;
}
