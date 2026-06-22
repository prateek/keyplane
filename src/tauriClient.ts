import { invoke } from "@tauri-apps/api/core";
import type { ImportCandidate, KeyboardSnapshot, RuntimeEvent } from "./domain";
import { fakeSnapshot } from "./fixtures";

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
