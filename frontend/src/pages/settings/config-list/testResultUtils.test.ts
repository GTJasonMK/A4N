import { describe, expect, it } from "vitest";
import { formatConfigTestTime } from "./testResultUtils";

describe("formatConfigTestTime", () => {
  it("preserves invalid timestamp text", () => {
    expect(formatConfigTestTime("not-a-date", {}, "en-US")).toBe("not-a-date");
  });

  it("formats with the active locale and optional seconds without a year", () => {
    const value = "2024-01-02T03:04:05Z";
    const date = new Date(value);
    const withoutSeconds = formatConfigTestTime(value, {}, "en-US");
    const withSeconds = formatConfigTestTime(value, { withSeconds: true }, "en-US");

    expect(withoutSeconds).toBe(date.toLocaleString("en-US", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }));
    expect(withSeconds).toBe(date.toLocaleString("en-US", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    }));
    expect(withoutSeconds).not.toContain("2024");
  });
});
