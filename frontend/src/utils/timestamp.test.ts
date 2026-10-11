import { describe, expect, it } from "vitest";
import { parseApiTimestampMs } from "./timestamp";

describe("API timestamp parsing", () => {
  it("treats offset-free datetimes as UTC and preserves explicit offsets", () => {
    const expected = Date.parse("2026-09-27T10:00:00Z");

    expect(parseApiTimestampMs("2026-09-27 10:00:00")).toBe(expected);
    expect(parseApiTimestampMs("2026-09-27T10:00:00")).toBe(expected);
    expect(parseApiTimestampMs("2026-09-27T18:00:00+08:00")).toBe(expected);
    expect(parseApiTimestampMs("2026-09-27T10:00:00Z")).toBe(expected);
  });

  it("keeps date-only values and rejects invalid or empty values", () => {
    expect(parseApiTimestampMs("2026-09-27")).toBe(Date.parse("2026-09-27"));
    expect(parseApiTimestampMs("not a timestamp")).toBeNull();
    expect(parseApiTimestampMs("  ")).toBeNull();
    expect(parseApiTimestampMs(null)).toBeNull();
  });
});
