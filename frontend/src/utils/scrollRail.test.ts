import { afterEach, describe, expect, it, vi } from "vitest";
import { getScrollRailCue, resolveScrollBehavior, scrollRailItemIntoView } from "./scrollRail";

afterEach(() => vi.unstubAllGlobals());

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

  it("uses an immediate scroll when the user requests reduced motion", () => {
    vi.stubGlobal("window", {
      getComputedStyle: vi.fn().mockReturnValue({ scrollPaddingLeft: "12px", scrollPaddingRight: "12px" }),
      matchMedia: vi.fn().mockReturnValue({ matches: true }),
    });
    const { rail, scrollTo } = railFixture();
    const item = { getBoundingClientRect: () => rect(280, 340) } as HTMLElement;

    scrollRailItemIntoView(rail, item, "smooth");

    expect(scrollTo).toHaveBeenCalledWith({ left: 132, behavior: "auto" });
  });
});

describe("resolveScrollBehavior", () => {
  it("keeps smooth scrolling when reduced motion is not requested", () => {
    vi.stubGlobal("window", {
      matchMedia: vi.fn().mockReturnValue({ matches: false }),
    });

    expect(resolveScrollBehavior("smooth")).toBe("smooth");
  });

  it("does not rewrite explicitly immediate scroll behavior", () => {
    vi.stubGlobal("window", {
      matchMedia: vi.fn().mockReturnValue({ matches: true }),
    });

    expect(resolveScrollBehavior("instant")).toBe("instant");
    expect(resolveScrollBehavior("auto")).toBe("auto");
  });
});

describe("getScrollRailCue", () => {
  it("does not treat reserved trailing hint space as tab content", () => {
    vi.stubGlobal("window", {
      getComputedStyle: () => ({ paddingRight: "32px" }),
    });
    const rail = { clientWidth: 320, scrollWidth: 352, scrollLeft: 0 } as HTMLElement;

    expect(getScrollRailCue(rail)).toEqual({ hasBefore: false, hasMore: false });
  });

  it("reports the remaining content at either end", () => {
    const rail = { clientWidth: 320, scrollWidth: 832, scrollLeft: 0 } as HTMLElement;

    expect(getScrollRailCue(rail)).toEqual({ hasBefore: false, hasMore: true });
    rail.scrollLeft = 512;
    expect(getScrollRailCue(rail)).toEqual({ hasBefore: true, hasMore: false });
  });
});
