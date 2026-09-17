import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  apiInvoke: vi.fn(),
}));

vi.mock("./index", () => ({ apiInvoke: mocks.apiInvoke }));

import {
  deleteChapterShootingScript,
  getChapterShootingScriptUnitProgress,
} from "./shootingScript";

describe("shootingScript API", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.apiInvoke.mockResolvedValue(null);
  });

  it("does not send the project id as a visual plan id for progress reads", async () => {
    await getChapterShootingScriptUnitProgress(26, 11);

    expect(mocks.apiInvoke).toHaveBeenCalledWith(
      "get_chapter_shooting_script_unit_progress",
      { input: { project_id: 26, source_chapter_id: 11 } },
    );
  });

  it("deletes the current revision with a compare-and-swap input", async () => {
    const view = {
      record: {
        id: 7,
        revision: 3,
        visual_plan_id: 2,
        visual_plan_version: 1,
        visual_plan_sha256: "a".repeat(64),
      },
      script: {} as never,
    };

    await deleteChapterShootingScript(26, 11, view);

    expect(mocks.apiInvoke).toHaveBeenCalledWith(
      "delete_chapter_shooting_script",
      {
        input: {
          project_id: 26,
          source_chapter_id: 11,
          record_id: 7,
          expected_revision: 3,
        },
      },
    );
  });
});
