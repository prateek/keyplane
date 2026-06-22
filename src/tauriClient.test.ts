import { describe, expect, it } from "vitest";
import type { ImportCandidate, Profile } from "./domain";
import { fakeSnapshot, navLayerEvent } from "./fixtures";
import {
  applyRuntimeEventSnapshot,
  commitImportCandidate,
  discoverKeyPeekDevices,
  loadLaunchAtLogin,
  refreshHostPermissionHealth,
  registerSentinelKeyShortcuts,
  requestHostInputPermissions,
  setLaunchAtLogin,
  setOverlayVisibilityPolicy,
  setOverlayVisible,
  setVisualStyleDensity,
  startOverlayDrag,
  startOverlayResize,
  startKanataTcpBackend,
  stopKanataTcpBackend,
  unregisterSentinelKeyShortcuts,
} from "./tauriClient";

describe("Tauri client fallbacks", () => {
  it("commits an Import Candidate into a Keyboard Snapshot when Tauri is unavailable", async () => {
    const previewProfile: Profile = {
      schema_version: 1,
      id: "profile-imported-preview",
      keyboard_id: "keyboard-imported-preview",
      name: "Imported Preview",
      sources: fakeSnapshot.sources,
      physical_layout: fakeSnapshot.physical_layout,
      keymap: fakeSnapshot.keymap,
      runtime_backends: fakeSnapshot.backends,
      sentinel_keys: fakeSnapshot.sentinel_keys,
      visual_style: fakeSnapshot.visual_style,
      overlay_window: fakeSnapshot.overlay_window,
      source_precedence: fakeSnapshot.source_precedence,
      user_overrides: [],
      source_provenance: fakeSnapshot.physical_layout.keys.map((key) => key.provenance),
    };
    const candidate: ImportCandidate = {
      id: "candidate-imported-preview",
      source: {
        id: "vial-file-import",
        name: "Vial File Import",
        kind: "vial-file-import",
        authority: "best-effort-preview",
      },
      best_effort_preview: true,
      preview_profile: previewProfile,
      conflicts: [],
      summary: {
        imported_keys: previewProfile.physical_layout.keys.length,
        imported_layers: previewProfile.keymap.layers.length,
        preserved_sections: [],
      },
    };

    const snapshot = await commitImportCandidate(candidate);

    expect(snapshot.profile_id).toBe("profile-imported-preview");
    expect(snapshot.keyboard_id).toBe("keyboard-imported-preview");
    expect(snapshot.profile_name).toBe("Imported Preview");
    expect(snapshot.runtime_state.layer_stack[0].layer_id).toBe("layer-0");
    expect(snapshot.effective_keys.length).toBe(previewProfile.physical_layout.keys.length);
    expect(snapshot.sources).toEqual(fakeSnapshot.sources);
    expect(snapshot.source_provenance).toEqual(previewProfile.source_provenance);
  });

  it("falls back to local Runtime Event application when Tauri is unavailable", async () => {
    const snapshot = await applyRuntimeEventSnapshot(fakeSnapshot, navLayerEvent);
    const effectiveKey = snapshot.effective_keys.find((key) => key.key_id === "k-q");

    expect(snapshot.runtime_state.layer_stack[0].layer_id).toBe("layer-1");
    expect(effectiveKey?.source_layer_id).toBe("layer-0");
    expect(effectiveKey?.inherited).toBe(true);
  });

  it("marks preview-only committed profiles as medium-confidence fallback state", async () => {
    const previewProfile: Profile = {
      schema_version: 1,
      id: "profile-preview-only",
      keyboard_id: "keyboard-preview-only",
      name: "Preview Only",
      sources: fakeSnapshot.sources,
      physical_layout: fakeSnapshot.physical_layout,
      keymap: fakeSnapshot.keymap,
      runtime_backends: [
        {
          ...fakeSnapshot.backends[0],
          capabilities: ["preview-only"],
          health: {
            backend_id: "preview-only",
            state: "stale",
            message: "Imported profile only",
          },
        },
      ],
      sentinel_keys: fakeSnapshot.sentinel_keys,
      visual_style: fakeSnapshot.visual_style,
      overlay_window: fakeSnapshot.overlay_window,
      source_precedence: fakeSnapshot.source_precedence,
      user_overrides: [],
      source_provenance: fakeSnapshot.physical_layout.keys.map((key) => key.provenance),
    };

    const snapshot = await commitImportCandidate({
      id: "candidate-preview-only",
      source: {
        id: "preview-only",
        name: "Preview Only",
        kind: "vial-file-import",
        authority: "best-effort-preview",
      },
      best_effort_preview: true,
      preview_profile: previewProfile,
      conflicts: [],
      summary: {
        imported_keys: previewProfile.physical_layout.keys.length,
        imported_layers: previewProfile.keymap.layers.length,
        preserved_sections: [],
      },
    });

    expect(snapshot.runtime_state.layer_stack[0].confidence.level).toBe("medium");
    expect(snapshot.runtime_state.layer_stack[0].confidence.reason).toBe(
      "Best-Effort Preview default layer",
    );
  });

  it("reports launch-at-login as unavailable when the Tauri plugin is absent", async () => {
    await expect(loadLaunchAtLogin()).resolves.toBeNull();
    await expect(setLaunchAtLogin(true)).resolves.toBeNull();
  });

  it("reports Sentinel Key shortcut registration as unavailable when Tauri is absent", async () => {
    await expect(registerSentinelKeyShortcuts()).resolves.toBeNull();
    await expect(unregisterSentinelKeyShortcuts()).resolves.toBeNull();
  });

  it("reports Host Input permission health as unavailable when Tauri is absent", async () => {
    await expect(refreshHostPermissionHealth()).resolves.toBeNull();
    await expect(requestHostInputPermissions()).resolves.toBeNull();
  });

  it("reports KeyPeek device discovery as unavailable when Tauri is absent", async () => {
    const discovery = await discoverKeyPeekDevices();

    expect(discovery?.devices).toEqual([]);
    const health = discovery?.snapshot.runtime_state.backend_health.find(
      (candidate) => candidate.backend_id === "keypeek-live",
    );
    expect(health).toEqual({
      backend_id: "keypeek-live",
      state: "disconnected",
      message: "KeyPeek device discovery unavailable",
    });
  });

  it("reports Kanata TCP runtime control as unavailable when Tauri is absent", async () => {
    await expect(startKanataTcpBackend({ host: "127.0.0.1", port: 7070 })).resolves.toBeNull();
    await expect(stopKanataTcpBackend()).resolves.toBeNull();
  });

  it("reports Visual Style density updates as unavailable when Tauri is absent", async () => {
    await expect(setVisualStyleDensity("compact")).resolves.toBeNull();
  });

  it("reports Overlay Visibility controls as unavailable when Tauri is absent", async () => {
    await expect(setOverlayVisibilityPolicy("manual-toggle")).resolves.toBeNull();
    await expect(setOverlayVisible(false)).resolves.toBeNull();
  });

  it("reports Overlay Window placement controls as unavailable when Tauri is absent", async () => {
    await expect(startOverlayDrag()).resolves.toBeNull();
    await expect(startOverlayResize("south-east")).resolves.toBeNull();
  });
});
