import { describe, expect, it } from "vitest";
import type { ImportCandidate } from "./domain";
import { fakeSnapshot, navLayerEvent } from "./fixtures";
import { applyRuntimeEvent, promoteImportCandidateSource, promoteSourceCandidate } from "./state";

describe("frontend runtime state", () => {
  it("recomputes inherited Effective Actions after a layer-stack Runtime Event", () => {
    const next = applyRuntimeEvent(fakeSnapshot, navLayerEvent);
    const q = next.effective_keys.find((key) => key.key_id === "k-q");

    expect(q?.semantic.label).toBe("Q");
    expect(q?.source_layer_id).toBe("layer-0");
    expect(q?.inherited).toBe(true);
  });

  it("ignores lower-priority layer-stack Runtime Events", () => {
    const next = applyRuntimeEvent(fakeSnapshot, {
      type: "layer-stack-changed",
      source_id: "sentinel-keys",
      layer_stack: [
        {
          layer_id: "layer-1",
          kind: "momentary",
          confidence: {
            level: "low",
            reason: "Sentinel Key inferred from Host Input Event",
          },
        },
        {
          layer_id: "layer-0",
          kind: "default",
          confidence: {
            level: "low",
            reason: "Sentinel Key inferred from Host Input Event",
          },
        },
      ],
    });

    expect(next.runtime_state.layer_stack[0].layer_id).toBe("layer-0");
    expect(next.runtime_state.layer_stack_source_id).toBe("fake-backend");
  });

  it("accepts higher-priority layer-stack Runtime Events", () => {
    const snapshot = structuredClone(fakeSnapshot);
    snapshot.runtime_state.layer_stack_source_id = "sentinel-keys";

    const next = applyRuntimeEvent(snapshot, {
      type: "layer-stack-changed",
      source_id: "keypeek-live",
      layer_stack: [
        {
          layer_id: "layer-1",
          kind: "momentary",
          confidence: {
            level: "high",
            reason: "KeyPeek firmware-module layer packet",
          },
        },
        {
          layer_id: "layer-0",
          kind: "default",
          confidence: {
            level: "high",
            reason: "Base layer retained below KeyPeek active layers",
          },
        },
      ],
    });

    expect(next.runtime_state.layer_stack[0].layer_id).toBe("layer-1");
    expect(next.runtime_state.layer_stack_source_id).toBe("keypeek-live");
  });

  it("keeps Backend Health as observable runtime state", () => {
    const next = applyRuntimeEvent(fakeSnapshot, {
      type: "backend-health-changed",
      health: {
        backend_id: "fake-backend",
        state: "disconnected",
        message: "Fake backend disconnected",
      },
    });

    expect(next.runtime_state.backend_health[0].state).toBe("disconnected");
    expect(next.backends[0].health.message).toBe("Fake backend disconnected");
  });

  it("adds permission health for newly discovered backends", () => {
    const next = applyRuntimeEvent(fakeSnapshot, {
      type: "backend-health-changed",
      health: {
        backend_id: "external-host-input",
        state: "permission-missing",
        message: "External host input permission is missing",
      },
    });

    expect(next.runtime_state.backend_health).toContainEqual({
      backend_id: "external-host-input",
      state: "permission-missing",
      message: "External host input permission is missing",
    });
    expect(next.backends.find((backend) => backend.id === "external-host-input")).toBeUndefined();
  });

  it("keeps Sentinel Keys visible as a lower-confidence backend", () => {
    const sentinel = fakeSnapshot.backends.find((backend) => backend.id === "sentinel-keys");

    expect(sentinel?.health.state).toBe("permission-missing");
    expect(fakeSnapshot.sentinel_keys).toContainEqual({
      host_input_code: "F24",
      layer_id: "layer-1",
      activation: "momentary",
    });
  });

  it("promotes a source conflict candidate to a User Override", () => {
    const conflict = fakeSnapshot.source_conflicts[0];
    const next = promoteSourceCandidate(fakeSnapshot, conflict, "keyviz-import");

    expect(next.user_overrides).toEqual([
      {
        field_path: ":visual/style :style/variant-id",
        value: "keyviz-minimal",
        reason: "Promoted from keyviz-import",
      },
    ]);
    expect(next.source_conflicts[0].selected_source_id).toBe("user-overrides");
    expect(next.source_conflicts[0].candidates).toContainEqual({
      source_id: "fake-backend",
      value: "keyplane-default",
      selected: false,
    });
    expect(next.source_conflicts[0].candidates).toContainEqual({
      source_id: "user-overrides",
      value: "keyviz-minimal",
      selected: true,
    });
    expect(next.visual_style.variant_id).toBe("keyviz-minimal");
  });

  it("applies promoted Visual Style color token conflicts", () => {
    const conflict = {
      field_path: ":visual/style :style/colors :color/keycap-background",
      selected_source_id: "fake-backend",
      candidates: [
        { source_id: "fake-backend", value: "nil", selected: true },
        { source_id: "keyviz-import", value: "#ffffff", selected: false },
      ],
    };
    const snapshot = structuredClone(fakeSnapshot);
    snapshot.source_conflicts = [conflict];

    const next = promoteSourceCandidate(snapshot, conflict, "keyviz-import");

    expect(next.user_overrides).toContainEqual({
      field_path: ":visual/style :style/colors :color/keycap-background",
      value: "#ffffff",
      reason: "Promoted from keyviz-import",
    });
    expect(next.visual_style.colors.keycap_background).toBe("#ffffff");
  });

  it("applies promoted Logical Keymap conflicts to rendered Effective Actions", () => {
    const conflict = {
      field_path: ":keyboard/keymap :layer-0 k-q :action/raw",
      selected_source_id: "fake-backend",
      candidates: [
        { source_id: "fake-backend", value: "KC_Q", selected: true },
        { source_id: "vial-import", value: "KC_TAB", selected: false },
      ],
    };
    const snapshot = structuredClone(fakeSnapshot);
    snapshot.source_conflicts = [conflict];

    const next = promoteSourceCandidate(snapshot, conflict, "vial-import");
    const keyAction = next.keymap.layers[0].actions.find((action) => action.key_id === "k-q");
    const effectiveKey = next.effective_keys.find((key) => key.key_id === "k-q");

    expect(next.user_overrides).toContainEqual({
      field_path: ":keyboard/keymap :layer-0 k-q :action/raw",
      value: "KC_TAB",
      reason: "Promoted from vial-import",
    });
    expect(next.source_conflicts[0].selected_source_id).toBe("user-overrides");
    expect(keyAction?.raw.value).toBe("KC_TAB");
    expect(keyAction?.semantic.label).toBe("Tab");
    expect(keyAction?.provenance.source_id).toBe("user-overrides");
    expect(effectiveKey?.raw.value).toBe("KC_TAB");
    expect(effectiveKey?.semantic.label).toBe("Tab");
  });

  it("applies promoted Physical Layout conflicts to rendered key geometry", () => {
    const conflict = {
      field_path: ":keyboard/physical-layout k-q :geometry/x",
      selected_source_id: "fake-backend",
      candidates: [
        { source_id: "fake-backend", value: "1.08", selected: true },
        { source_id: "vial-import", value: "2.5", selected: false },
      ],
    };
    const snapshot = structuredClone(fakeSnapshot);
    snapshot.source_conflicts = [conflict];

    const next = promoteSourceCandidate(snapshot, conflict, "vial-import");
    const physicalKey = next.physical_layout.keys.find((key) => key.id === "k-q");

    expect(next.user_overrides).toContainEqual({
      field_path: ":keyboard/physical-layout k-q :geometry/x",
      value: "2.5",
      reason: "Promoted from vial-import",
    });
    expect(next.source_conflicts[0].selected_source_id).toBe("user-overrides");
    expect(physicalKey?.geometry.x).toBe(2.5);
    expect(physicalKey?.provenance.source_id).toBe("user-overrides");
    expect(physicalKey?.provenance.field_path).toBe(
      ":keyboard/physical-layout k-q :geometry/x",
    );
  });

  it("applies promoted Physical Layout matrix conflicts to rendered key matrix positions", () => {
    const conflict = {
      field_path: ":keyboard/physical-layout k-q :matrix/row",
      selected_source_id: "fake-backend",
      candidates: [
        { source_id: "fake-backend", value: "0", selected: true },
        { source_id: "vial-import", value: "9", selected: false },
      ],
    };
    const snapshot = structuredClone(fakeSnapshot);
    snapshot.source_conflicts = [conflict];

    const next = promoteSourceCandidate(snapshot, conflict, "vial-import");
    const physicalKey = next.physical_layout.keys.find((key) => key.id === "k-q");

    expect(next.user_overrides).toContainEqual({
      field_path: ":keyboard/physical-layout k-q :matrix/row",
      value: "9",
      reason: "Promoted from vial-import",
    });
    expect(next.source_conflicts[0].selected_source_id).toBe("user-overrides");
    expect(physicalKey?.matrix?.row).toBe(9);
    expect(physicalKey?.provenance.source_id).toBe("user-overrides");
    expect(physicalKey?.provenance.field_path).toBe(
      ":keyboard/physical-layout k-q :matrix/row",
    );
  });

  it("promotes Import Review candidates to pending Profile User Overrides", () => {
    const conflict = {
      field_path: ":visual/style :style/colors :color/keycap-background",
      selected_source_id: "keyviz-import",
      candidates: [
        { source_id: "fake-backend", value: "nil", selected: false },
        { source_id: "keyviz-import", value: "#ffffff", selected: true },
      ],
    };
    const candidate: ImportCandidate = {
      id: "candidate-keyviz",
      source: {
        id: "keyviz-import",
        name: "keyviz import",
        kind: "keyviz-style-import",
        authority: "best-effort-preview",
      },
      best_effort_preview: true,
      preview_profile: {
        schema_version: 1,
        id: "profile-keyviz",
        keyboard_id: "keyboard-keyviz",
        name: "keyviz profile",
        sources: fakeSnapshot.sources,
        physical_layout: fakeSnapshot.physical_layout,
        keymap: fakeSnapshot.keymap,
        runtime_backends: fakeSnapshot.backends,
        sentinel_keys: fakeSnapshot.sentinel_keys,
        visual_style: {
          ...fakeSnapshot.visual_style,
          colors: {
            ...fakeSnapshot.visual_style.colors,
            keycap_background: "#ffffff",
          },
        },
        overlay_window: fakeSnapshot.overlay_window,
        source_precedence: fakeSnapshot.source_precedence,
        user_overrides: [],
        source_provenance: fakeSnapshot.source_provenance,
      },
      conflicts: [conflict],
      summary: {
        imported_keys: 0,
        imported_layers: 0,
        preserved_sections: [],
      },
    };

    const next = promoteImportCandidateSource(candidate, conflict, "fake-backend");

    expect(next.preview_profile.user_overrides).toEqual([
      {
        field_path: ":visual/style :style/colors :color/keycap-background",
        value: "nil",
        reason: "Promoted from fake-backend",
      },
    ]);
    expect(next.conflicts[0].selected_source_id).toBe("user-overrides");
    expect(next.conflicts[0].candidates).toContainEqual({
      source_id: "user-overrides",
      value: "nil",
      selected: true,
    });
    expect(next.preview_profile.visual_style.colors.keycap_background).toBeNull();
    expect(candidate.preview_profile.user_overrides).toEqual([]);
  });

  it("promotes Import Review keymap candidates to pending Profile User Overrides", () => {
    const conflict = {
      field_path: ":keyboard/keymap :layer-0 k-q",
      selected_source_id: "fake-backend",
      candidates: [
        { source_id: "fake-backend", value: "KC_Q", selected: true },
        { source_id: "vial-import", value: "KC_TAB", selected: false },
      ],
    };
    const candidate: ImportCandidate = {
      id: "candidate-vial-keymap",
      source: {
        id: "vial-import",
        name: "Vial import",
        kind: "vial-file-import",
        authority: "best-effort-preview",
      },
      best_effort_preview: true,
      preview_profile: {
        schema_version: 1,
        id: "profile-vial",
        keyboard_id: "keyboard-vial",
        name: "Vial profile",
        sources: fakeSnapshot.sources,
        physical_layout: fakeSnapshot.physical_layout,
        keymap: fakeSnapshot.keymap,
        runtime_backends: fakeSnapshot.backends,
        sentinel_keys: fakeSnapshot.sentinel_keys,
        visual_style: fakeSnapshot.visual_style,
        overlay_window: fakeSnapshot.overlay_window,
        source_precedence: fakeSnapshot.source_precedence,
        user_overrides: [],
        source_provenance: fakeSnapshot.source_provenance,
      },
      conflicts: [conflict],
      summary: {
        imported_keys: 1,
        imported_layers: 1,
        preserved_sections: [],
      },
    };

    const next = promoteImportCandidateSource(candidate, conflict, "vial-import");
    const keyAction = next.preview_profile.keymap.layers[0].actions.find(
      (action) => action.key_id === "k-q",
    );

    expect(next.preview_profile.user_overrides).toContainEqual({
      field_path: ":keyboard/keymap :layer-0 k-q",
      value: "KC_TAB",
      reason: "Promoted from vial-import",
    });
    expect(next.conflicts[0].selected_source_id).toBe("user-overrides");
    expect(keyAction?.raw.value).toBe("KC_TAB");
    expect(keyAction?.semantic.label).toBe("Tab");
    expect(keyAction?.provenance.source_id).toBe("user-overrides");
    expect(candidate.preview_profile.user_overrides).toEqual([]);
  });

  it("promotes Import Review Physical Layout candidates to pending Profile User Overrides", () => {
    const conflict = {
      field_path: ":keyboard/physical-layout k-q :geometry/x",
      selected_source_id: "fake-backend",
      candidates: [
        { source_id: "fake-backend", value: "1.08", selected: true },
        { source_id: "vial-import", value: "2.5", selected: false },
      ],
    };
    const candidate: ImportCandidate = {
      id: "candidate-vial-layout",
      source: {
        id: "vial-import",
        name: "Vial import",
        kind: "vial-file-import",
        authority: "best-effort-preview",
      },
      best_effort_preview: true,
      preview_profile: {
        schema_version: 1,
        id: "profile-vial",
        keyboard_id: "keyboard-vial",
        name: "Vial profile",
        sources: fakeSnapshot.sources,
        physical_layout: fakeSnapshot.physical_layout,
        keymap: fakeSnapshot.keymap,
        runtime_backends: fakeSnapshot.backends,
        sentinel_keys: fakeSnapshot.sentinel_keys,
        visual_style: fakeSnapshot.visual_style,
        overlay_window: fakeSnapshot.overlay_window,
        source_precedence: fakeSnapshot.source_precedence,
        user_overrides: [],
        source_provenance: fakeSnapshot.source_provenance,
      },
      conflicts: [conflict],
      summary: {
        imported_keys: 1,
        imported_layers: 0,
        preserved_sections: [],
      },
    };

    const next = promoteImportCandidateSource(candidate, conflict, "vial-import");
    const physicalKey = next.preview_profile.physical_layout.keys.find((key) => key.id === "k-q");

    expect(next.preview_profile.user_overrides).toContainEqual({
      field_path: ":keyboard/physical-layout k-q :geometry/x",
      value: "2.5",
      reason: "Promoted from vial-import",
    });
    expect(next.conflicts[0].selected_source_id).toBe("user-overrides");
    expect(physicalKey?.geometry.x).toBe(2.5);
    expect(physicalKey?.provenance.source_id).toBe("user-overrides");
    expect(candidate.preview_profile.user_overrides).toEqual([]);
  });
});
