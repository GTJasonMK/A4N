import { describe, expect, it } from "vitest";

import { analysisTextList } from "./chapterAnalysisText";

describe("analysisTextList", () => {
  it("deduplicates in first-seen order before applying the limit", () => {
    expect(
      analysisTextList(["alpha", "beta", "alpha", "gamma", "beta"], {
        unique: true,
        limit: 2,
      }),
    ).toEqual(["alpha", "beta"]);
  });
});
