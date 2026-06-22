// TypeScript mirror of the `keyplane-core` DTOs serialized by serde.
//
// These match the JSON shapes produced by Rust (tagged enums use the same tag
// keys and kebab-case variant names). The frontend renders these and never
// re-implements keyboard logic (ADR 0036).

export type RawAction = { source: string; value: string | number };

export type LayerSwitch = "momentary" | "toggle" | "tap" | "one-shot" | "default";

export type SemanticAction =
  | { kind: "key"; label: string }
  | { kind: "modifier"; label: string }
  | { kind: "layer"; switch: LayerSwitch; layer: string; tap?: string }
  | { kind: "tap-hold"; tap: string; hold: string }
  | { kind: "macro"; label: string }
  | { kind: "transparent" }
  | { kind: "none" }
  | { kind: "mouse"; label: string }
  | { kind: "unknown"; raw: string };

export interface DisplayLegend {
  primary?: string;
  shifted?: string;
  tap?: string;
  hold?: string;
  layer?: string;
  action?: string;
  icon?: string;
}

export interface KeyGeometry {
  x: number;
  y: number;
  w: number;
  h: number;
  rotation?: number;
  rotation_origin?: [number, number];
}

export interface ResolvedKey {
  key: string;
  geometry: KeyGeometry;
  effective: SemanticAction;
  legend: DisplayLegend;
  source_layer: string;
  inherited: boolean;
}

export type Confidence = "authoritative" | "inferred" | "unknown";

export type ActivationKind =
  | "default"
  | "momentary"
  | "toggle"
  | "tap-hold"
  | "lock"
  | "remapper"
  | "unknown";

export interface ActiveLayer {
  layer: string;
  activation: ActivationKind;
}

export interface LayerStack {
  active: ActiveLayer[];
}

export interface LayerInfo {
  id: string;
  index: number;
  name?: string;
}

export type HealthState =
  | { state: "ok" }
  | { state: "permission-missing"; permission: string; detail: string }
  | { state: "disconnected"; detail: string }
  | { state: "stale"; detail: string }
  | { state: "unsupported"; detail: string }
  | { state: "parse-error"; detail: string }
  | { state: "protocol-error"; detail: string };

export interface BackendHealth {
  backend_id: string;
  name: string;
  capabilities: string[];
  health: HealthState;
}

export interface VisualStyle {
  variant: "detailed" | "minimal";
  show_inherited_indicator: boolean;
  opacity: number;
  accent?: string;
  keycap_color?: string;
  text_color?: string;
}

export interface KeyboardSnapshot {
  schema: number;
  keyboard_name?: string;
  extent: [number, number];
  style: VisualStyle;
  layers: LayerInfo[];
  layer_stack: LayerStack;
  confidence: Confidence;
  keys: ResolvedKey[];
  backends: BackendHealth[];
}

export type RuntimeEvent =
  | {
      type: "layer-stack";
      layer_stack: LayerStack;
      confidence: Confidence;
      keys: ResolvedKey[];
    }
  | { type: "pressed-keys"; pressed: string[] }
  | { type: "backend-health"; health: BackendHealth };

export interface FieldConflict {
  field: string;
  current?: { value: string; provenance: Provenance };
  incoming: { value: string; provenance: Provenance };
  winner: Provenance;
}

export interface Provenance {
  source: string;
  kind: string;
  raw?: string;
}

export interface ImportReview {
  source?: { id: string; kind: string; label?: string };
  additions: number;
  conflicts: FieldConflict[];
  notes: string[];
  best_effort_preview: boolean;
}
