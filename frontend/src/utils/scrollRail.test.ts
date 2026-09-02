import { describe, expect, it, vi } from "vitest";
import { scrollRailItemIntoView } from "./scrollRail";

const rect = (left: number, right: number): DOMRect => ({ left, right } as DOMRect);

function railFixture(scrollLeft = 100) {
  const scrollTo = vi.fn();
  const rail = {
    clientWidth: 320,
    scrollWidth: 800,
    scrollLeft,
    getBoundingClientRect: () => rect(0, 320),
    scrollTo,
  } as unknown as HTMLElement;
  return { rail, scrollTo };
}

describe("scrollRailItemIntoView", () => {
  it("does not move an item already inside the visible rail", () => {
    const { rail, scrollTo } = railFixture();
    const item = { getBoundingClientRect: () => rect(120, 200) } as HTMLElement;

    scrollRailItemIntoView(rail, item, "smooth");

    expect(scrollTo).not.toHaveBeenCalled();
  });

  it("moves only the clipped distance on the right", () => {
    const { rail, scrollTo } = railFixture();
    const item = { getBoundingClientRect: () => rect(280, 340) } as HTMLElement;

    scrollRailItemIntoView(rail, item, "smooth");

    expect(scrollTo).toHaveBeenCalledWith({ left: 132, behavior: "smooth" });
  });

  it("moves only the clipped distance on the left", () => {
    const { rail, scrollTo } = railFixture(200);
    const item = { getBoundingClientRect: () => rect(-20, 40) } as HTMLElement;

    scrollRailItemIntoView(rail, item, "instant");

    expect(scrollTo).toHaveBeenCalledWith({ left: 168, behavior: "instant" });
  });
});
