// Thin Tauri bridge: command invocations and runtime-event subscription.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ImportReview, KeyboardSnapshot, RuntimeEvent } from "./types";

export const EVENT_RUNTIME = "keyplane://runtime-event";
export const EVENT_SNAPSHOT = "keyplane://snapshot";

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
  kind: "vial" | "via";
  vid?: number;
  pid?: number;
  jsonPath?: string;
  layout?: string;
}) =>
  invoke<unknown>("connect_keypeek", {
    kind: args.kind,
    vid: args.vid ?? null,
    pid: args.pid ?? null,
    jsonPath: args.jsonPath ?? null,
    layout: args.layout ?? null,
  });
export const setPositioningMode = (enabled: boolean) =>
  invoke<void>("set_positioning_mode", { enabled });
export const setOverlayVisible = (visible: boolean) =>
  invoke<void>("set_overlay_visible", { visible });

/** Subscribe to streamed Runtime Events; returns an unlisten function. */
export const onRuntimeEvent = (handler: (event: RuntimeEvent) => void) =>
  listen<RuntimeEvent>(EVENT_RUNTIME, (e) => handler(e.payload));

/** Subscribe to full snapshot replacements (after imports / hand edits). */
export const onSnapshot = (handler: (snapshot: KeyboardSnapshot) => void) =>
  listen<KeyboardSnapshot>(EVENT_SNAPSHOT, (e) => handler(e.payload));
