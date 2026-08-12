import { beforeEach, describe, expect, it, vi } from "vitest";
import type { GenerationTaskSummary } from "./generated/generationContracts";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("./index", () => ({
  apiInvoke: mocks.apiInvoke,
  extractErrorMessage: (error: unknown, fallback: string) =>
    typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : fallback,
}));

import {
  chapterContentCommitToastType,
  chapterGenerationNumber,
  generateChapterAnalysis,
  generateChapterScene,
  deleteChapterScene,
  listChapterScenes,
  previewChapterAnalysisRequest,
  previewChapterSceneRequest,
  previewResumeChapterAnalysisSyncRequest,
  reorderChapterScenes,
  resumeChapterAnalysisSync,
  shouldPropagateGenerationRagPrefetchError,
  updateChapterScene,
} from "./chapter";

beforeEach(() => mocks.apiInvoke.mockReset());

const summary = (targetId: string, kind = "chapter"): GenerationTaskSummary => ({
  handle: {
    attempt: 1,
    event_schema_version: 1,
    status: "running",
    task_id: "123e4567-e89b-42d3-a456-426614174000",
  },
  result_ref: null,
  snapshot_state: "available",
  started: {
    delivery_mode: "stream",
    project_id: 7,
    root_task_id: "123e4567-e89b-42d3-a456-426614174000",
    target: { id: targetId, kind },
    task_kind: "llm_call",
  },
  terminal_event: null,
  updated_at_ms: 1,
});

describe("chapter generation target", () => {
  it("accepts only the canonical target for the current project", () => {
    expect(chapterGenerationNumber(summary("7:3"), 7)).toBe(3);
    expect(chapterGenerationNumber(summary("8:3"), 7)).toBeNull();
    expect(chapterGenerationNumber(summary("7:3:1"), 7)).toBeNull();
    expect(chapterGenerationNumber(summary("7:3", "image"), 7)).toBeNull();
  });

  it("propagates RAG contract errors but allows optional capability failures", () => {
    expect(
      shouldPropagateGenerationRagPrefetchError("Embedding identity 不匹配"),
    ).toBe(true);
    expect(
      shouldPropagateGenerationRagPrefetchError("参数无效：query vector 维度不匹配"),
    ).toBe(true);
    expect(
      shouldPropagateGenerationRagPrefetchError("嵌入计算失败：本机模型未安装"),
    ).toBe(false);
    expect(
      shouldPropagateGenerationRagPrefetchError("向量检索失败：服务暂时不可用"),
    ).toBe(false);
  });

  it("keeps a committed chapter successful when derived sync needs recovery", () => {
    expect(chapterContentCommitToastType("failed")).toBe("info");
    expect(chapterContentCommitToastType("running")).toBe("info");
    expect(chapterContentCommitToastType("completed")).toBe("success");
  });
});

describe("chapter analysis host contract", () => {
  it("sends only the canonical input and optional task identity", async () => {
    const input = { chapter_id: 9 };

    await generateChapterAnalysis(input, "analysis-task");
    await previewChapterAnalysisRequest(input);
    await resumeChapterAnalysisSync(input);
    await previewResumeChapterAnalysisSyncRequest(input);

    expect(mocks.apiInvoke.mock.calls).toEqual([
      ["generate_chapter_analysis", { input, taskId: "analysis-task" }],
      ["preview_chapter_analysis_request", { input }],
      ["resume_chapter_analysis_sync", { input, taskId: null }],
      ["preview_resume_chapter_analysis_sync_request", { input }],
    ]);
  });
});

describe("chapter scene host contract", () => {
  it("sends only canonical scene inputs and task identity", async () => {
    const generationInput = {
      chapter_id: 9,
      scene_number: 2,
      scene_prompt: "推进码头冲突",
    };
    const updateInput = { scene_id: 11, title: "码头" };
    const reorderInput = { order: [[11, 2], [12, 1]] as [number, number][] };

    await listChapterScenes(9);
    await generateChapterScene(generationInput, "scene-task");
    await previewChapterSceneRequest(generationInput);
    await updateChapterScene(updateInput);
    await reorderChapterScenes(reorderInput);
    await deleteChapterScene(11);

    expect(mocks.apiInvoke.mock.calls).toEqual([
      ["list_chapter_scenes", { chapterId: 9 }],
      ["generate_chapter_scene", { input: generationInput, taskId: "scene-task" }],
      ["preview_chapter_scene_request", { input: generationInput }],
      ["update_chapter_scene", { input: updateInput }],
      ["reorder_chapter_scenes", { input: reorderInput }],
      ["delete_chapter_scene", { sceneId: 11 }],
    ]);
  });
});
