import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import App, { ImportReview } from "./App";
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
  });

  it("renders inherited legends after a fake layer event", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /fake event/i }));

    expect(screen.getAllByText("inherited").length).toBeGreaterThan(0);
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
    expect(screen.getByLabelText(/sentinel keys/i)).toBeInTheDocument();
    expect(screen.getAllByText(/unavailable|enabled|disabled/i).length).toBeGreaterThanOrEqual(2);
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

    expect(screen.getAllByText("KeyPeek Live").length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByText("Kanata TCP").length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByText("disconnected").length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText(":visual/style :style/variant-id")).toBeInTheDocument();
    expect(screen.getByText("keyviz-import")).toBeInTheDocument();
    expect(screen.getByText("keyviz-minimal")).toBeInTheDocument();
    expect(screen.getByText("Sources")).toBeInTheDocument();
    expect(screen.getByText("Source Provenance")).toBeInTheDocument();
    expect(screen.getByText(":keyboard/physical-layout k-q")).toBeInTheDocument();
    expect(screen.getByText("matrix:0,1")).toBeInTheDocument();
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
