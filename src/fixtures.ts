import type {
  BackendStatus,
  KeyboardSnapshot,
  KeyAction,
  Layer,
  PhysicalKey,
  RuntimeEvent,
  RuntimeState,
  Source,
  SourceRef,
} from "./domain";
import { resolveEffectiveKeys } from "./state";

const sourceId = "fake-backend";

const sourceRef = (fieldPath: string, raw: string | null = null): SourceRef => ({
  source_id: sourceId,
  field_path: fieldPath,
  raw,
});

const keys: PhysicalKey[] = [
  key("k-esc", 0, 0, "matrix:0,0"),
  key("k-q", 0, 1, "matrix:0,1"),
  key("k-w", 0, 2, "matrix:0,2"),
  key("k-e", 0, 3, "matrix:0,3"),
  key("k-a", 1, 0, "matrix:1,0", 0.28),
  key("k-s", 1, 1, "matrix:1,1", 0.28),
  key("k-d", 1, 2, "matrix:1,2", 0.28),
  key("k-f", 1, 3, "matrix:1,3", 0.28),
  key("k-shift", 2, 0, "matrix:2,0"),
  key("k-z", 2, 1, "matrix:2,1"),
  key("k-space", 2, 2, "matrix:2,2", 0, 1.7, -4),
  key("k-fn", 2, 3, "matrix:2,3", 0, 1, -4),
];

const baseLayer: Layer = {
  id: "layer-0",
  name: "Base",
  actions: [
    action("k-esc", "KC_ESC", "Esc"),
    action("k-q", "KC_Q", "Q"),
    action("k-w", "KC_W", "W"),
    action("k-e", "KC_E", "E"),
    action("k-a", "KC_A", "A"),
    action("k-s", "KC_S", "S"),
    action("k-d", "KC_D", "D"),
    action("k-f", "KC_F", "F"),
    action("k-shift", "KC_LSFT", "Shift", "modifier"),
    action("k-z", "KC_Z", "Z"),
    action("k-space", "LT(1, KC_SPC)", "Space", "layer-tap", "layer-1"),
    action("k-fn", "MO(1)", "hold", "layer-momentary", "layer-1"),
  ],
};

const navLayer: Layer = {
  id: "layer-1",
  name: "Navigation",
  actions: [
    action("k-esc", "KC_GRV", "GRV"),
    transparent("k-q"),
    action("k-w", "KC_UP", "Up"),
    transparent("k-e"),
    action("k-a", "KC_LEFT", "Left"),
    action("k-s", "KC_DOWN", "Down"),
    action("k-d", "KC_RGHT", "Right"),
    transparent("k-f"),
    transparent("k-shift"),
    transparent("k-z"),
    action("k-space", "KC_BSPC", "Backspace"),
    transparent("k-fn"),
  ],
};

const backend: BackendStatus = {
  id: sourceId,
  name: "Fake Backend",
  capabilities: ["import-geometry", "import-keymaps", "stream-layer-stack", "stream-pressed-keys"],
  health: {
    backend_id: sourceId,
    state: "ok",
    message: "Streaming deterministic layer stack events",
  },
};

const keyPeekBackend: BackendStatus = {
  id: "keypeek-live",
  name: "KeyPeek Live",
  capabilities: [
    "discover-devices",
    "import-geometry",
    "import-keymaps",
    "stream-layer-stack",
    "stream-pressed-keys",
  ],
  health: {
    backend_id: "keypeek-live",
    state: "disconnected",
    message: "No KeyPeek-compatible device is connected",
  },
};

const kanataBackend: BackendStatus = {
  id: "kanata-tcp",
  name: "Kanata TCP",
  capabilities: ["stream-layer-stack", "poll-state"],
  health: {
    backend_id: "kanata-tcp",
    state: "disconnected",
    message: "Kanata TCP runtime is not connected",
  },
};

const sentinelBackend: BackendStatus = {
  id: "sentinel-keys",
  name: "Sentinel Keys",
  capabilities: ["stream-layer-stack"],
  health: {
    backend_id: "sentinel-keys",
    state: "permission-missing",
    message: "Input monitoring permission is required before Sentinel Keys can infer layers",
  },
};

const overlayWindowBackend: BackendStatus = {
  id: "overlay-window",
  name: "Overlay Window",
  capabilities: [
    "render-overlay-window",
    "transparent-overlay-window",
    "click-through-overlay-window",
    "position-overlay-window",
  ],
  health: {
    backend_id: "overlay-window",
    state: "ok",
    message: "Overlay Window is ready",
  },
};

