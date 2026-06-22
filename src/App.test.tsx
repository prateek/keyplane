import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App, {
  FADE_VISIBILITY_INACTIVITY_MS,
  ImportReview,
  OverlaySurface,
  kanataTcpSettingsFromSnapshot,
} from "./App";
import type { ImportCandidate } from "./domain";
import { fakeSnapshot } from "./fixtures";

describe("Keyplane app", () => {
  it("renders the full-keyboard overlay as the first surface", async () => {
    render(<App />);

    const overlay = await screen.findByRole("region", { name: "Keyboard overlay" });
    expect(overlay).toBeInTheDocument();
    expect(overlay).toHaveStyle({ opacity: "0.92" });
    expect(screen.getByRole("button", { name: /k-q q/i })).toBeInTheDocument();
    expect(screen.getAllByText("ok").length).toBeGreaterThan(0);
    expect(screen.getByText("click-through")).toBeInTheDocument();
    expect(within(overlay).getByText("Default layer from fake backend")).toBeInTheDocument();
    expect(within(overlay).getByText("Streaming deterministic layer stack events")).toBeInTheDocument();
  });

  it("renders inherited legends after a fake layer event", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /fake event/i }));

    expect(screen.getAllByText("inherited").length).toBeGreaterThan(0);
    expect(screen.getByText("Momentary layer from fake backend Runtime Event")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /k-w up/i })).toHaveClass("top-active-layer");
    expect(screen.getByRole("button", { name: /k-q q/i })).toHaveClass("inherited");
    expect(screen.getByRole("button", { name: /k-q q/i })).not.toHaveClass("top-active-layer");
  });

  it("shows transparent Raw Actions and inherited sources in Source Inspector", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /fake event/i }));
    await user.click(screen.getByRole("button", { name: /source inspector/i }));

    expect(screen.getByText("Transparent Entries")).toBeInTheDocument();
    expect(screen.getAllByText("KC_TRNS").length).toBeGreaterThan(0);
    expect(screen.getByText("inherits Q from layer-0")).toBeInTheDocument();
    expect(screen.getByText(":keyboard/keymap k-q")).toBeInTheDocument();
  });

  it("shows Layer Stack precedence and confidence in Source Inspector", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /fake event/i }));
    await user.click(screen.getByRole("button", { name: /source inspector/i }));

    expect(screen.getByText("Layer Stack")).toBeInTheDocument();
    expect(screen.getByText(/1\.\s*Nav/)).toBeInTheDocument();
    expect(screen.getByText("top precedence")).toBeInTheDocument();
    expect(screen.getByText("layer-1 - momentary")).toBeInTheDocument();
    expect(screen.getByText("Momentary layer from fake backend Runtime Event")).toBeInTheDocument();
    expect(screen.getByText(/2\.\s*Base/)).toBeInTheDocument();
    expect(screen.getByText("lower precedence")).toBeInTheDocument();
    expect(screen.getByText("layer-0 - default")).toBeInTheDocument();
  });

  it("uses Visual Style density to collapse or preserve structured Legend Slots", () => {
    const renderOverlay = (density: "compact" | "standard" | "rich") =>
      render(
        <OverlaySurface
          snapshot={snapshotWithDensity(density)}
          onAdvance={() => undefined}
          onDragOverlay={() => undefined}
          onResizeOverlay={() => undefined}
          onTogglePositioningMode={() => undefined}
        />,
      );

    const compact = renderOverlay("compact");
    expect(screen.getByRole("button", { name: /k-space space/i })).toBeInTheDocument();
    expect(screen.queryByText("nav-hint")).not.toBeInTheDocument();
    expect(screen.queryByText("hold-layer")).not.toBeInTheDocument();
    compact.unmount();

    const standard = renderOverlay("standard");
    expect(screen.getByText("nav-hint")).toBeInTheDocument();
    expect(screen.queryByText("hold-layer")).not.toBeInTheDocument();
    expect(screen.queryByText("layer-icon")).not.toBeInTheDocument();
    standard.unmount();

    renderOverlay("rich");
    expect(screen.getByText("nav-hint")).toBeInTheDocument();
    expect(screen.getByText("hold-layer")).toBeInTheDocument();
    expect(screen.getByText("layer-icon")).toHaveClass("slot-icon");
  });

  it("applies Visual Style color tokens to the overlay surface", () => {
    const snapshot = structuredClone(fakeSnapshot);
    snapshot.visual_style = {
      ...snapshot.visual_style,
      colors: {
        keycap_background: "#ffffff",
        keycap_text: "#111111",
        keycap_border: "#222222",
        modifier_accent: "#3a86ff",
        overlay_background: "#ffffff99",
      },
    };

    render(
      <OverlaySurface
        snapshot={snapshot}
        onAdvance={() => undefined}
        onDragOverlay={() => undefined}
        onResizeOverlay={() => undefined}
        onTogglePositioningMode={() => undefined}
      />,
    );

    const overlay = screen.getByRole("region", { name: "Keyboard overlay" });
    expect(overlay.getAttribute("style")).toContain("--keyplane-keycap-background: #ffffff");
    expect(overlay.getAttribute("style")).toContain("--keyplane-keycap-text: #111111");
    expect(overlay.getAttribute("style")).toContain("--keyplane-keycap-border: #222222");
    expect(overlay.getAttribute("style")).toContain("--keyplane-modifier-accent: #3a86ff");
    expect(overlay.getAttribute("style")).toContain("--keyplane-overlay-background: #ffffff99");
  });

  it("exposes active Profile EDN save and load actions", async () => {
    render(<App />);

    expect(await screen.findByRole("button", { name: /save edn/i })).toBeInTheDocument();
    expect(screen.getByText(/load edn/i)).toBeInTheDocument();
    expect(screen.getByText(/style json/i)).toBeInTheDocument();
    expect(screen.getByText(/keypeek qmk info/i)).toBeInTheDocument();
    expect(screen.getByText(/overkeys json/i)).toBeInTheDocument();
    expect(screen.getByText(/zmk keymap/i)).toBeInTheDocument();
  });

  it("exposes KeyPeek Live VID and PID connection controls", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByLabelText("KeyPeek VID")).toBeInTheDocument();
    expect(screen.getByLabelText("KeyPeek PID")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /connect/i })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /scan keypeek devices/i }));
    expect(await screen.findByText("KeyPeek device discovery unavailable")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /import vial device/i })).toBeInTheDocument();
  });

  it("exposes startup and Sentinel Key backend settings", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /settings/i }));

    expect(screen.getByRole("region", { name: /app settings/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/launch at login/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/kanata host/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/kanata port/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /connect kanata/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /stop kanata/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /compact/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /standard/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /rich/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /pinned/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /manual-toggle/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /fade/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/overlay visible/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/sentinel keys/i)).toBeInTheDocument();
    expect(screen.getByText(/host input permissions/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /check/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /request/i })).toBeInTheDocument();
    expect(screen.getAllByText(/unavailable|enabled|disabled/i).length).toBeGreaterThanOrEqual(2);
  });

  it("reads Kanata TCP settings from the active Keyboard Snapshot", () => {
    const snapshot = structuredClone(fakeSnapshot);
    const kanataBackend = snapshot.backends.find((backend) => backend.id === "kanata-tcp");
    if (!kanataBackend) throw new Error("fixture should include Kanata TCP");
    kanataBackend.config = { kind: "kanata-tcp", host: "10.0.0.20", port: 4039 };

    expect(kanataTcpSettingsFromSnapshot(snapshot)).toEqual({
      host: "10.0.0.20",
      port: "4039",
    });
  });

  it("updates Visual Style density from Settings", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect((await screen.findAllByText("layer-1")).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: /settings/i }));
    await user.click(screen.getByRole("button", { name: /compact/i }));
    await screen.findByText("Visual style density set to compact");
    await user.click(screen.getByRole("button", { name: /overlay/i }));

    await waitFor(() => {
      expect(screen.queryByText("layer-1")).not.toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: /settings/i }));
    await user.click(screen.getByRole("button", { name: /rich/i }));
    await screen.findByText("Visual style density set to rich");
    await user.click(screen.getByRole("button", { name: /overlay/i }));

    expect((await screen.findAllByText("layer-1")).length).toBeGreaterThan(0);
  });

  it("updates Overlay Visibility Policy and visible state from Settings", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /settings/i }));
    await user.click(screen.getByRole("button", { name: /manual-toggle/i }));
    await screen.findByText("Overlay Visibility Policy set to manual-toggle");
    await user.click(screen.getByLabelText(/overlay visible/i));
    await screen.findByText("Overlay Window hidden");

    expect(screen.getByText("hidden")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /overlay/i }));

    expect(screen.getByText("manual-toggle")).toBeInTheDocument();
    expect(screen.getByText("hidden")).toBeInTheDocument();
  });

  it("fades the Overlay Window after inactivity and shows it again on Runtime Events", async () => {
    render(<App />);
    await screen.findByRole("region", { name: "Keyboard overlay" });

    fireEvent.click(screen.getByRole("button", { name: /settings/i }));
    vi.useFakeTimers();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /fade/i }));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(screen.getByText("Overlay Visibility Policy set to fade")).toBeInTheDocument();
    expect(screen.getByText("visible")).toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(FADE_VISIBILITY_INACTIVITY_MS);
    });

    expect(screen.getByText("hidden")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /overlay/i }));
    fireEvent.click(screen.getByRole("button", { name: /fake event/i }));
    expect(screen.getByText("visible")).toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(FADE_VISIBILITY_INACTIVITY_MS - 1);
    });
    expect(screen.getByText("visible")).toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });

    expect(screen.getByText("hidden")).toBeInTheDocument();
  });

  it("shows overlay drag and resize affordances in Positioning Mode", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /position/i }));

    expect(screen.getByRole("button", { name: /lock/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /drag/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /resize/i })).toBeInTheDocument();
  });

  it("surfaces Source Conflicts outside the overlay", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /source inspector/i }));

    expect(screen.getByText("Active Profile")).toBeInTheDocument();
    expect(screen.getByText("Keyboard ID: keyboard-keyplane-demo")).toBeInTheDocument();
    expect(screen.getByText("Style ID: style-keyplane-default")).toBeInTheDocument();
    expect(screen.getAllByText("Overlay Window").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText(/render-overlay-window/)).toBeInTheDocument();
    expect(screen.getAllByText("KeyPeek Live").length).toBeGreaterThanOrEqual(2);
    expect(
      screen.getByText("discover-devices, stream-layer-stack, stream-pressed-keys"),
    ).toBeInTheDocument();
    expect(screen.getAllByText("Kanata TCP").length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByText("disconnected").length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("No KeyPeek-compatible device is connected")).toBeInTheDocument();
    expect(screen.getByText("Kanata TCP runtime is not connected")).toBeInTheDocument();
    expect(
      screen.getByText("Input monitoring permission is required before Sentinel Keys can infer layers"),
    ).toBeInTheDocument();
    expect(screen.getByText(":visual/style :style/variant-id")).toBeInTheDocument();
    expect(screen.getAllByText("keyviz-import").length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("keyviz-minimal")).toBeInTheDocument();
    expect(screen.getAllByText('{"style":"minimal"}').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Sources")).toBeInTheDocument();
    expect(
      screen.getAllByText(
        "user-overrides -> vial-import -> zmk-import -> overkeys-import -> fake-backend",
      ),
    ).toHaveLength(2);
    expect(screen.getByText("Source Provenance")).toBeInTheDocument();
    expect(screen.getByText(":keyboard/physical-layout k-q")).toBeInTheDocument();
    expect(screen.getByText("matrix:0,1")).toBeInTheDocument();
    expect(screen.getAllByText("variant=keyplane-default").length).toBeGreaterThanOrEqual(1);
  });

  it("promotes a Source Conflict candidate to a User Override", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /source inspector/i }));
    await user.click(screen.getByRole("button", { name: /promote/i }));

    expect(screen.getAllByText("user-overrides").length).toBeGreaterThan(0);
    expect(screen.getAllByText("keyviz-minimal").length).toBeGreaterThan(0);
    expect(screen.getByText("Promoted from keyviz-import")).toBeInTheDocument();
  });

  it("shows an Active Profile diff before committing an Import Candidate", () => {
    const previewProfile = {
      schema_version: 1,
      id: "profile-small-preview",
      keyboard_id: "keyboard-small-preview",
      name: "Small Preview",
      sources: fakeSnapshot.sources.slice(0, 1),
      physical_layout: {
        keys: fakeSnapshot.physical_layout.keys.slice(0, 1),
        fallback: true,
      },
      keymap: {
        layers: fakeSnapshot.keymap.layers.slice(0, 1),
      },
      runtime_backends: fakeSnapshot.backends.slice(0, 1),
      sentinel_keys: [],
      visual_style: {
        id: "style-vial-preview",
        variant_id: "vial-preview",
        density: "standard" as const,
        colors: fakeSnapshot.visual_style.colors,
      },
      overlay_window: fakeSnapshot.overlay_window,
      source_precedence: fakeSnapshot.source_precedence,
      user_overrides: [],
      source_provenance: fakeSnapshot.source_provenance,
    };
    const candidate: ImportCandidate = {
      id: "candidate-small-preview",
      source: previewProfile.sources[0],
      best_effort_preview: true,
      preview_profile: previewProfile,
      conflicts: fakeSnapshot.source_conflicts,
      summary: {
        imported_keys: 1,
        imported_layers: 1,
        preserved_sections: [":keyboard/physical-layout"],
      },
    };

    render(
      <ImportReview
        activeSnapshot={fakeSnapshot}
        candidate={candidate}
        error={null}
        onCommit={() => undefined}
        onPromote={() => undefined}
      />,
    );

    expect(screen.getByText("Profile Diff")).toBeInTheDocument();
    expect(screen.getByText("Physical Keys")).toBeInTheDocument();
    expect(screen.getByText("12 -> 1")).toBeInTheDocument();
    expect(screen.getByText("Visual Style")).toBeInTheDocument();
    expect(
      screen.getByText("style-keyplane-default (keyplane-default) -> style-vial-preview (vial-preview)"),
    ).toBeInTheDocument();
    expect(screen.getByText("Style ID")).toBeInTheDocument();
    expect(screen.getByText("style-keyplane-default -> style-vial-preview")).toBeInTheDocument();
    expect(screen.getByText("Keyboard ID")).toBeInTheDocument();
    expect(screen.getByText("keyboard-keyplane-demo -> keyboard-small-preview")).toBeInTheDocument();
    expect(screen.getByText("Fallback Layout")).toBeInTheDocument();
    expect(screen.getByText("no -> yes")).toBeInTheDocument();
    expect(screen.getByText(":visual/style :style/variant-id")).toBeInTheDocument();
    expect(screen.getByText("Selected")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /promote/i })).toBeInTheDocument();
    expect(screen.getByText("variant=keyplane-default")).toBeInTheDocument();
    expect(screen.getByText('{"style":"minimal"}')).toBeInTheDocument();
  });

  it("promotes an Import Review Source Conflict candidate before commit", async () => {
    const user = userEvent.setup();
    const onPromote = vi.fn();
    const candidate: ImportCandidate = {
      id: "candidate-style",
      source: fakeSnapshot.sources[0],
      best_effort_preview: true,
      preview_profile: {
        schema_version: 1,
        id: "profile-style",
        keyboard_id: "keyboard-style",
        name: "Style Preview",
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
      conflicts: fakeSnapshot.source_conflicts,
      summary: {
        imported_keys: 0,
        imported_layers: 0,
        preserved_sections: [],
      },
    };

    render(
      <ImportReview
        activeSnapshot={fakeSnapshot}
        candidate={candidate}
        error={null}
        onCommit={() => undefined}
        onPromote={onPromote}
      />,
    );

    await user.click(screen.getByRole("button", { name: /promote/i }));

    expect(onPromote).toHaveBeenCalledWith(fakeSnapshot.source_conflicts[0], "keyviz-import");
  });
});

function snapshotWithDensity(density: "compact" | "standard" | "rich") {
  const snapshot = structuredClone(fakeSnapshot);
  snapshot.visual_style = {
    ...snapshot.visual_style,
    density,
  };
  snapshot.effective_keys = snapshot.effective_keys.map((key) =>
    key.key_id === "k-space"
      ? {
          ...key,
          legend: {
            slots: [
              { slot: "primary", text: "Space" },
              { slot: "layer-hint", text: "nav-hint" },
              { slot: "hold-role", text: "hold-layer" },
              { slot: "icon", text: "layer-icon" },
            ],
          },
        }
      : key,
  );
  return snapshot;
}
