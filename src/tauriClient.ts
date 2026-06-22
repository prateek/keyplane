import { invoke } from "@tauri-apps/api/core";
import type { ImportCandidate, KeyboardSnapshot, RuntimeEvent } from "./domain";
import { fakeSnapshot } from "./fixtures";

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

export async function importVialFile(contents: string): Promise<ImportCandidate> {
  return invoke<ImportCandidate>("import_vial_file", { contents });
}
