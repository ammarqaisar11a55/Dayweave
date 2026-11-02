import { act, render, screen } from "@testing-library/react";

import { showToast, toasts } from "../../lib/store";
import { Toasts } from "./Toast";

describe("Toasts", () => {
  afterEach(() => act(() => toasts.set([])));

  it("renders on the body so an open dialog can't cover an error", () => {
    const { container } = render(<Toasts />);
    act(() => showToast("Set a target above zero", { tone: "error" }));
    const message = screen.getByText("Set a target above zero");
    expect(container).not.toContainElement(message);
    expect(document.body).toContainElement(message);
  });

  it("offers the action it was given", () => {
    const run = vi.fn();
    render(<Toasts />);
    act(() => showToast("Task completed", { action: { label: "Undo", run } }));
    act(() => screen.getByRole("button", { name: "Undo" }).click());
    expect(run).toHaveBeenCalledOnce();
    expect(screen.queryByText("Task completed")).not.toBeInTheDocument();
  });
});
