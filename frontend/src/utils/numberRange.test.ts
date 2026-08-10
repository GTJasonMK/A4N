import { describe, expect, it } from "vitest";
import { clampFiniteRounded } from "./numberRange";

describe("clampFiniteRounded", () => {
  it("rounds values inside the range", () => {
    expect(clampFiniteRounded(221.6, 180, 400, 240)).toBe(222);
  });

  it("clips values outside the range", () => {
    expect(clampFiniteRounded(100, 180, 400, 240)).toBe(180);
    expect(clampFiniteRounded(500, 180, 400, 240)).toBe(400);
  });

  it("uses the fallback only for non-finite values", () => {
    expect(clampFiniteRounded(Number.NaN, 180, 400, 240)).toBe(240);
    expect(clampFiniteRounded(Number.POSITIVE_INFINITY, 180, 400, 240)).toBe(240);
  });
});
