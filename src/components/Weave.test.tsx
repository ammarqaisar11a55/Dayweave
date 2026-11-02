import { render, screen } from "@testing-library/react";

import { layOut, Weave } from "./Weave";

const items = (count: number, done = 0) =>
  Array.from({ length: count }, (_, index) => ({ title: `Item ${index}`, done: index < done }));

describe("layOut", () => {
  it("pads a short list with ghost threads around the real ones", () => {
    const strands = layOut(items(2), 5, 9);
    expect(strands).toHaveLength(5);
    expect(strands.filter((strand) => strand.ghost)).toHaveLength(3);
    expect(strands[1]?.title).toBe("Item 0");
  });

  it("keeps every real item when there is room", () => {
    expect(layOut(items(9), 5, 9).every((strand) => !strand.ghost)).toBe(true);
  });

  it("keeps the proportion done when there are too many items to draw", () => {
    const strands = layOut(items(40, 20), 5, 10);
    expect(strands).toHaveLength(10);
    expect(strands.filter((strand) => strand.done)).toHaveLength(5);
  });

  it("draws an empty loom for an empty day", () => {
    expect(layOut([], 5, 9).every((strand) => strand.ghost)).toBe(true);
  });
});

describe("Weave", () => {
  it("describes itself to screen readers", () => {
    render(<Weave tasks={items(3, 1)} habits={items(2, 2)} label="3 of 5 done today" />);
    expect(screen.getByRole("img", { name: "3 of 5 done today" })).toBeInTheDocument();
  });
});
