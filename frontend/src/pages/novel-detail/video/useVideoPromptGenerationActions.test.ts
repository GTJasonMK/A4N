import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ChapterRow } from "../shared";
import type { ChapterVideoPrompt, VideoSegmentExecutionRecipe } from "../../../api/video";
import type { LoadedSegmentPlanDraft } from "./loadVideoTabData";
import {
  DEFAULT_VIDEO_OPTIONS,
  videoDurationLimitsFromConfig,
} from "./videoTabUtils";

const mocks = vi.hoisted(() => ({
  startChapterVideoFramePrompts: vi.fn(),
  startChapterVideoPrompts: vi.fn(),
  startChapterVideoSegmentPlan: vi.fn(),
  runGenerationProgressTaskThen: vi.fn(),
  confirmDialog: vi.fn(),
}));

vi.mock("../../../api/video", () => ({
  startChapterVideoFramePrompts: mocks.startChapterVideoFramePrompts,
  startChapterVideoPrompts: mocks.startChapterVideoPrompts,
  startChapterVideoSegmentPlan: mocks.startChapterVideoSegmentPlan,
}));

vi.mock("../../../api/image", () => ({
  startImageGeneration: vi.fn(),
  uploadImage: vi.fn(),
  verifyImageGenerationResult: vi.fn(),
}));

vi.mock("../../../api/generationTasks", () => ({
  generationClient: {
    cancel: vi.fn(),
    waitForTerminal: vi.fn(),
  },
}));

vi.mock("../../../components/feedback/ConfirmDialog", () => ({
  confirmDialog: mocks.confirmDialog,
}));

vi.mock("../../../hooks/runGenerationProgressTask", () => ({
  runGenerationProgressTaskThen: mocks.runGenerationProgressTaskThen,
}));

import { useVideoPromptGenerationActions } from "./useVideoPromptGenerationActions";

