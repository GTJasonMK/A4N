import { describe, expect, it } from "vitest";
import { isPositiveI32 } from "./numeric";

describe("positive i32 input validation", () => {
  it("accepts the signed 32-bit positive range", () => {
    expect(isPositiveI32(1)).toBe(true);
    expect(isPositiveI32(2_147_483_647)).toBe(true);
  });

  it("rejects zero, negative, fractional, unsafe, and overflowing values", () => {
    for (const value of [0, -1, 1.5, 2_147_483_648, Number.MAX_SAFE_INTEGER + 1]) {
      expect(isPositiveI32(value)).toBe(false);
    }
  });
});
