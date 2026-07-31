import { beforeEach, describe, expect, it, vi } from "vitest";
import type { GenerationTaskSummary } from "../../../api/generated/generationContracts";
import type { ChapterRow } from "../shared";

const mocks = vi.hoisted(() => ({
  listChapterVideoPrompts: vi.fn(),
  listVideosByChapter: vi.fn(),
  listVideoReferenceImages: vi.fn(),
  listEffectiveVideoConfigs: vi.fn(),
  listWorkflowRuns: vi.fn(),
  loadVideoSegmentPlanDraft: vi.fn(),
}));

vi.mock("../../../api/image", () => ({
  listVideoReferenceImages: mocks.listVideoReferenceImages,
}));

vi.mock("../../../api/streaming", () => ({
  listWorkflowRuns: mocks.listWorkflowRuns,
}));

vi.mock("../../../api/video", () => ({
  listChapterVideoPrompts: mocks.listChapterVideoPrompts,
  listVideosByChapter: mocks.listVideosByChapter,
  listEffectiveVideoConfigs: mocks.listEffectiveVideoConfigs,
  loadVideoSegmentPlanDraft: mocks.loadVideoSegmentPlanDraft,
}));

import {
  activeVideoSegmentPlanTask,
  loadVideoSegmentPlanDraft,
  loadVideoTabData,
  videoSegmentPlanGenerationTargetId,
} from "./loadVideoTabData";

const segmentPlanFingerprint = (
  optionOverrides: Record<string, unknown> = {},
): Record<string, unknown> => ({
  schema_version: 1,
  workflow_kind: "video_segment_plan",
  project_id: 19,
  chapter_id: 3,
  workflow_mode: "shared_segment_plan",
  source: { content_sha256: "source-hash" },
  options: {
    style: "原始规划风格",
    prompt_output_language: "en",
    prompt_batch_size: 1,
    segment_plan_prompt_id: null,
    target_shot_number: null,
    segment_recipe_fingerprint: null,
    duration_secs: 10,
    duration_secs_options: [6, 10],
    aspect_ratio: "9:16",
    reference_image_ids: [],
    video_library_image_ids: [],
    duration_capability: null,
    regenerate_existing: true,
    ...optionOverrides,
  },
});

