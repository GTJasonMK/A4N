import { describe, expect, it } from "vitest";
import { resolveVisibleSettingsTab } from "./settingsTabs";

describe("resolveVisibleSettingsTab", () => {
  const visibleTabIds = ["llm", "about"] as const;

  it("rejects a deep link to a hidden tab and keeps the visible tab", () => {
    expect(resolveVisibleSettingsTab("agent", "about", visibleTabIds)).toBe("about");
  });

  it("falls back when the persisted tab is no longer visible", () => {
    expect(resolveVisibleSettingsTab("agent", "agent", visibleTabIds)).toBe("llm");
  });

  it("accepts visible deep links", () => {
    expect(resolveVisibleSettingsTab("about", "llm", visibleTabIds)).toBe("about");
  });
});
