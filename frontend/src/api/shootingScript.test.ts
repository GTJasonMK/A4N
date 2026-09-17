import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  apiInvoke: vi.fn(),
}));

vi.mock("./index", () => ({ apiInvoke: mocks.apiInvoke }));

import { getChapterShootingScriptUnitProgress } from "./shootingScript";

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
});
