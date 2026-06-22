import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("Keyplane app", () => {
  it("renders the full-keyboard overlay as the first surface", async () => {
    render(<App />);

    expect(await screen.findByRole("region", { name: "Keyboard overlay" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /k-q q/i })).toBeInTheDocument();
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

    expect(screen.getByText(":visual/style :style/variant-id")).toBeInTheDocument();
    expect(screen.getByText("keyviz-import")).toBeInTheDocument();
    expect(screen.getByText("keyviz-minimal")).toBeInTheDocument();
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
});
