import type {
  EffectiveKey,
  ImportCandidate,
  KeyboardSnapshot,
  KeyAction,
  LogicalKeymap,
  PhysicalLayout,
  RuntimeEvent,
  RuntimeState,
  SourceConflict,
  UserOverride,
  VisualStyle,
} from "./domain";

const userOverrideSourceId = "user-overrides";

export function applyRuntimeEvent(
  snapshot: KeyboardSnapshot,
  event: RuntimeEvent,
): KeyboardSnapshot {
  const next: KeyboardSnapshot = structuredClone(snapshot);

  if (event.type === "layer-stack-changed") {
    if (shouldApplyRuntimeStateEvent(next, event.source_id ?? null)) {
      next.runtime_state.layer_stack = event.layer_stack;
      next.runtime_state.layer_stack_source_id = event.source_id ?? null;
    }
  }

  if (event.type === "backend-health-changed") {
    const existing = next.runtime_state.backend_health.findIndex(
      (health) => health.backend_id === event.health.backend_id,
    );
    if (existing >= 0) {
      next.runtime_state.backend_health[existing] = event.health;
    } else {
      next.runtime_state.backend_health.push(event.health);
    }
    const backend = next.backends.find((item) => item.id === event.health.backend_id);
    if (backend) backend.health = event.health;
  }

  if (event.type === "pressed-keys-changed") {
    next.runtime_state.pressed_keys = event.pressed_keys;
  }

  next.effective_keys = resolveEffectiveKeys(next.keymap, next.runtime_state);
  return next;
}

function shouldApplyRuntimeStateEvent(
  snapshot: KeyboardSnapshot,
  incomingSourceId: string | null,
) {
  if (!incomingSourceId) return true;
  const currentSourceId = snapshot.runtime_state.layer_stack_source_id;
  if (!currentSourceId || currentSourceId === incomingSourceId) return true;

  const incomingRank = runtimeStateSourceRank(snapshot, incomingSourceId);
  const currentRank = runtimeStateSourceRank(snapshot, currentSourceId);

  if (incomingRank !== null && currentRank !== null) return incomingRank <= currentRank;
  if (incomingRank !== null && currentRank === null) return true;
  if (incomingRank === null && currentRank !== null) return false;
  return true;
}

function runtimeStateSourceRank(snapshot: KeyboardSnapshot, sourceId: string): number | null {
  const rule = snapshot.source_precedence
    .filter((candidate) => fieldPathMatchesScope(":runtime/state", candidate.field_scope))
    .sort((left, right) => right.field_scope.length - left.field_scope.length)[0];
  if (!rule) return null;

  const rank = rule.source_order.findIndex((candidate) => candidate === sourceId);
  return rank >= 0 ? rank : null;
}

function fieldPathMatchesScope(fieldPath: string, fieldScope: string) {
  return fieldPath === fieldScope || fieldPath.startsWith(`${fieldScope} `);
}

export function promoteSourceCandidate(
  snapshot: KeyboardSnapshot,
  conflict: SourceConflict,
  sourceId: string,
): KeyboardSnapshot {
  const selected = conflict.candidates.find((candidate) => candidate.source_id === sourceId);
  if (!selected) return snapshot;

  const next: KeyboardSnapshot = structuredClone(snapshot);
  const nextConflict = next.source_conflicts.find(
    (candidate) => candidate.field_path === conflict.field_path,
  );
  if (!nextConflict) return snapshot;

  promoteConflictSelection(
    next.user_overrides,
    nextConflict,
    conflict.field_path,
    selected.value,
    sourceId,
  );
  applyPhysicalLayoutSelection(
    next.physical_layout,
    conflict.field_path,
    selected.value,
    userOverrideSourceId,
  );
  applyVisualStyleSelection(next.visual_style, conflict.field_path, selected.value);
  applyKeymapSelection(next.keymap, conflict.field_path, selected.value, userOverrideSourceId);
  next.effective_keys = resolveEffectiveKeys(next.keymap, next.runtime_state);

  return next;
}

