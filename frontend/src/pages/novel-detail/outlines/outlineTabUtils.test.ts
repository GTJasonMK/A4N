import { describe, expect, it } from "vitest";
import {
  isValidDeletionCount,
  isValidChapterOutlineCount,
  MAX_CHAPTER_OUTLINE_GENERATION_COUNT,
  partGenerationInputError,
} from "./outlineTabUtils";

describe("chapter outline generation count", () => {
  it("accepts positive whole counts through the configured maximum", () => {
    expect(isValidChapterOutlineCount(1)).toBe(true);
    expect(isValidChapterOutlineCount(MAX_CHAPTER_OUTLINE_GENERATION_COUNT)).toBe(true);
  });

  it("rejects empty, fractional, and out-of-range counts", () => {
    expect(isValidChapterOutlineCount("")).toBe(false);
    expect(isValidChapterOutlineCount(0)).toBe(false);
    expect(isValidChapterOutlineCount(1.5)).toBe(false);
    expect(isValidChapterOutlineCount(MAX_CHAPTER_OUTLINE_GENERATION_COUNT + 1)).toBe(false);
  });
});

describe("outline deletion count", () => {
  it("accepts only whole counts that fit the current list", () => {
    expect(isValidDeletionCount(1, 4)).toBe(true);
    expect(isValidDeletionCount(4, 4)).toBe(true);
    expect(isValidDeletionCount("", 4)).toBe(false);
    expect(isValidDeletionCount(1.5, 4)).toBe(false);
    expect(isValidDeletionCount(5, 4)).toBe(false);
  });
});

describe("part generation input", () => {
  it("allows inferred chapters per part and rejects values outside the API integer contract", () => {
    expect(partGenerationInputError(1, null)).toBeNull();
    expect(partGenerationInputError(1, 6)).toBeNull();
    expect(partGenerationInputError(0, 6)).toBe("partCount");
    expect(partGenerationInputError(1, 1.5)).toBe("chaptersPerPart");
    expect(partGenerationInputError(1, 2_147_483_648)).toBe("chaptersPerPart");
    expect(partGenerationInputError(2_147_483_648, 6)).toBe("partCount");
  });
});
