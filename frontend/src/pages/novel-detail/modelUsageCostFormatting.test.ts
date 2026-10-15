import { describe, expect, it } from "vitest";
import { formatMicros } from "./modelUsageCostFormatting";

describe("formatMicros", () => {
  it("preserves large signed 64-bit micro amounts", () => {
    expect(formatMicros("9223372036854775807", "en-US")).toBe("9,223,372,036,854.775807");
    expect(formatMicros("-1000000", "en-US")).toBe("-1.00");
  });

  it("uses the locale decimal separator and minimum two fraction digits", () => {
    expect(formatMicros("1230000", "de-DE")).toBe("1,23");
    expect(formatMicros("1000000", "de-DE")).toBe("1,00");
  });
});
