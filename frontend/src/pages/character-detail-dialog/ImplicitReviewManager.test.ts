import { describe, expect, it } from "vitest";
import { parseReviewParameter } from "./ImplicitReviewManager";

describe("implicit review parameter boundaries", () => {
  it("rejects malformed and out-of-range chapter windows without rewriting them", () => {
    expect(parseReviewParameter("51", 50)).toBeNull();
    expect(parseReviewParameter("1.5", 50)).toBeNull();
    expect(parseReviewParameter("0", 50)).toBeNull();
    expect(parseReviewParameter("10", 50)).toBe(10);
  });

  it("keeps the backend threshold contract independent from the window limit", () => {
    expect(parseReviewParameter("100")).toBe(100);
  });
});
