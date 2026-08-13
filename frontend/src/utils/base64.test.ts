import { describe, expect, it } from "vitest";
import { base64ToBytes } from "./base64";

describe("base64ToBytes", () => {
  it("decodes raw base64 bytes", () => {
    expect(Array.from(base64ToBytes("AAH+/w=="))).toEqual([0, 1, 254, 255]);
  });
});