export function promoteImportCandidateSource(
  candidate: ImportCandidate,
  conflict: SourceConflict,
  sourceId: string,
): ImportCandidate {
  const selected = conflict.candidates.find((candidate) => candidate.source_id === sourceId);
  if (!selected) return candidate;

  const next: ImportCandidate = structuredClone(candidate);
  const nextConflict = next.conflicts.find(
    (candidate) => candidate.field_path === conflict.field_path,
  );
  if (!nextConflict) return candidate;

  promoteConflictSelection(
    next.preview_profile.user_overrides,
    nextConflict,
    conflict.field_path,
    selected.value,
    sourceId,
  );
  applyPhysicalLayoutSelection(
    next.preview_profile.physical_layout,
    conflict.field_path,
    selected.value,
    userOverrideSourceId,
  );
  applyVisualStyleSelection(next.preview_profile.visual_style, conflict.field_path, selected.value);
  applyKeymapSelection(
    next.preview_profile.keymap,
    conflict.field_path,
    selected.value,
    userOverrideSourceId,
  );

  return next;
}

function promoteConflictSelection(
  userOverrides: UserOverride[],
  conflict: SourceConflict,
  fieldPath: string,
  value: string,
  sourceId: string,
) {
  const userOverride = {
    field_path: fieldPath,
    value,
    reason: `Promoted from ${sourceId}`,
  };

  const existingOverride = userOverrides.findIndex((override) => override.field_path === fieldPath);
  if (existingOverride >= 0) {
    userOverrides[existingOverride] = userOverride;
  } else {
    userOverrides.push(userOverride);
  }

  const existingOverrideCandidate = conflict.candidates.find(
    (candidate) => candidate.source_id === userOverrideSourceId,
  );
  if (existingOverrideCandidate) {
    existingOverrideCandidate.value = value;
  } else {
    conflict.candidates.push({
      source_id: userOverrideSourceId,
      value,
      selected: false,
    });
  }

  conflict.selected_source_id = userOverrideSourceId;
  conflict.candidates = conflict.candidates.map((candidate) => ({
    ...candidate,
    selected: candidate.source_id === userOverrideSourceId,
  }));
}

function applyVisualStyleSelection(
  visualStyle: VisualStyle,
  fieldPath: string,
  value: string,
) {
  if (fieldPath === ":visual/style :style/id") visualStyle.id = value;
  if (fieldPath === ":visual/style :style/variant-id") visualStyle.variant_id = value;
  if (fieldPath === ":visual/style :style/density") {
    if (value === "compact" || value === "standard" || value === "rich") {
      visualStyle.density = value;
    }
  }
  if (fieldPath === ":visual/style :style/colors :color/keycap-background") {
    visualStyle.colors.keycap_background = sourceConflictOptionalValue(value);
  }
  if (fieldPath === ":visual/style :style/colors :color/keycap-text") {
    visualStyle.colors.keycap_text = sourceConflictOptionalValue(value);
  }
  if (fieldPath === ":visual/style :style/colors :color/keycap-border") {
    visualStyle.colors.keycap_border = sourceConflictOptionalValue(value);
  }
  if (fieldPath === ":visual/style :style/colors :color/modifier-accent") {
    visualStyle.colors.modifier_accent = sourceConflictOptionalValue(value);
  }
  if (fieldPath === ":visual/style :style/colors :color/overlay-background") {
    visualStyle.colors.overlay_background = sourceConflictOptionalValue(value);
  }
}

function sourceConflictOptionalValue(value: string) {
  return value === "nil" ? null : value;
}

function applyPhysicalLayoutSelection(
  physicalLayout: PhysicalLayout,
  fieldPath: string,
  value: string,
  sourceId: string,
) {
  const target = physicalLayoutFieldPath(fieldPath);
  if (!target) return;

  const key = physicalLayout.keys.find((candidate) => candidate.id === target.keyId);
  if (!key) return;

  if (target.field === "matrix-row" || target.field === "matrix-col") {
    const matrixCoordinate = sourceConflictMatrixCoordinate(value);
    if (matrixCoordinate === null) return;
    key.matrix ??= { row: 0, col: 0 };
    if (target.field === "matrix-row") key.matrix.row = matrixCoordinate;
    if (target.field === "matrix-col") key.matrix.col = matrixCoordinate;
  } else {
    const numericValue = sourceConflictFiniteNumber(value);
    if (numericValue === null) return;

    if (target.field === "geometry-x") key.geometry.x = numericValue;
    if (target.field === "geometry-y") key.geometry.y = numericValue;
    if (target.field === "geometry-width") key.geometry.width = numericValue;
    if (target.field === "geometry-height") key.geometry.height = numericValue;
    if (target.field === "geometry-rotation") key.geometry.rotation = numericValue;
  }

  key.provenance = {
    source_id: sourceId,
    field_path: fieldPath,
    raw: value,
  };
}