describe("video tab loading", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listChapterVideoPrompts.mockResolvedValue([]);
    mocks.listVideosByChapter.mockResolvedValue([]);
    mocks.listVideoReferenceImages.mockResolvedValue([]);
    mocks.listEffectiveVideoConfigs.mockResolvedValue([]);
  });

  it("does not block base video data on workflow draft loading", async () => {
    await expect(
      loadVideoTabData(19, { id: 3 } as ChapterRow, [19]),
    ).resolves.toMatchObject({
      prompts: [],
      videos: [],
      images: [],
      activeVideoConfig: null,
    });

    expect(mocks.listWorkflowRuns).not.toHaveBeenCalled();
    expect(mocks.loadVideoSegmentPlanDraft).not.toHaveBeenCalled();
  });

  it("loads the newest workflow and restores each segment recipe independently", async () => {
    mocks.listWorkflowRuns.mockResolvedValue([{
      id: 8,
      project_id: 19,
      chapter_id: 3,
      workflow_kind: "video_segment_plan",
      status: "cancelled",
      can_resume: true,
    }]);
    mocks.loadVideoSegmentPlanDraft.mockResolvedValue({
      run: {
        id: 8,
        project_id: 19,
        chapter_id: 3,
        workflow_kind: "video_segment_plan",
        status: "cancelled",
        can_resume: true,
        input_fingerprint_json: segmentPlanFingerprint(),
        progress: {
          current: 87,
          total: 100,
          message: "已完成镜头 1",
          percent: 87,
          meta: null,
          updated_at: "2026-07-29T00:00:00Z",
        },
      },
      checkpoints: [{
        id: 1,
        workflow_run_id: 8,
        step_key: "video_segment_plan.shot.unit_1_shot_1",
        step_order: 200,
        status: "completed",
        input_fingerprint_json: { extra: { shot_index: 1, total_shots: 2 } },
        checkpoint_json: { parsed_output: [
          {
            segment_number: 1,
            source_shot_id: "unit_1_shot_1",
            duration_secs: 10,
            aspect_ratio: "9:16",
          },
          {
            segment_number: 2,
            source_shot_id: "unit_1_shot_1",
            duration_secs: 10,
            aspect_ratio: "9:16",
          },
        ] },
        artifacts_json: { segment_recipes: [
          {
            schema_version: "video_segment_execution_recipe_v1",
            segment_number: 1,
            workflow_mode: "chat_video",
          },
          {
            schema_version: "video_segment_execution_recipe_v1",
            segment_number: 2,
            workflow_mode: "storyboard",
          },
        ] },
        generation_run_ids_json: null,
        error_message: null,
        created_at: "2026-07-29T00:00:00Z",
        updated_at: "2026-07-29T00:00:00Z",
        completed_at: "2026-07-29T00:00:00Z",
      }],
      generation_runs: [],
    });

    const loaded = await loadVideoSegmentPlanDraft(19, 3);
    expect(loaded).toMatchObject({
      workflowRunId: 8,
      workflowStatus: "cancelled",
      resumable: true,
      completedShotCount: 1,
      totalShotCount: 2,
      resumeOptions: {
        workflow_mode: "shared_segment_plan",
        style: "原始规划风格",
        prompt_output_language: "en",
        prompt_batch_size: 1,
        duration_secs: 10,
        duration_secs_options: [6, 10],
        aspect_ratio: "9:16",
        reference_image_ids: [],
        video_library_image_ids: [],
        regenerate_existing: true,
      },
      workflowProgress: {
        current: 87,
        total: 100,
        message: "已完成镜头 1",
        percent: 87,
      },
    });
    expect(loaded?.recipes.get(1)?.workflow_mode).toBe("chat_video");
    expect(loaded?.recipes.get(2)?.workflow_mode).toBe("storyboard");
    expect(mocks.listWorkflowRuns).toHaveBeenCalledWith({
      project_id: 19,
      chapter_id: 3,
      workflow_kind: "video_segment_plan",
      limit: 1,
    });
    expect(mocks.loadVideoSegmentPlanDraft).toHaveBeenCalledWith(19, 3, 8);
  });

  it("reports a damaged newest draft instead of silently falling back to an older run", async () => {
    mocks.listWorkflowRuns.mockResolvedValue([{ id: 10, status: "failed", can_resume: true }]);
    mocks.loadVideoSegmentPlanDraft.mockResolvedValue({
      run: {
        id: 10,
        project_id: 19,
        chapter_id: 3,
        workflow_kind: "video_segment_plan",
        status: "failed",
        can_resume: true,
      },
      checkpoints: [{
        id: 10,
        workflow_run_id: 10,
        step_key: "video_segment_plan.shot.unit_1_shot_1",
        step_order: 200,
        status: "completed",
        input_fingerprint_json: { extra: { shot_index: 1, total_shots: 2 } },
        checkpoint_json: { parsed_output: [] },
        artifacts_json: null,
        generation_run_ids_json: null,
        error_message: null,
        created_at: "2026-07-29T00:00:00Z",
        updated_at: "2026-07-29T00:00:00Z",
        completed_at: "2026-07-29T00:00:00Z",
      }],
      generation_runs: [],
    });

    await expect(loadVideoSegmentPlanDraft(19, 3)).rejects.toThrow(
      "缺少已校验 segments",
    );
  });

  it("rejects damaged resume options instead of using current form defaults", async () => {
    mocks.listWorkflowRuns.mockResolvedValue([{ id: 11, status: "cancelled", can_resume: true }]);
    mocks.loadVideoSegmentPlanDraft.mockResolvedValue({
      run: {
        id: 11,
        project_id: 19,
        chapter_id: 3,
        workflow_kind: "video_segment_plan",
        status: "cancelled",
        can_resume: true,
        input_fingerprint_json: segmentPlanFingerprint({ duration_secs_options: [10, 6] }),
        progress: null,
      },
      checkpoints: [{
        id: 11,
        workflow_run_id: 11,
        step_key: "video_segment_plan.shot.unit_1_shot_1",
        step_order: 200,
        status: "completed",
        input_fingerprint_json: { extra: { shot_index: 1, total_shots: 2 } },
        checkpoint_json: { parsed_output: [{
          segment_number: 1,
          source_shot_id: "unit_1_shot_1",
          duration_secs: 10,
          aspect_ratio: "9:16",
        }] },
        artifacts_json: { segment_recipes: [{
          schema_version: "video_segment_execution_recipe_v1",
          segment_number: 1,
          workflow_mode: "chat_video",
        }] },
        generation_run_ids_json: null,
        error_message: null,
        created_at: "2026-07-29T00:00:00Z",
        updated_at: "2026-07-29T00:00:00Z",
        completed_at: "2026-07-29T00:00:00Z",
      }],
      generation_runs: [],
    });

    await expect(loadVideoSegmentPlanDraft(19, 3)).rejects.toThrow(
      "options.duration_secs_options 无效，无法安全续跑",
    );
  });

  it("matches the active segment-plan task by canonical target", () => {
    const targetId = videoSegmentPlanGenerationTargetId(19, 3);
    const summary = (taskId: string, updatedAt: number, target: string): GenerationTaskSummary => ({
      handle: {
        event_schema_version: 1,
        task_id: taskId,
        attempt: 1,
        status: "running",
      },
      started: {
        task_kind: "llm_call",
        delivery_mode: "stream",
        project_id: 19,
        root_project_id: null,
        workflow_run_id: null,
        generation_run_id: null,
        target: { kind: "video_prompt_segment_plan", id: target, revision: null },
        batch_policy: "best_effort",
        root_task_id: taskId,
        parent_task_id: null,
        item_key: null,
        retry_of_task_id: null,
      },
      updated_at_ms: updatedAt,
      result_ref: null,
      snapshot_state: "available",
      terminal_event: null,
    });

    expect(
      activeVideoSegmentPlanTask([
        summary("old", 1, targetId),
        summary("other", 3, videoSegmentPlanGenerationTargetId(19, 4)),
        summary("current", 2, targetId),
      ], 19, 3)?.handle.task_id,
    ).toBe("current");
  });
});