describe("video segment-plan generation actions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.startChapterVideoSegmentPlan.mockResolvedValue({
      event_schema_version: 1,
      task_id: "resume-task",
      attempt: 1,
      status: "running",
    });
    mocks.startChapterVideoFramePrompts.mockResolvedValue({
      event_schema_version: 1,
      task_id: "frame-task",
      attempt: 1,
      status: "running",
    });
    mocks.startChapterVideoPrompts.mockResolvedValue({
      event_schema_version: 1,
      task_id: "video-prompt-task",
      attempt: 1,
      status: "running",
    });
    mocks.runGenerationProgressTaskThen.mockImplementation(
      async (_runProgressTask, _handle, _options, afterTerminal, onUpdate) => {
        onUpdate?.({
          latestCheckpoint: { checkpoint_key: "video_segment_plan.shot.shot-2" },
          streams: { segment_plan_draft: { text: "" } },
        });
        return afterTerminal({});
      },
    );
  });

  it("continues with the persisted workflow options instead of the current form", async () => {
    const resumeOptions: LoadedSegmentPlanDraft["resumeOptions"] = {
      workflow_mode: "shared_segment_plan",
      style: "首次运行风格",
      prompt_output_language: "en",
      prompt_batch_size: 1,
      duration_secs: 10,
      duration_secs_options: [6, 10],
      aspect_ratio: "9:16",
      reference_image_ids: [],
      video_library_image_ids: [],
      regenerate_existing: false,
    };
    const segmentPlanDraft: LoadedSegmentPlanDraft = {
      workflowRunId: 8,
      workflowStatus: "cancelled",
      resumeOptions,
      workflowProgress: {
        current: 87,
        total: 100,
        message: "已完成镜头 1",
        percent: 87,
        meta: null,
        updated_at: "2026-07-29T00:00:00Z",
      },
      segments: [{ segment_number: 1 }],
      recipes: new Map(),
      completedShotCount: 1,
      totalShotCount: 3,
    };
    const addToast = vi.fn();
    const refreshPrompts = vi.fn().mockResolvedValue(undefined);
    const refreshSegmentPlanDraft = vi.fn().mockResolvedValue(segmentPlanDraft);
    const actions = useVideoPromptGenerationActions({
      projectId: 19,
      chapter: { id: 3 } as ChapterRow,
      chapterGenerating: false,
      options: DEFAULT_VIDEO_OPTIONS,
      segmentPlanOptions: { ...DEFAULT_VIDEO_OPTIONS, style: "" },
      durationLimits: videoDurationLimitsFromConfig(null),
      promptInputSelectionError: "当前表单无效",
      referenceIds: [],
      videoLibraryImageIds: [],
      frameImages: [],
      segmentPlanPrompt: null,
      segmentPlanDraft,
      framePrompts: [],
      allFramePrompts: [],
      allFinalVideoPrompts: [],
      currentContentHash: null,
      actionScopeKey: "19:3",
      addToast,
      runProgressTask: vi.fn(),
      loadAll: vi.fn().mockResolvedValue(undefined),
      refreshPrompts,
      refreshSegmentPlanDraft,
      setSegmentPlanPreview: vi.fn(),
      beginAction: vi.fn().mockReturnValue(true),
      isCurrentActionScope: vi.fn().mockReturnValue(true),
      finishAction: vi.fn(),
    });

    await actions.handleGenerateSegmentPlan();

    expect(mocks.startChapterVideoSegmentPlan).toHaveBeenCalledWith(19, 3, resumeOptions);
    expect(mocks.runGenerationProgressTaskThen.mock.calls[0]?.[2]).toMatchObject({
      startMessage: expect.stringContaining("已加载持久化进度 87%"),
    });
    expect(refreshSegmentPlanDraft).toHaveBeenCalledOnce();
    expect(addToast).not.toHaveBeenCalledWith(expect.stringContaining("请填写"), "error");
  });

  it("supports fresh planning followed by boundary, first-frame, storyboard, and Chat chains", async () => {
    const commonParams: Parameters<typeof useVideoPromptGenerationActions>[0] = {
      projectId: 19,
      chapter: { id: 3 } as ChapterRow,
      chapterGenerating: false,
      options: DEFAULT_VIDEO_OPTIONS,
      segmentPlanOptions: DEFAULT_VIDEO_OPTIONS,
      durationLimits: videoDurationLimitsFromConfig(null),
      promptInputSelectionError: null,
      referenceIds: [],
      videoLibraryImageIds: [],
      frameImages: [],
      segmentPlanPrompt: null,
      segmentPlanDraft: null,
      framePrompts: [],
      allFramePrompts: [],
      allFinalVideoPrompts: [],
      currentContentHash: null,
      actionScopeKey: "19:3",
      addToast: vi.fn(),
      runProgressTask: vi.fn(),
      loadAll: vi.fn().mockResolvedValue(undefined),
      refreshPrompts: vi.fn().mockResolvedValue(undefined),
      refreshSegmentPlanDraft: vi.fn().mockResolvedValue(null),
      setSegmentPlanPreview: vi.fn(),
      beginAction: vi.fn().mockReturnValue(true),
      isCurrentActionScope: vi.fn().mockReturnValue(true),
      finishAction: vi.fn(),
    };

    await useVideoPromptGenerationActions(commonParams).handleGenerateSegmentPlan();

    expect(mocks.startChapterVideoSegmentPlan).toHaveBeenCalledWith(
      19,
      3,
      expect.objectContaining({
        workflow_mode: "shared_segment_plan",
        reference_image_ids: [],
        video_library_image_ids: [],
      }),
    );

    const recipe = (
      segmentNumber: number,
      workflowMode: VideoSegmentExecutionRecipe["workflow_mode"],
    ): VideoSegmentExecutionRecipe => ({
      schema_version: "video_segment_execution_recipe_v1",
      segment_number: segmentNumber,
      workflow_mode: workflowMode,
      style: `片段 ${segmentNumber} 风格`,
      prompt_output_language: "zh_cn",
      duration_secs: 6,
      duration_secs_options: [6],
      aspect_ratio: "16:9",
      reference_image_ids: [100 + segmentNumber],
      video_library_image_ids: [200 + segmentNumber],
      guidance_mode: workflowMode === "chat_video" ? "none" : "system_frames",
      guidance_reference_image_ids: [],
      continuity_source_video_id: null,
      reference_media: null,
      adopted_video_id: null,
    });
    const chains = [
      { recipe: recipe(2, "boundary_frames"), frameBatchSize: 2 },
      { recipe: recipe(1, "first_frame_chain"), frameBatchSize: 1 },
      { recipe: recipe(3, "storyboard"), frameBatchSize: 1 },
      { recipe: recipe(4, "chat_video"), frameBatchSize: null },
    ];
    const actions = useVideoPromptGenerationActions({
      ...commonParams,
      segmentPlanPrompt: { id: 44 } as ChapterVideoPrompt,
    });

    for (const chain of chains) {
      if (chain.frameBatchSize != null) {
        await actions.handleGenerateFramePrompts(chain.recipe);
      }
      await actions.handleGenerateVideoPrompts(chain.recipe);
    }

    expect(mocks.startChapterVideoFramePrompts).toHaveBeenCalledTimes(3);
    chains.slice(0, 3).forEach((chain, index) => {
      expect(mocks.startChapterVideoFramePrompts).toHaveBeenNthCalledWith(
        index + 1,
        19,
        3,
        expect.objectContaining({
          workflow_mode: chain.recipe.workflow_mode,
          segment_plan_prompt_id: 44,
          target_shot_number: chain.recipe.segment_number,
          prompt_batch_size: chain.frameBatchSize,
          reference_image_ids: chain.recipe.reference_image_ids,
          video_library_image_ids: chain.recipe.video_library_image_ids,
        }),
      );
    });

    expect(mocks.startChapterVideoPrompts).toHaveBeenCalledTimes(4);
    chains.forEach((chain, index) => {
      expect(mocks.startChapterVideoPrompts).toHaveBeenNthCalledWith(
        index + 1,
        19,
        3,
        expect.objectContaining({
          workflow_mode: chain.recipe.workflow_mode,
          segment_plan_prompt_id: 44,
          target_shot_number: chain.recipe.segment_number,
          prompt_batch_size: 1,
          reference_image_ids: chain.recipe.reference_image_ids,
          video_library_image_ids: chain.recipe.video_library_image_ids,
        }),
      );
    });
  });
});
