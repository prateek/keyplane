// Thin Tauri bridge: command invocations and runtime-event subscription.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ImportReview, KeyboardSnapshot, RuntimeEvent } from "./types";

export const EVENT_RUNTIME = "keyplane://runtime-event";
export const EVENT_SNAPSHOT = "keyplane://snapshot";
export const EVENT_POSITIONING = "keyplane://positioning";

export const getSnapshot = () => invoke<KeyboardSnapshot>("get_snapshot");
export const activeProfileEdn = () => invoke<string>("active_profile_edn");
export const applyProfileEdn = (edn: string) =>
  invoke<unknown>("apply_profile_edn", { edn });
export const importPreview = (format: string, contents: string) =>
  invoke<ImportReview>("import_preview", { format, contents });
export const commitImport = (format: string, contents: string) =>
  invoke<unknown>("commit_import", { format, contents });
export const promoteOverride = (field: string, value: unknown, note?: string) =>
  invoke<unknown>("promote_override", { field, value, note: note ?? null });
export const connectKeypeek = (args: {
  kind: "vial" | "via" | "zmk";
  vid?: number;
  pid?: number;
  jsonPath?: string;
  layout?: string;
  serialPort?: string;
  bleId?: string;
}) =>
  invoke<unknown>("connect_keypeek", {
    kind: args.kind,
    vid: args.vid ?? null,
    pid: args.pid ?? null,
    jsonPath: args.jsonPath ?? null,
    layout: args.layout ?? null,
    serialPort: args.serialPort ?? null,
    bleId: args.bleId ?? null,
  });
export const connectKanata = (port: number, host?: string) =>
  invoke<unknown>("connect_kanata", { host: host ?? null, port });
export const connectSentinel = (
  keys: { host_key: string; action: "momentary" | "toggle"; layer: string }[],
  osCapture = false,
) => invoke<unknown>("connect_sentinel", { keys, osCapture });
export const feedHostEvent = (key: string, down: boolean) =>
  invoke<unknown>("feed_host_event", { key, down });
export const inputMonitoringStatus = () =>
  invoke<boolean>("input_monitoring_status");
export const requestInputMonitoring = () =>
  invoke<boolean>("request_input_monitoring");
export const setDisplayTargeting = (args: {
  x?: number;
  y?: number;
  width?: number;
  height?: number;
}) =>
  invoke<unknown>("set_display_targeting", {
    x: args.x ?? null,
    y: args.y ?? null,
    width: args.width ?? null,
    height: args.height ?? null,
  });
export const setPositioningMode = (enabled: boolean) =>
  invoke<void>("set_positioning_mode", { enabled });
export const setOverlayVisible = (visible: boolean) =>
  invoke<void>("set_overlay_visible", { visible });
export const setAutostart = (enabled: boolean) =>
  invoke<void>("set_autostart", { enabled });
export const getAutostart = () => invoke<boolean>("get_autostart");

/** Subscribe to streamed Runtime Events; returns an unlisten function. */
export const onRuntimeEvent = (handler: (event: RuntimeEvent) => void) =>
  listen<RuntimeEvent>(EVENT_RUNTIME, (e) => handler(e.payload));

/** Subscribe to full snapshot replacements (after imports / hand edits). */
export const onSnapshot = (handler: (snapshot: KeyboardSnapshot) => void) =>
  listen<KeyboardSnapshot>(EVENT_SNAPSHOT, (e) => handler(e.payload));

/** Subscribe to Positioning Mode toggles (overlay shows drag/resize handles). */
export const onPositioning = (handler: (enabled: boolean) => void) =>
  listen<boolean>(EVENT_POSITIONING, (e) => handler(e.payload));

/** Start dragging the overlay window from a pointer-down (Positioning Mode). */
export const startOverlayDrag = () => getCurrentWindow().startDragging();

/** Start resizing the overlay window from its bottom-right corner.
 * `ResizeDirection` isn't re-exported by this API version, so the direction
 * string is passed through a typed cast. */
export const startOverlayResize = () =>
  (
    getCurrentWindow().startResizeDragging as (d: string) => Promise<void>
  )("SouthEast");
