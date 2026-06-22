import type {
  EffectiveKey,
  KeyboardSnapshot,
  KeyAction,
  LogicalKeymap,
  RuntimeEvent,
  RuntimeState,
  SourceConflict,
} from "./domain";

const userOverrideSourceId = "user-overrides";

export function applyRuntimeEvent(
  snapshot: KeyboardSnapshot,
  event: RuntimeEvent,
): KeyboardSnapshot {
  const next: KeyboardSnapshot = structuredClone(snapshot);

  if (event.type === "layer-stack-changed") {
    next.runtime_state.layer_stack = event.layer_stack;
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

  const userOverride = {
    field_path: conflict.field_path,
    value: selected.value,
    reason: `Promoted from ${sourceId}`,
  };

  const existingOverride = next.user_overrides.findIndex(
    (override) => override.field_path === conflict.field_path,
  );
  if (existingOverride >= 0) {
    next.user_overrides[existingOverride] = userOverride;
  } else {
    next.user_overrides.push(userOverride);
  }

  const existingOverrideCandidate = nextConflict.candidates.find(
    (candidate) => candidate.source_id === userOverrideSourceId,
  );
  if (existingOverrideCandidate) {
    existingOverrideCandidate.value = selected.value;
  } else {
    nextConflict.candidates.push({
      source_id: userOverrideSourceId,
      value: selected.value,
      selected: false,
    });
  }

  nextConflict.selected_source_id = userOverrideSourceId;
  nextConflict.candidates = nextConflict.candidates.map((candidate) => ({
    ...candidate,
    selected: candidate.source_id === userOverrideSourceId,
  }));

  applyVisualConflictSelection(next, conflict.field_path, selected.value);

  return next;
}

function applyVisualConflictSelection(
  snapshot: KeyboardSnapshot,
  fieldPath: string,
  value: string,
) {
  if (fieldPath === ":visual/style :style/id") snapshot.visual_style.id = value;
  if (fieldPath === ":visual/style :style/variant-id") snapshot.visual_style.variant_id = value;
  if (fieldPath === ":visual/style :style/density") {
    if (value === "compact" || value === "standard" || value === "rich") {
      snapshot.visual_style.density = value;
    }
  }
  if (fieldPath === ":visual/style :style/colors :color/keycap-background") {
    snapshot.visual_style.colors.keycap_background = sourceConflictOptionalValue(value);
  }
  if (fieldPath === ":visual/style :style/colors :color/keycap-text") {
    snapshot.visual_style.colors.keycap_text = sourceConflictOptionalValue(value);
  }
  if (fieldPath === ":visual/style :style/colors :color/keycap-border") {
    snapshot.visual_style.colors.keycap_border = sourceConflictOptionalValue(value);
  }
  if (fieldPath === ":visual/style :style/colors :color/modifier-accent") {
    snapshot.visual_style.colors.modifier_accent = sourceConflictOptionalValue(value);
  }
  if (fieldPath === ":visual/style :style/colors :color/overlay-background") {
    snapshot.visual_style.colors.overlay_background = sourceConflictOptionalValue(value);
  }
}

function sourceConflictOptionalValue(value: string) {
  return value === "nil" ? null : value;
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
