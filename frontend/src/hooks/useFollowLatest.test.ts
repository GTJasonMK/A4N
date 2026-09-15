import { describe, expect, it } from "vitest";
import { isNearBottom, shouldFollowAfterScroll } from "./useFollowLatest";

describe("isNearBottom", () => {
  it("allows browser rounding near the latest viewport", () => {
    expect(isNearBottom({ scrollHeight: 1000, scrollTop: 875, clientHeight: 100 })).toBe(false);
    expect(isNearBottom({ scrollHeight: 1000, scrollTop: 876, clientHeight: 100 })).toBe(true);
    expect(isNearBottom({ scrollHeight: 1000, scrollTop: 900, clientHeight: 100 })).toBe(true);
  });

  it("supports an explicit tolerance for callers that own the policy", () => {
    expect(
      isNearBottom({ scrollHeight: 1000, scrollTop: 804, clientHeight: 100 }, 96),
    ).toBe(true);
  });
});

describe("shouldFollowAfterScroll", () => {
  it("stops following as soon as the user scrolls upward", () => {
    expect(
      shouldFollowAfterScroll(
        { scrollHeight: 1000, scrollTop: 850, clientHeight: 100 },
        900,
        true,
      ),
    ).toBe(false);
  });

  it("resumes following when the viewport reaches the latest bottom", () => {
    expect(
      shouldFollowAfterScroll(
        { scrollHeight: 1000, scrollTop: 900, clientHeight: 100 },
        850,
        false,
        1,
        true,
      ),
    ).toBe(true);
  });

  it("keeps a paused viewport paused across a duplicate scroll event", () => {
    expect(
      shouldFollowAfterScroll(
        { scrollHeight: 1000, scrollTop: 850, clientHeight: 100 },
        850,
        false,
      ),
    ).toBe(false);
  });

  it("does not resume a paused viewport from a programmatic scroll to the bottom", () => {
    expect(
      shouldFollowAfterScroll(
        { scrollHeight: 1000, scrollTop: 900, clientHeight: 100 },
        850,
        false,
        1,
        false,
      ),
    ).toBe(false);
  });
});
