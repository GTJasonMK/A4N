import { describe, expect, it } from "vitest";
import { advancePacedStreamText } from "./usePacedStreamText";
import {
  estimateAdaptiveStreamRate,
  normalizeStreamDisplayMode,
  normalizeStreamDisplayRate,
} from "../utils/streamDisplaySettings";

describe("advancePacedStreamText", () => {
  it("reveals one Unicode character at a time", () => {
    expect(advancePacedStreamText("", "猫🙂")).toBe("猫");
    expect(advancePacedStreamText("猫", "猫🙂")).toBe("猫🙂");
  });

  it("reconciles a replaced stream at its common prefix", () => {
    expect(advancePacedStreamText("旧内容", "新内容")).toBe("");
    expect(advancePacedStreamText("共同旧", "共同新")).toBe("共同");
  });

  it("keeps the persisted rate inside the supported range", () => {
    expect(normalizeStreamDisplayRate(0)).toBe(1);
    expect(normalizeStreamDisplayRate(999)).toBe(120);
    expect(normalizeStreamDisplayRate("invalid")).toBe(30);
  });

  it("smooths adaptive samples without exceeding the supported range", () => {
    expect(estimateAdaptiveStreamRate(null, 20, 1000)).toBe(20);
    expect(estimateAdaptiveStreamRate(20, 40, 1000)).toBe(25);
    expect(estimateAdaptiveStreamRate(20, 0, 1000, 40)).toBe(20);
    expect(estimateAdaptiveStreamRate(null, 999, 1)).toBe(120);
  });

  it("falls back to fixed mode for invalid mode values", () => {
    expect(normalizeStreamDisplayMode("adaptive")).toBe("adaptive");
    expect(normalizeStreamDisplayMode("unknown")).toBe("fixed");
  });
});
