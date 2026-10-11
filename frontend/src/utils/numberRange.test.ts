import { describe, expect, it } from "vitest";
import { clampFiniteRounded, isPositiveSafeInteger, isSafeInteger } from "./numberRange";

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

describe("isPositiveSafeInteger", () => {
  it("accepts only positive safe integer values", () => {
    expect(isPositiveSafeInteger(1)).toBe(true);
    expect(isPositiveSafeInteger(Number.MAX_SAFE_INTEGER)).toBe(true);
    expect(isPositiveSafeInteger(0)).toBe(false);
    expect(isPositiveSafeInteger(-1)).toBe(false);
    expect(isPositiveSafeInteger(1.5)).toBe(false);
    expect(isPositiveSafeInteger(Number.MAX_SAFE_INTEGER + 1)).toBe(false);
    expect(isPositiveSafeInteger("1")).toBe(false);
  });
});

describe("isSafeInteger", () => {
  it("rejects values outside JavaScript's exact integer range", () => {
    expect(isSafeInteger(Number.MAX_SAFE_INTEGER)).toBe(true);
    expect(isSafeInteger(Number.MAX_SAFE_INTEGER + 1)).toBe(false);
    expect(isSafeInteger(1.5)).toBe(false);
  });
});
