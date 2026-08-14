import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("../index", () => ({ apiInvoke: mocks.apiInvoke }));

import { attributeHistory } from "./attributes";
import { listProfiles } from "./profiles";

describe("character API identity", () => {
  beforeEach(() => mocks.apiInvoke.mockReset().mockResolvedValue([]));

  it("uses the authenticated Host identity instead of duplicating userId", async () => {
    await listProfiles(7);
    await attributeHistory(11, { category: "social" });

    for (const [, args] of mocks.apiInvoke.mock.calls) {
      expect(args).not.toHaveProperty("userId");
    }
  });
});
