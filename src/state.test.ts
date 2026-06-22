import { describe, expect, it } from "vitest";
import { fakeSnapshot, navLayerEvent } from "./fixtures";
import { applyRuntimeEvent, promoteSourceCandidate } from "./state";

describe("frontend runtime state", () => {
  it("recomputes inherited Effective Actions after a layer-stack Runtime Event", () => {
    const next = applyRuntimeEvent(fakeSnapshot, navLayerEvent);
    const q = next.effective_keys.find((key) => key.key_id === "k-q");

    expect(q?.semantic.label).toBe("Q");
    expect(q?.source_layer_id).toBe("layer-0");
    expect(q?.inherited).toBe(true);
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
});