function sourceConflictFiniteNumber(value: string): number | null {
  if (value.trim() === "") return null;
  const numericValue = Number(value);
  return Number.isFinite(numericValue) ? numericValue : null;
}

function sourceConflictMatrixCoordinate(value: string): number | null {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) return null;
  const numericValue = Number(trimmed);
  return Number.isInteger(numericValue) && numericValue <= 65535 ? numericValue : null;
}

type PhysicalLayoutTarget = {
  keyId: string;
  field:
    | "geometry-x"
    | "geometry-y"
    | "geometry-width"
    | "geometry-height"
    | "geometry-rotation"
    | "matrix-row"
    | "matrix-col";
};

function physicalLayoutFieldPath(fieldPath: string): PhysicalLayoutTarget | null {
  const parts = fieldPath.trim().split(/\s+/);
  if (parts.length !== 3 || parts[0] !== ":keyboard/physical-layout") return null;

  const field = {
    ":geometry/x": "geometry-x",
    ":geometry/y": "geometry-y",
    ":geometry/width": "geometry-width",
    ":geometry/height": "geometry-height",
    ":geometry/rotation": "geometry-rotation",
    ":matrix/row": "matrix-row",
    ":matrix/col": "matrix-col",
  }[parts[2]] as PhysicalLayoutTarget["field"] | undefined;

  return field ? { keyId: parts[1], field } : null;
}

function applyKeymapSelection(
  keymap: LogicalKeymap,
  fieldPath: string,
  value: string,
  sourceId: string,
) {
  const target = keymapActionFieldPath(fieldPath);
  if (!target) return;

  const layer = keymap.layers.find((candidate) => candidate.id === target.layerId);
  const action = layer?.actions.find((candidate) => candidate.key_id === target.keyId);
  if (!action) return;

  const dialect = action.raw.dialect;
  const nextAction = deriveFrontendAction(dialect, value, target.keyId, sourceId, fieldPath);
  Object.assign(action, nextAction);
}

function keymapActionFieldPath(fieldPath: string): { layerId: string; keyId: string } | null {
  const parts = fieldPath.trim().split(/\s+/);
  if (parts[0] !== ":keyboard/keymap") return null;
  if (parts.length !== 3 && !(parts.length === 4 && parts[3] === ":action/raw")) return null;

  return {
    layerId: parts[1].replace(/^:/, ""),
    keyId: parts[2],
  };
}

function deriveFrontendAction(
  dialect: string,
  rawValue: string,
  keyId: string,
  sourceId: string,
  fieldPath: string,
): KeyAction {
  const semantic = deriveFrontendSemantic(rawValue);
  const slots: KeyAction["legend"]["slots"] = [{ slot: "primary", text: semantic.label }];

  if (semantic.target_layer) {
    slots.push({ slot: "layer-hint" as const, text: semantic.target_layer });
  }

  return {
    key_id: keyId,
    raw: {
      dialect,
      value: rawValue,
    },
    semantic,
    legend: { slots },
    provenance: {
      source_id: sourceId,
      field_path: fieldPath,
      raw: rawValue,
    },
  };
}