const sources: Source[] = [
  {
    id: sourceId,
    name: "Fake Backend",
    kind: "fake",
    authority: "authoritative",
  },
  {
    id: "keypeek-live",
    name: "KeyPeek Live",
    kind: "keypeek-firmware",
    authority: "authoritative",
  },
  {
    id: "kanata-tcp",
    name: "Kanata TCP",
    kind: "kanata",
    authority: "authoritative",
  },
  {
    id: "sentinel-keys",
    name: "Sentinel Keys",
    kind: "sentinel-keys",
    authority: "inferred",
  },
];

const runtimeState: RuntimeState = {
  layer_stack: [
    {
      layer_id: "layer-0",
      kind: "default",
      confidence: {
        level: "high",
        reason: "Default layer from fake backend",
      },
    },
  ],
  pressed_keys: [],
  backend_health: [
    backend.health,
    overlayWindowBackend.health,
    keyPeekBackend.health,
    kanataBackend.health,
    sentinelBackend.health,
  ],
};

const visualStyleProvenance: SourceRef[] = [
  {
    source_id: sourceId,
    field_path: ":visual/style",
    raw: "variant=keyplane-default",
  },
  {
    source_id: "keyviz-import",
    field_path: ":visual/style",
    raw: '{"style":"minimal"}',
  },
];

export const fakeSnapshot: KeyboardSnapshot = {
  profile_id: "profile-keyplane-demo",
  profile_name: "Keyplane Demo",
  sources,
  physical_layout: {
    keys,
    fallback: false,
  },
  keymap: {
    layers: [baseLayer, navLayer],
  },
  runtime_state: runtimeState,
  effective_keys: resolveEffectiveKeys({ layers: [baseLayer, navLayer] }, runtimeState),
  backends: [backend, overlayWindowBackend, keyPeekBackend, kanataBackend, sentinelBackend],
  sentinel_keys: [
    {
      host_input_code: "F24",
      layer_id: "layer-1",
      activation: "momentary",
    },
  ],
  source_conflicts: [
    {
      field_path: ":visual/style :style/variant-id",
      selected_source_id: sourceId,
      candidates: [
        { source_id: sourceId, value: "keyplane-default", selected: true },
        { source_id: "keyviz-import", value: "keyviz-minimal", selected: false },
      ],
    },
  ],
  source_provenance: [...keys.map((key) => key.provenance), ...visualStyleProvenance],
  source_precedence: [
    {
      field_scope: ":visual/style",
      source_order: [sourceId, "keyviz-import"],
    },
  ],
  user_overrides: [],
  visual_style: {
    variant_id: "keyplane-default",
    density: "rich",
    colors: {
      keycap_background: null,
      keycap_text: null,
      keycap_border: null,
      modifier_accent: null,
      overlay_background: null,
    },
  },
  overlay_window: {
    visibility: "pinned",
    visible: true,
    click_through: true,
    positioning_mode: false,
    display_targeting: {
      display_id: null,
      x: 72,
      y: 72,
      width: 940,
      height: 320,
      opacity: 0.92,
    },
  },
};

export const navLayerEvent: RuntimeEvent = {
  type: "layer-stack-changed",
  layer_stack: [
    {
      layer_id: "layer-1",
      kind: "momentary",
      confidence: {
        level: "high",
        reason: "Momentary layer from fake backend Runtime Event",
      },
    },
    {
      layer_id: "layer-0",
      kind: "default",
      confidence: {
        level: "high",
        reason: "Default layer from fake backend",
      },
    },
  ],
};

function key(
  id: string,
  row: number,
  col: number,
  raw: string,
  xOffset = 0,
  width = 1,
  rotation = 0,
): PhysicalKey {
  return {
    id,
    matrix: { row, col },
    geometry: {
      x: col * 1.08 + xOffset,
      y: row * 1.05,
      width,
      height: 1,
      rotation,
    },
    provenance: sourceRef(`:keyboard/physical-layout ${id}`, raw),
  };
}

function action(
  keyId: string,
  raw: string,
  label: string,
  kind: KeyAction["semantic"]["kind"] = "key",
  targetLayer: string | null = null,
): KeyAction {
  return {
    key_id: keyId,
    raw: {
      dialect: "qmk",
      value: raw,
    },
    semantic: {
      kind,
      label,
      target_layer: targetLayer,
      hold_label: null,
    },
    legend: {
      slots: [
        { slot: "primary", text: label },
        ...(targetLayer ? [{ slot: "layer-hint" as const, text: targetLayer }] : []),
      ],
    },
    provenance: sourceRef(`:keyboard/keymap ${keyId}`, raw),
  };
}

function transparent(keyId: string): KeyAction {
  return action(keyId, "KC_TRNS", "trans", "transparent");
}
