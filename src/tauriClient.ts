import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  HostInputEvent,
  ImportCandidate,
  KeyboardSnapshot,
  KeyPeekDeviceDiscovery,
  Profile,
  RuntimeEvent,
  RuntimeState,
  SourceConflict,
  StyleDensity,
  VisibilityPolicy,
} from "./domain";
import { fakeSnapshot } from "./fixtures";
import { promoteSourceCandidate, resolveEffectiveKeys } from "./state";

export const runtimeEventName = "runtime-event";

export type OverlayResizeDirection =
  | "east"
  | "north"
  | "north-east"
  | "north-west"
  | "south"
  | "south-east"
  | "south-west"
  | "west";

export interface KeyPeekConnectionRequest {
  vid: string;
  pid: string;
}

export interface KanataConnectionRequest {
  host: string;
  port: number;
}

export type LaunchAtLoginState = boolean | null;

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

export async function ingestSentinelHostInputEvent(
  event: HostInputEvent,
): Promise<RuntimeEvent | null> {
  try {
    return await invoke<RuntimeEvent | null>("ingest_sentinel_host_input_event", { event });
  } catch {
    return null;
  }
}

export async function registerSentinelKeyShortcuts(): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("register_sentinel_key_shortcuts");
  } catch {
    return null;
  }
}

export async function unregisterSentinelKeyShortcuts(): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("unregister_sentinel_key_shortcuts");
  } catch {
    return null;
  }
}

export async function refreshHostPermissionHealth(): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("refresh_host_permission_health");
  } catch {
    return null;
  }
}

export async function requestHostInputPermissions(): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("request_host_input_permissions");
  } catch {
    return null;
  }
}

export async function discoverKeyPeekDevices(): Promise<KeyPeekDeviceDiscovery | null> {
  try {
    return await invoke<KeyPeekDeviceDiscovery>("discover_keypeek_devices");
  } catch {
    return {
      devices: [],
      snapshot: {
        ...fakeSnapshot,
        backends: fakeSnapshot.backends.map((backend) =>
          backend.id === "keypeek-live"
            ? {
                ...backend,
                health: {
                  backend_id: "keypeek-live",
                  state: "disconnected",
                  message: "KeyPeek device discovery unavailable",
                },
              }
            : backend,
        ),
        runtime_state: {
          ...fakeSnapshot.runtime_state,
          backend_health: fakeSnapshot.runtime_state.backend_health.map((health) =>
            health.backend_id === "keypeek-live"
              ? {
                  backend_id: "keypeek-live",
                  state: "disconnected",
                  message: "KeyPeek device discovery unavailable",
                }
              : health,
          ),
        },
      },
    };
  }
}

export async function startKeyPeekLiveBackend(
  request: KeyPeekConnectionRequest,
): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("start_keypeek_live_backend", { request });
  } catch {
    return null;
  }
}

export async function stopKeyPeekLiveBackend(): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("stop_keypeek_live_backend");
  } catch {
    return null;
  }
}

export async function startKanataTcpBackend(
  request: KanataConnectionRequest,
): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("start_kanata_tcp_backend", { request });
  } catch {
    return null;
  }
}

export async function stopKanataTcpBackend(): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("stop_kanata_tcp_backend");
  } catch {
    return null;
  }
}

export async function listenToRuntimeEvents(
  onEvent: (event: RuntimeEvent) => void,
): Promise<UnlistenFn | null> {
  try {
    return await listen<RuntimeEvent>(runtimeEventName, (event) => onEvent(event.payload));
  } catch {
    return null;
  }
}

export async function loadLaunchAtLogin(): Promise<LaunchAtLoginState> {
  try {
    return await isEnabled();
  } catch {
    return null;
  }
}

export async function setLaunchAtLogin(enabled: boolean): Promise<LaunchAtLoginState> {
  try {
    if (enabled) {
      await enable();
    } else {
      await disable();
    }
    return await isEnabled();
  } catch {
    return null;
  }
}

export async function setOverlayPositioningMode(
  enabled: boolean,
): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("set_overlay_positioning_mode", { enabled });
  } catch {
    return null;
  }
}

export async function setOverlayVisibilityPolicy(
  visibility: VisibilityPolicy,
): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("set_overlay_visibility_policy", { visibility });
  } catch {
    return null;
  }
}

export async function setOverlayVisible(visible: boolean): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("set_overlay_visible", { visible });
  } catch {
    return null;
  }
}

export async function setVisualStyleDensity(
  density: StyleDensity,
): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("set_visual_style_density", { density });
  } catch {
    return null;
  }
}

export async function startOverlayDrag(): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("start_overlay_drag");
  } catch {
    return null;
  }
}

export async function startOverlayResize(
  direction: OverlayResizeDirection,
): Promise<KeyboardSnapshot | null> {
  try {
    return await invoke<KeyboardSnapshot>("start_overlay_resize", { direction });
  } catch {
    return null;
  }
}

export async function importVialFile(contents: string): Promise<ImportCandidate> {
  return invoke<ImportCandidate>("import_vial_file", { contents });
}

export async function importVialDevice(request: KeyPeekConnectionRequest): Promise<ImportCandidate> {
  return invoke<ImportCandidate>("import_vial_device", { request });
}

export async function importKeyPeekQmkInfoFile(contents: string): Promise<ImportCandidate> {
  return invoke<ImportCandidate>("import_keypeek_qmk_info_file", { contents });
}

export async function importKeyvizStyleFile(contents: string): Promise<ImportCandidate> {
  return invoke<ImportCandidate>("import_keyviz_style_file", { contents });
}

export async function importOverkeysCompanionFile(contents: string): Promise<ImportCandidate> {
  return invoke<ImportCandidate>("import_overkeys_companion_file", { contents });
}

export async function importZmkKeymapFile(contents: string): Promise<ImportCandidate> {
  return invoke<ImportCandidate>("import_zmk_keymap_file", { contents });
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
    keyboard_id: profile.keyboard_id,
    profile_name: profile.name,
    sources: profile.sources,
    physical_layout: profile.physical_layout,
    keymap: profile.keymap,
    runtime_state: runtimeState,
    effective_keys: resolveEffectiveKeys(profile.keymap, runtimeState),
    backends: profile.runtime_backends,
    sentinel_keys: profile.sentinel_keys,
    source_conflicts: sourceConflicts,
    source_provenance: profile.source_provenance,
    source_precedence: profile.source_precedence,
    user_overrides: profile.user_overrides,
    visual_style: profile.visual_style,
    overlay_window: profile.overlay_window,
  };
}
