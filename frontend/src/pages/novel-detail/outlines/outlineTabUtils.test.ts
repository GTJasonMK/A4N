import { describe, expect, it } from "vitest";
import {
  isValidDeletionCount,
  isValidChapterOutlineCount,
  MAX_CHAPTER_OUTLINE_GENERATION_COUNT,
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
