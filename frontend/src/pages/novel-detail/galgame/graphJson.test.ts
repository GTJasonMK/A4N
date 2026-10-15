import { describe, expect, it } from "vitest";
import { containsUnrepresentableJsonNumber } from "./graphJson";

describe("containsUnrepresentableJsonNumber", () => {
  it("rejects unsafe integers and non-finite numbers in parsed JSON", () => {
    expect(containsUnrepresentableJsonNumber(JSON.parse('{"amount":9007199254740993}'))).toBe(true);
    expect(containsUnrepresentableJsonNumber(JSON.parse("1e400"))).toBe(true);
  });

  it("accepts exact numbers and decimal strings", () => {
    expect(containsUnrepresentableJsonNumber(JSON.parse('{"amount":9007199254740991}'))).toBe(false);
    expect(containsUnrepresentableJsonNumber(JSON.parse('{"amount":"9223372036854775807"}'))).toBe(false);
  });
});
