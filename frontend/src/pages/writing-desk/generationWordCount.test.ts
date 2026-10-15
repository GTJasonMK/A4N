import { describe, expect, it } from "vitest";
import { MAX_I32 } from "../../utils/numeric";
import {
  defaultGenerationWordCountRange,
  parseGenerationWordCountRange,
  type GenerationWordCountLabels,
} from "./generationWordCount";

const labels: GenerationWordCountLabels = {
  min: "Minimum words",
  max: "Maximum words",
  required: " is required",
  positive: " must be an i32 positive integer",
  minGreaterMax: "Maximum words cannot be less than minimum words",
};

describe("chapter generation word count range", () => {
  it("matches the Host i32 boundary for both range fields", () => {
    expect(parseGenerationWordCountRange(String(MAX_I32), String(MAX_I32), labels).range)
      .toEqual({ min: MAX_I32, max: MAX_I32 });
    expect(parseGenerationWordCountRange(String(MAX_I32 + 1), String(MAX_I32 + 1), labels).error)
      .toBe(`Minimum words${labels.positive}`);
    expect(parseGenerationWordCountRange("1", String(MAX_I32 + 1), labels).error)
      .toBe(`Maximum words${labels.positive}`);
  });

  it("surfaces an outline-derived default that cannot fit the Host contract", () => {
    const defaults = defaultGenerationWordCountRange(MAX_I32);

    expect(parseGenerationWordCountRange(String(defaults.min), String(defaults.max), labels).error)
      .toBe(`Maximum words${labels.positive}`);
  });
});
