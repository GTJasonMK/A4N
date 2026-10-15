import { describe, expect, it } from "vitest";
import { firstDistinctText, isDistinctText } from "./text";

describe("distinct summary text", () => {
  it("skips blank values and text already shown as the summary", () => {
    expect(firstDistinctText(["  ", " same ", " place "], "same")).toBe("place");
    expect(isDistinctText(" same ", "same")).toBe(false);
    expect(isDistinctText(" place ", "same")).toBe(true);
  });

  it("keeps the first useful value when there is no summary", () => {
    expect(firstDistinctText(["  ", " first ", "second"], null)).toBe("first");
    expect(firstDistinctText([" same ", "same"], "same")).toBeNull();
    expect(isDistinctText("  ", null)).toBe(false);
  });
});
