// @vitest-environment jsdom
//
// Overlay rendering tests (PRD "Overlay behavior tests"): component tests cover
// what visual inspection would, against the real serialized snapshot fixture —
// every Physical Key renders, effective legends appear, the active layer is
// highlighted, transparent keys show the inherited indicator, and the minimal
// Style Variant collapses legends.

import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import dtos from "../__fixtures__/dtos.json";
import type { KeyboardSnapshot, ResolvedKey, RuntimeEvent } from "../types";
import { topLayer } from "../overlayState";
import { Keyboard } from "./Keyboard";

const snapshot = dtos.snapshot as unknown as KeyboardSnapshot;
const navEvent = dtos.navEvent as unknown as RuntimeEvent & { keys: ResolvedKey[] };

function renderKeyboard(keys: ResolvedKey[], extras?: Partial<Parameters<typeof Keyboard>[0]>) {
  return render(
    <Keyboard
      keys={keys}
      extent={snapshot.extent}
      style={snapshot.style}
      topLayer={topLayer(snapshot.layer_stack)}
      {...extras}
    />,
  );
}

describe("Keyboard rendering", () => {
  it("renders one keycap per Physical Key with its legend", () => {
    const { container } = renderKeyboard(snapshot.keys);
    expect(container.querySelectorAll("[data-key]")).toHaveLength(snapshot.keys.length);
    // The base-layer 'A' key shows its label.
    expect(container.textContent).toContain("A");
  });

  it("highlights the top active layer's keys", () => {
    const { container } = renderKeyboard(snapshot.keys);
    // The base demo: top layer is layer-0, so its keys carry the on-top class.
    expect(container.querySelectorAll(".keycap.on-top").length).toBeGreaterThan(0);
  });

  it("draws the inherited indicator on transparent keys under a momentary layer", () => {
    const { container } = render(
      <Keyboard
        keys={navEvent.keys}
        extent={snapshot.extent}
        style={snapshot.style}
        topLayer="layer-1"
      />,
    );
    // k8 is transparent on layer-1 and inherits from the base layer.
    expect(container.querySelectorAll(".inherited-dot").length).toBeGreaterThan(0);
  });

  it("collapses legends to a single label in the minimal Style Variant", () => {
    const { container } = renderKeyboard(snapshot.keys, {
      style: { ...snapshot.style, variant: "minimal" },
    });
    // Minimal renders one primary legend span per key (no multi-slot legends).
    expect(container.querySelectorAll(".legend-hold").length).toBe(0);
    expect(container.querySelectorAll("[data-key]").length).toBe(snapshot.keys.length);
  });
});
