import { describe, expect, it } from "vitest";
import { advancePacedStreamText } from "./usePacedStreamText";
import {
  estimateAdaptiveStreamRate,
  normalizeStreamDisplayEnabled,
  normalizeStreamDisplayMode,
  normalizeStreamDisplayRate,
  normalizeStreamDisplaySettings,
} from "../utils/streamDisplaySettings";

describe("advancePacedStreamText", () => {
  it("reveals one Unicode character at a time", () => {
    expect(advancePacedStreamText("", "猫🙂")).toBe("猫");
    expect(advancePacedStreamText("猫", "猫🙂")).toBe("猫🙂");
  });

  it("reconciles a replaced stream at its common prefix", () => {
    expect(advancePacedStreamText("旧内容", "新内容")).toBe("");
    expect(advancePacedStreamText("共同旧", "共同新")).toBe("共同");
    expect(advancePacedStreamText("😀旧", "😁新")).toBe("");
    expect(advancePacedStreamText("猫🙂旧", "猫🙂新")).toBe("猫🙂");
    expect(advancePacedStreamText("猫🙂旧", "猫🙂")).toBe("猫🙂");
    expect(advancePacedStreamText("猫🙂", "猫🙂")).toBe("猫🙂");
  });

  it("keeps the theme rate inside the supported range", () => {
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

  it("normalizes a theme stream display document", () => {
    expect(normalizeStreamDisplaySettings(undefined)).toEqual({
      enabled: false,
      mode: "fixed",
      rate: 30,
    });
    expect(normalizeStreamDisplaySettings({ enabled: true, mode: "adaptive", rate: 45 })).toEqual({
      enabled: true,
      mode: "adaptive",
      rate: 45,
    });
  });

  it("keeps stream styling opt-in", () => {
    expect(normalizeStreamDisplayEnabled(false)).toBe(false);
    expect(normalizeStreamDisplayEnabled(true)).toBe(true);
    expect(normalizeStreamDisplayEnabled("true")).toBe(false);
  });
});
