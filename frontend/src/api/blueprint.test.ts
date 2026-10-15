import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("./index", () => ({
  apiInvoke: mocks.apiInvoke,
  extractErrorMessage: (error: unknown, fallback: string) =>
    error instanceof Error ? error.message : fallback,
}));
vi.mock("./generationTasks", () => ({ generationClient: { start: vi.fn() } }));

import {
  diffBlueprintVersions,
  getBlueprint,
  listBlueprintVersions,
  matchesBlueprintTaskResult,
  previewGenerateBlueprintRequest,
  previewRefineBlueprintRequest,
  restoreBlueprintVersion,
  saveBlueprint,
  syncBlueprintMaterials,
  updateWorldSetting,
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

  it("uses authenticated Host identity and canonical blueprint envelopes", async () => {
    mocks.apiInvoke.mockResolvedValue(undefined);

    await getBlueprint(7);
    await previewGenerateBlueprintRequest({
      project_id: 7,
      title: "雾城",
      genre: "悬疑",
      target_length: 30,
      variables: null,
    });
    await previewRefineBlueprintRequest({ project_id: 7, focus: "加强动机", variables: null });
    await listBlueprintVersions(7);
    await diffBlueprintVersions({ project_id: 7, version_a: "1.0", version_b: "2.0" });
    await restoreBlueprintVersion({ project_id: 7, version: "1.0" });
    await saveBlueprint({
      project_id: 7,
      expected_current_version: "2.0",
      genre: "悬疑",
      style: null,
      tone: null,
      target_audience: null,
      one_sentence_summary: null,
      full_synopsis: null,
      total_chapters: 30,
    });
    await updateWorldSetting({
      project_id: 7,
      expected_current_version: "2.0",
      world_setting: { premise: "迷雾倒流" },
    });

    expect(mocks.apiInvoke.mock.calls).toEqual([
      ["get_blueprint", { projectId: 7 }],
      ["preview_generate_blueprint_request", {
        input: {
          project_id: 7,
          title: "雾城",
          genre: "悬疑",
          target_length: 30,
          variables: null,
        },
      }],
      ["preview_refine_blueprint_request", {
        input: { project_id: 7, focus: "加强动机", variables: null },
      }],
      ["list_blueprint_versions", { input: { project_id: 7 } }],
      ["diff_blueprint_versions", {
        input: { project_id: 7, version_a: "1.0", version_b: "2.0" },
      }],
      ["restore_blueprint_version", { input: { project_id: 7, version: "1.0" } }],
      ["save_blueprint", {
        input: {
          project_id: 7,
          expected_current_version: "2.0",
          genre: "悬疑",
          style: null,
          tone: null,
          target_audience: null,
          one_sentence_summary: null,
          full_synopsis: null,
          total_chapters: 30,
        },
      }],
      ["update_world_setting", {
        input: {
          project_id: 7,
          expected_current_version: "2.0",
          world_setting: { premise: "迷雾倒流" },
        },
      }],
    ]);
  });

  it("reports a sync command failure without turning the formal blueprint into a failure", async () => {
    mocks.apiInvoke.mockRejectedValueOnce(new Error("向量服务不可用"));

    await expect(syncBlueprintMaterials(7)).resolves.toEqual([
      expect.stringContaining("向量服务不可用"),
    ]);
  });
});
