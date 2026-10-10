import { describe, expect, it } from "vitest";
import { isSupportedImageMime, supportedImageAccept } from "./imageUpload";

describe("image upload formats", () => {
  it("uses the generated upload MIME contract", () => {
    expect(supportedImageAccept()).toBe("image/png,image/jpeg,image/webp");
    expect(isSupportedImageMime(" image/PNG ")).toBe(true);
    expect(isSupportedImageMime("image/gif")).toBe(false);
  });
});
