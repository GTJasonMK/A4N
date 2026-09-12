import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("../index", () => ({ apiInvoke: mocks.apiInvoke }));

import { attributeHistory } from "./attributes";
import { listProfiles } from "./profiles";

describe("character API identity", () => {
  beforeEach(() => mocks.apiInvoke.mockReset().mockResolvedValue([]));

  it("maps profile and attribute history inputs to their Host contracts", async () => {
    await listProfiles(7);
    await attributeHistory(11, { category: "social" });

    expect(mocks.apiInvoke.mock.calls).toEqual([
      ["list_profiles", { input: { project_id: 7 } }],
      [
        "attribute_history",
        {
          input: {
            profile_id: 11,
            chapter_from: null,
            chapter_to: null,
            category: "social",
            key: null,
            operation: null,
            limit: null,
          },
        },
      ],
    ]);
  });
});
