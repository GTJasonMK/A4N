import { describe, expect, it } from "vitest";
import { isCancelledOperation } from "./cancellation";

describe("isCancelledOperation", () => {
  it("recognizes localized and English stop messages", () => {
    expect(isCancelledOperation("任务已停止")).toBe(true);
    expect(isCancelledOperation("Generation stopped")).toBe(true);
    expect(isCancelledOperation("Generation canceled")).toBe(true);
    expect(isCancelledOperation("Generation failed")).toBe(false);
  });
});
