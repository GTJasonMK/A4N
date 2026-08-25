import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getChapterShootingScript: vi.fn(),
  getChapterShootingScriptUnitProgress: vi.fn(),
  getChapterVisualPlan: vi.fn(),
}));

vi.mock("../../api/shootingScript", () => mocks);
vi.mock("../../api/chapterVisualPlan", () => mocks);

import { loadShootingScriptData } from "./loadShootingScriptData";

describe("loadShootingScriptData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getChapterVisualPlan.mockResolvedValue({ confirmed: null });
    mocks.getChapterShootingScriptUnitProgress.mockResolvedValue({
      workflow_run_id: 8,
      script: { scenes: [] },
      completed_unit_count: 1,
      total_unit_count: 2,
      completed_record: null,
    });
  });

  it("skips discarded unit progress when a complete script exists", async () => {
    const view = { record: { id: 3 }, script: { scenes: [{ id: "scene-1" }] } };
    mocks.getChapterShootingScript.mockResolvedValue(view);

    await expect(loadShootingScriptData(19, 3)).resolves.toEqual({
      view,
      unitProgress: null,
      visualPlan: { confirmed: null },
    });
    expect(mocks.getChapterShootingScriptUnitProgress).not.toHaveBeenCalled();
    expect(mocks.getChapterShootingScript).toHaveBeenCalledTimes(1);
    expect(mocks.getChapterVisualPlan).toHaveBeenCalledTimes(1);
  });

  it("loads unit progress only when no complete script exists", async () => {
    mocks.getChapterShootingScript.mockResolvedValue(null);

    const loaded = await loadShootingScriptData(19, 3);

    expect(loaded.view).toBeNull();
    expect(loaded.unitProgress).not.toBeNull();
    expect(mocks.getChapterShootingScriptUnitProgress).toHaveBeenCalledWith(19, 3);
  });
});
