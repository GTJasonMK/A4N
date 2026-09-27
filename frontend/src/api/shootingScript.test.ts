import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  apiInvoke: vi.fn(),
  start: vi.fn(),
}));

vi.mock("./index", () => ({ apiInvoke: mocks.apiInvoke }));
vi.mock("./generationTasks", () => ({ generationClient: { start: mocks.start } }));

import {
  deleteChapterShootingScript,
  getChapterShootingScriptUnitProgress,
  startChapterShootingScriptGeneration,
  readChapterShootingScriptGenerationResult,
} from "./shootingScript";

describe("shootingScript API", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.apiInvoke.mockResolvedValue(null);
    mocks.start.mockImplementation((start) => start({ start_request_id: "request-id", retry_of_task_id: null }));
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
      plan_sync: { stale_shot_ids: [], missing_shot_ids: [] },
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

  it("starts background work with one request identity and the selected generation mode", async () => {
    const handle = { task_id: "task-id", attempt: 1, status: "reserved", event_schema_version: 1 };
    mocks.apiInvoke.mockResolvedValue(handle);
    await expect(startChapterShootingScriptGeneration(26, 11, 7, "next_scene")).resolves.toEqual(handle);
    expect(mocks.apiInvoke).toHaveBeenCalledWith("start_chapter_shooting_script_generation", {
      input: {
        meta: { start_request_id: "request-id", retry_of_task_id: null },
        project_id: 26, source_chapter_id: 11, visual_plan_id: 7, shot_ids: [],
        mode: "next_scene",
      },
    });
  });

  it("reloads only the completed task's matching script or scene progress", async () => {
    const view = { record: { id: 7, revision: 3 }, script: {} };
    mocks.apiInvoke.mockResolvedValue(view);
    await expect(readChapterShootingScriptGenerationResult(26, 11, {
      kind: "chapter_shooting_script", id: "7", revision: "3",
    })).resolves.toEqual(view);
    await expect(readChapterShootingScriptGenerationResult(26, 11, {
      kind: "chapter_shooting_script", id: "8", revision: "3",
    })).rejects.toThrow("已变化");

    const progress = { workflow_run_id: 42, script: {
      schema_version: "chapter_shooting_script_v2",
      scenes: [{ title: "第一场", scene_index: 1 }],
      directing_statement: null,
    } };
    mocks.apiInvoke.mockResolvedValue(progress);
    const reference = { kind: "chapter_shooting_script_progress", id: "42", revision: "5285741519dd65cbef09ebde56930c75f67605655665decd5902079d6b3d865d" };
    await expect(readChapterShootingScriptGenerationResult(26, 11, reference)).resolves.toEqual(progress);
    progress.script.scenes[0].title = "a newer manual edit";
    await expect(readChapterShootingScriptGenerationResult(26, 11, reference)).rejects.toThrow("已变化");
    await expect(readChapterShootingScriptGenerationResult(26, 11, null)).rejects.toThrow("结果引用");
  });
});
