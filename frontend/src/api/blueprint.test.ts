import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("./index", () => ({
  apiInvoke: mocks.apiInvoke,
  extractErrorMessage: (error: unknown, fallback: string) =>
    error instanceof Error ? error.message : fallback,
}));
vi.mock("./authStorage", () => ({ requireCurrentUserId: () => 42 }));
vi.mock("./generationTasks", () => ({ generationClient: { start: vi.fn() } }));

import {
  matchesBlueprintTaskResult,
  syncBlueprintMaterials,
  type Blueprint,
} from "./blueprint";

const blueprint: Blueprint = {
  id: 9,
  project_id: 7,
  title: "雾城",
  genre: "悬疑",
  style: null,
  tone: null,
  target_audience: null,
  one_sentence_summary: null,
  full_synopsis: null,
  world_setting: null,
  current_version: "2.0",
  total_chapters: 30,
  chapters_per_part: null,
  source_review_status: "not_required",
};

beforeEach(() => mocks.apiInvoke.mockReset());

describe("blueprint formal result and material sync", () => {
  it("accepts only the persisted blueprint named by the terminal result", () => {
    expect(matchesBlueprintTaskResult(
      blueprint,
      7,
      { kind: "novel_blueprint", id: "9", revision: "2.0" },
    )).toBe(true);
    expect(matchesBlueprintTaskResult(
      blueprint,
      7,
      { kind: "novel_blueprint", id: "9", revision: "1.0" },
    )).toBe(false);
    expect(matchesBlueprintTaskResult(
      blueprint,
      8,
      { kind: "novel_blueprint", id: "9", revision: "2.0" },
    )).toBe(false);
    expect(matchesBlueprintTaskResult(
      blueprint,
      7,
      { kind: "chapter", id: "9", revision: "2.0" },
    )).toBe(false);
  });

  it("uses the project-scoped material sync command", async () => {
    mocks.apiInvoke.mockResolvedValueOnce({ warnings: [] });

    await expect(syncBlueprintMaterials(7)).resolves.toEqual([]);
    expect(mocks.apiInvoke).toHaveBeenCalledWith("sync_blueprint_materials", {
      projectId: 7,
    });
  });

  it("reports a sync command failure without turning the formal blueprint into a failure", async () => {
    mocks.apiInvoke.mockRejectedValueOnce(new Error("向量服务不可用"));

    await expect(syncBlueprintMaterials(7)).resolves.toEqual([
      expect.stringContaining("向量服务不可用"),
    ]);
  });
});
