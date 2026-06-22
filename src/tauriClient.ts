import { invoke } from "@tauri-apps/api/core";
import type {
  ImportCandidate,
  KeyboardSnapshot,
  Profile,
  RuntimeEvent,
  RuntimeState,
  SourceConflict,
} from "./domain";
import { fakeSnapshot } from "./fixtures";
import { promoteSourceCandidate, resolveEffectiveKeys } from "./state";

export type OverlayResizeDirection =
  | "east"
  | "north"
  | "north-east"
  | "north-west"
  | "south"
  | "south-east"
  | "south-west"
  | "west";

export async function loadInitialSnapshot(): Promise<KeyboardSnapshot> {
  try {
    return await invoke<KeyboardSnapshot>("initial_snapshot");
  } catch {
    return fakeSnapshot;
  }
}

export async function loadFakeRuntimeEvents(): Promise<RuntimeEvent[]> {
  try {
    return await invoke<RuntimeEvent[]>("fake_runtime_events");
  } catch {
    return [];
  }
}

export async function setOverlayPositioningMode(enabled: boolean): Promise<void> {
  try {
    await invoke("set_overlay_positioning_mode", { enabled });
  } catch {
    return;
  }
}

export async function startOverlayDrag(): Promise<void> {
  try {
    await invoke("start_overlay_drag");
  } catch {
    return;
  }
}

export async function startOverlayResize(direction: OverlayResizeDirection): Promise<void> {
  try {
    await invoke("start_overlay_resize", { direction });
  } catch {
    return;
  }
}

export async function importVialFile(contents: string): Promise<ImportCandidate> {
  return invoke<ImportCandidate>("import_vial_file", { contents });
}

export async function importKeyvizStyleFile(contents: string): Promise<ImportCandidate> {
  return invoke<ImportCandidate>("import_keyviz_style_file", { contents });
}

export async function importOverkeysCompanionFile(contents: string): Promise<ImportCandidate> {
  return invoke<ImportCandidate>("import_overkeys_companion_file", { contents });
}

export async function commitImportCandidate(candidate: ImportCandidate): Promise<KeyboardSnapshot> {
  try {
    return await invoke<KeyboardSnapshot>("commit_import_candidate", { candidate });
  } catch {
    return snapshotFromProfile(candidate.preview_profile, candidate.conflicts);
  }
}

export async function promoteActiveSourceCandidate(
  snapshot: KeyboardSnapshot,
  conflict: SourceConflict,
  sourceId: string,
): Promise<KeyboardSnapshot> {
  try {
    return await invoke<KeyboardSnapshot>("promote_source_candidate", {
      conflict,
      sourceId,
    });
  } catch {
    return promoteSourceCandidate(snapshot, conflict, sourceId);
  }
}

export async function saveActiveProfileEdn(): Promise<string> {
  return invoke<string>("save_active_profile_edn");
}

export async function loadActiveProfileEdn(contents: string): Promise<KeyboardSnapshot> {
  return invoke<KeyboardSnapshot>("load_active_profile_edn", { contents });
}

function snapshotFromProfile(
  profile: Profile,
  sourceConflicts: SourceConflict[],
): KeyboardSnapshot {
  const hasRuntimeBackends = profile.runtime_backends.length > 0;
  const hasAuthoritativeRuntime = profile.runtime_backends.some(
    (backend) => backend.health.state === "ok" && !backend.capabilities.includes("preview-only"),
  );
  const runtimeState: RuntimeState = {
    layer_stack: profile.keymap.layers[0]
      ? [
          {
            layer_id: profile.keymap.layers[0].id,
            kind: "default",
            confidence: {
              level: !hasRuntimeBackends ? "low" : hasAuthoritativeRuntime ? "high" : "medium",
              reason: !hasRuntimeBackends
                ? "Active Profile has no runtime backend"
                : hasAuthoritativeRuntime
                  ? "Active Profile default layer"
                  : "Best-Effort Preview default layer",
            },
          },
        ]
      : [],
    pressed_keys: [],
    backend_health: profile.runtime_backends.map((backend) => backend.health),
  };

  return {
    profile_id: profile.id,
    profile_name: profile.name,
    physical_layout: profile.physical_layout,
    keymap: profile.keymap,
    runtime_state: runtimeState,
    effective_keys: resolveEffectiveKeys(profile.keymap, runtimeState),
    backends: profile.runtime_backends,
    source_conflicts: sourceConflicts,
    source_precedence: profile.source_precedence,
    user_overrides: profile.user_overrides,
    visual_style: profile.visual_style,
    overlay_window: profile.overlay_window,
  };
}
