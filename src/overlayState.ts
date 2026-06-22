// Pure overlay-state reducer — the frontend's testable seam.
//
// Rust owns resolution, so a layer-stack event already carries re-resolved
// keys; the reducer just swaps the rendered values and tracks confidence and
// backend health. Keeping this pure (no DOM, no Tauri) makes it unit-testable
// against serialized fixtures (PRD frontend contract tests).

import type {
  BackendHealth,
  KeyboardSnapshot,
  LayerStack,
  ResolvedKey,
  RuntimeEvent,
} from "./types";

export interface OverlayState {
  snapshot: KeyboardSnapshot;
  pressed: Set<string>;
}

export function initialState(snapshot: KeyboardSnapshot): OverlayState {
  return { snapshot, pressed: new Set() };
}

/** Apply one Runtime Event, returning a new state (no mutation). */
export function applyEvent(state: OverlayState, event: RuntimeEvent): OverlayState {
  switch (event.type) {
    case "layer-stack":
      return {
        ...state,
        snapshot: {
          ...state.snapshot,
          layer_stack: event.layer_stack,
          confidence: event.confidence,
          keys: event.keys,
        },
      };
    case "pressed-keys":
      return { ...state, pressed: new Set(event.pressed) };
    case "backend-health":
      return {
        ...state,
        snapshot: {
          ...state.snapshot,
          backends: upsertBackend(state.snapshot.backends, event.health),
        },
      };
  }
}

function upsertBackend(backends: BackendHealth[], next: BackendHealth): BackendHealth[] {
  const i = backends.findIndex((b) => b.backend_id === next.backend_id);
  if (i === -1) return [...backends, next];
  const copy = backends.slice();
  copy[i] = next;
  return copy;
}

/** The id of the topmost active layer, which the overlay highlights. */
export function topLayer(stack: LayerStack): string | undefined {
  return stack.active.at(-1)?.layer;
}

/** Collapse a key's structured legend into a single label (minimal variant). */
export function collapseLegend(key: ResolvedKey): string {
  const l = key.legend;
  return l.tap ?? l.primary ?? l.layer ?? l.action ?? l.hold ?? l.shifted ?? l.icon ?? "";
}
