import { describe, expect, it } from "vitest";
import { parseCssRgb, readCssRgbVariable, rgb, rgba, type Rgb } from "./cssColor";

const styleWith = (value: string) => ({ getPropertyValue: () => value });

describe("CSS color variables", () => {
  const fallback: Rgb = [1, 2, 3];

  it("parses the active theme channel, rgb, rgba, and hex formats", () => {
    expect(readCssRgbVariable(styleWith("139 69 19"), "--color", fallback)).toEqual([
      139, 69, 19,
    ]);
    expect(readCssRgbVariable(styleWith("rgb(74, 141, 179)"), "--color", fallback)).toEqual([
      74, 141, 179,
    ]);
    expect(readCssRgbVariable(styleWith("rgba(74 159 110 / 0.5)"), "--color", fallback)).toEqual([
      74, 159, 110,
    ]);
    expect(readCssRgbVariable(styleWith("#8b4513"), "--color", fallback)).toEqual([
      139, 69, 19,
    ]);
    expect(parseCssRgb("#8b4513", fallback)).toEqual([139, 69, 19]);
  });

  it("keeps the caller-owned fallback for an invalid custom theme value", () => {
    expect(readCssRgbVariable(styleWith("invalid"), "--color", fallback)).toBe(fallback);
    expect(rgb(fallback)).toBe("rgb(1, 2, 3)");
    expect(rgba(fallback, 0.5)).toBe("rgba(1, 2, 3, 0.5)");
  });
});
