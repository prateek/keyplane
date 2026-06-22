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

  it("surfaces Source Conflicts outside the overlay", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /source inspector/i }));

    expect(screen.getByText(":visual/style :style/variant-id")).toBeInTheDocument();
    expect(screen.getByText(/keyviz-import: keyviz-minimal/)).toBeInTheDocument();
  });
});
