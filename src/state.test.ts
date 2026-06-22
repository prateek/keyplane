import { describe, expect, it } from "vitest";
import { fakeSnapshot, navLayerEvent } from "./fixtures";
import { applyRuntimeEvent } from "./state";

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
});