function deriveFrontendSemantic(rawValue: string): KeyAction["semantic"] {
  const value = rawValue.trim();
  const upper = value.toUpperCase();

  if (upper === "KC_TRNS" || upper === "_______" || upper === "TRANSPARENT" || upper === "&TRANS") {
    return { kind: "transparent", label: "trans", target_layer: null, hold_label: null };
  }

  if (upper === "KC_NO" || upper === "XXXXXXX" || upper === "NONE" || upper === "&NONE") {
    return { kind: "none", label: "none", target_layer: null, hold_label: null };
  }

  const momentaryLayer = wrappedArg(value, "MO") ?? spaceSeparatedArg(value, "&mo");
  if (momentaryLayer) {
    return {
      kind: "layer-momentary",
      label: "hold",
      target_layer: `layer-${momentaryLayer}`,
      hold_label: null,
    };
  }

  const toggleLayer = wrappedArg(value, "TG") ?? spaceSeparatedArg(value, "&tog");
  if (toggleLayer) {
    return {
      kind: "layer-toggle",
      label: "toggle",
      target_layer: `layer-${toggleLayer}`,
      hold_label: null,
    };
  }

  if (upper.startsWith("KC_MS_") || upper.startsWith("MS_")) {
    return {
      kind: "mouse",
      label: keyLabel(value.replace(/^KC_MS_/i, "").replace(/^MS_/i, "")),
      target_layer: null,
      hold_label: null,
    };
  }

  if (upper.startsWith("MACRO") || upper.startsWith("DM_")) {
    return { kind: "macro", label: "macro", target_layer: null, hold_label: null };
  }

  if (
    ["KC_LCTL", "KC_RCTL", "KC_LSFT", "KC_RSFT", "KC_LALT", "KC_RALT", "KC_LGUI", "KC_RGUI"].includes(
      upper,
    )
  ) {
    return { kind: "modifier", label: keyLabel(value), target_layer: null, hold_label: null };
  }

  if (upper.startsWith("KC_") || upper.startsWith("&KP ")) {
    return { kind: "key", label: keyLabel(value), target_layer: null, hold_label: null };
  }

  return { kind: "unknown", label: value, target_layer: null, hold_label: null };
}

function wrappedArg(value: string, name: string): string | null {
  const prefix = `${name}(`;
  if (!value.startsWith(prefix) || !value.endsWith(")")) return null;
  return value.slice(prefix.length, -1).trim();
}

function spaceSeparatedArg(value: string, behavior: string): string | null {
  const [candidate, target] = value.trim().split(/\s+/);
  return candidate?.toLowerCase() === behavior.toLowerCase() && target ? target : null;
}

function keyLabel(value: string): string {
  const token = value.trim().toUpperCase().replace(/^KC_/, "").replace(/^&KP\s+/, "");
  const labels: Record<string, string> = {
    ESC: "Esc",
    ENT: "Enter",
    ENTER: "Enter",
    BSPC: "Backspace",
    SPC: "Space",
    SPACE: "Space",
    TAB: "Tab",
    DEL: "Del",
    LCTL: "Ctrl",
    RCTL: "Ctrl",
    LSFT: "Shift",
    RSFT: "Shift",
    LALT: "Alt",
    RALT: "Alt",
    LGUI: "Cmd",
    RGUI: "Cmd",
    LEFT: "Left",
    RGHT: "Right",
    RIGHT: "Right",
    UP: "Up",
    DOWN: "Down",
  };

  return labels[token] ?? (token.length === 1 ? token : token.split("_").join(" "));
}

export function resolveEffectiveKeys(
  keymap: LogicalKeymap,
  runtimeState: RuntimeState,
): EffectiveKey[] {
  const baseLayer = keymap.layers[0];
  if (!baseLayer) return [];

  return baseLayer.actions.map((baseAction) => {
    return resolveEffectiveKey(baseAction.key_id, keymap, runtimeState) ?? {
      key_id: baseAction.key_id,
      raw: baseAction.raw,
      semantic: baseAction.semantic,
      legend: baseAction.legend,
      source_layer_id: baseLayer.id,
      inherited: false,
    };
  });
}

function resolveEffectiveKey(
  keyId: string,
  keymap: LogicalKeymap,
  runtimeState: RuntimeState,
): EffectiveKey | null {
  let inherited = false;

  for (const activation of runtimeState.layer_stack) {
    const layer = keymap.layers.find((candidate) => candidate.id === activation.layer_id);
    if (!layer) {
      inherited = true;
      continue;
    }

    const action = layer.actions.find((candidate) => candidate.key_id === keyId);

    if (!action) {
      inherited = true;
      continue;
    }

    if (action.semantic.kind === "transparent") {
      inherited = true;
      continue;
    }

    return fromAction(action, layer.id, inherited);
  }

  return null;
}

function fromAction(action: KeyAction, sourceLayerId: string, inherited: boolean): EffectiveKey {
  return {
    key_id: action.key_id,
    raw: action.raw,
    semantic: action.semantic,
    legend: action.legend,
    source_layer_id: sourceLayerId,
    inherited,
  };
}
