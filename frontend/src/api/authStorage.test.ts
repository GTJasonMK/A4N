import { afterEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  readPortableItem: vi.fn(),
}));

vi.mock("../utils/portableStorage", () => ({
  readPortableItem: mocks.readPortableItem,
  removePortableItem: vi.fn(),
  writePortableItem: vi.fn(),
}));

import { requireCurrentUserId } from "./authStorage";

describe("requireCurrentUserId", () => {
  afterEach(() => {
    mocks.readPortableItem.mockReset();
    vi.unstubAllGlobals();
  });

  it("returns a stored positive id and rejects a missing identity", () => {
    vi.stubGlobal("window", {});
    mocks.readPortableItem.mockReturnValueOnce("42").mockReturnValueOnce(null);

    expect(requireCurrentUserId()).toBe(42);
    expect(() => requireCurrentUserId()).toThrow("未登录");
  });
});
