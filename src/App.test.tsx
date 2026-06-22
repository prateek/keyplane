import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App, { FADE_VISIBILITY_INACTIVITY_MS, ImportReview, OverlaySurface } from "./App";
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
    standard.unmount();

    renderOverlay("rich");
    expect(screen.getByText("nav-hint")).toBeInTheDocument();
    expect(screen.getByText("hold-layer")).toBeInTheDocument();
  });

  it("exposes active Profile EDN save and load actions", async () => {
    render(<App />);

    expect(await screen.findByRole("button", { name: /save edn/i })).toBeInTheDocument();
    expect(screen.getByText(/load edn/i)).toBeInTheDocument();
    expect(screen.getByText(/style json/i)).toBeInTheDocument();
    expect(screen.getByText(/overkeys json/i)).toBeInTheDocument();
    expect(screen.getByText(/zmk keymap/i)).toBeInTheDocument();
  });

  it("exposes KeyPeek Live VID and PID connection controls", async () => {
    render(<App />);

    expect(await screen.findByLabelText("KeyPeek VID")).toBeInTheDocument();
    expect(screen.getByLabelText("KeyPeek PID")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /connect/i })).toBeInTheDocument();
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

    expect(screen.getAllByText("Overlay Window").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText(/render-overlay-window/)).toBeInTheDocument();
    expect(screen.getAllByText("KeyPeek Live").length).toBeGreaterThanOrEqual(2);
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
        variant_id: "vial-preview",
        density: "standard" as const,
      },
      overlay_window: fakeSnapshot.overlay_window,
      source_precedence: fakeSnapshot.source_precedence,
      user_overrides: [],
      source_provenance: fakeSnapshot.source_provenance.slice(0, 2),
    };
    const candidate: ImportCandidate = {
      id: "candidate-small-preview",
      source: previewProfile.sources[0],
      best_effort_preview: true,
      preview_profile: previewProfile,
      conflicts: [],
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
      />,
    );

    expect(screen.getByText("Profile Diff")).toBeInTheDocument();
    expect(screen.getByText("Physical Keys")).toBeInTheDocument();
    expect(screen.getByText("12 -> 1")).toBeInTheDocument();
    expect(screen.getByText("Visual Style")).toBeInTheDocument();
    expect(screen.getByText("keyplane-default -> vial-preview")).toBeInTheDocument();
    expect(screen.getByText("Fallback Layout")).toBeInTheDocument();
    expect(screen.getByText("no -> yes")).toBeInTheDocument();
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
            ],
          },
        }
      : key,
  );
  return snapshot;
}
