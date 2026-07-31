import { describe, expect, it } from "vitest";
import type { WorkflowCheckpoint } from "../../../api/streaming";
import type { ChapterVideoPrompt, GeneratedVideo } from "../../../api/video";
import {
  completedSegmentPlanPreviewItems,
  currentFinalVideoPromptMatchesSegment,
  guidanceModeForWorkflowChange,
  isSegmentPlanDraftCheckpointKey,
  latestVideoSegmentPlanPrompt,
  readVideoSegmentRecipes,
  segmentPlanDraftSnapshotFromCheckpoints,
  videoSegmentCardsFromPlanPrompt,
  videoSegmentCardsFromSegments,
  videoExecutionPromptsShareGenerationChain,
  videoExecutionSegmentIdentityMatchesVideo,
  videoGuidanceConfigurationError,
} from "./videoTabUtils";

const segmentPlanCheckpoint = (
  shotIndex: number,
  totalShots: number,
  segments: Record<string, unknown>[],
  status = "completed",
): WorkflowCheckpoint => {
  const canonicalSegments = segments.map((segment, index) => ({
    segment_number: index + 1,
    duration_secs: 6,
    aspect_ratio: "16:9",
    ...segment,
  }));
  return {
  id: shotIndex,
  workflow_run_id: 9,
  step_key: `video_segment_plan.shot.shot-${shotIndex}`,
  step_order: 199 + shotIndex,
  status,
  input_fingerprint_json: {
    extra: { shot_index: shotIndex, total_shots: totalShots },
  },
  checkpoint_json: {
    parsed_output: canonicalSegments,
  },
  artifacts_json: {
    segment_recipes: canonicalSegments.map((segment) => ({
      schema_version: "video_segment_execution_recipe_v1",
      segment_number: segment.segment_number,
      workflow_mode: "chat_video",
    })),
  },
  generation_run_ids_json: null,
  error_message: null,
  created_at: "2026-07-29T00:00:00Z",
  updated_at: "2026-07-29T00:00:00Z",
  completed_at: status === "completed" ? "2026-07-29T00:00:00Z" : null,
  };
};

describe("completedSegmentPlanPreviewItems", () => {
  it("ignores incomplete groups and lets a later checkpoint replace the same shot", () => {
    const partial = '{"segments":[{"source_shot_id":"shot-1","title":"第一段","scene_summary":"字符串里的 } 不结束对象"},{"source_shot_id":"shot-1"';
    expect(completedSegmentPlanPreviewItems(partial)).toEqual([]);

    const firstResult = `${JSON.stringify({
      shot_index: 1,
      segments: [{ source_shot_id: "shot-1", segment_number: 1, title: "旧结果" }],
    })}\n`;
    const secondShot = `${JSON.stringify({
      shot_index: 2,
      segments: [{ source_shot_id: "shot-2", segment_number: 1, title: "第二镜头" }],
    })}\n`;
    const repairedFirstShot = `${JSON.stringify({
      shot_index: 1,
      segments: [
        { source_shot_id: "shot-1", segment_number: 1, title: "第一镜头 A" },
        { source_shot_id: "shot-1", segment_number: 2, title: "第一镜头 B" },
      ],
    })}\n`;

    expect(
      completedSegmentPlanPreviewItems(
        `${secondShot}${firstResult}${repairedFirstShot}`,
      ).map((item) => item.title),
    ).toEqual(["第一镜头 A", "第一镜头 B", "第二镜头"]);
  });

  it("recognizes only segment-plan draft checkpoints", () => {
    expect(isSegmentPlanDraftCheckpointKey("video_segment_plan.recovered")).toBe(true);
    expect(isSegmentPlanDraftCheckpointKey("video_segment_plan.shot.shot-2")).toBe(true);
    expect(
      isSegmentPlanDraftCheckpointKey(
        "video_segment_plan.shot.shot-2:schema_repair",
      ),
    ).toBe(false);
    expect(isSegmentPlanDraftCheckpointKey("video_segment_plan.final")).toBe(false);
    expect(isSegmentPlanDraftCheckpointKey("video_prompt_item_committed")).toBe(false);
  });
});

describe("segmentPlanDraftSnapshotFromCheckpoints", () => {
  it("loads completed shots in shot order and rebuilds continuous display numbers", () => {
    const snapshot = segmentPlanDraftSnapshotFromCheckpoints([
      segmentPlanCheckpoint(3, 3, [
        { source_shot_id: "shot-3", title: "第三镜头" },
      ]),
      segmentPlanCheckpoint(2, 3, [
        { source_shot_id: "shot-2", title: "失败结果" },
      ], "failed"),
      segmentPlanCheckpoint(1, 3, [
        { source_shot_id: "shot-1", title: "第一镜头 A" },
        { source_shot_id: "shot-1", title: "第一镜头 B" },
      ]),
      segmentPlanCheckpoint(2, 3, [
        { source_shot_id: "shot-2", title: "第二镜头" },
      ]),
    ]);

    expect(snapshot?.segments.map((item) => item.title)).toEqual([
      "第一镜头 A",
      "第一镜头 B",
      "第二镜头",
      "第三镜头",
    ]);
    expect(snapshot?.segments.map((item) => item.segment_number)).toEqual([1, 2, 3, 4]);
    expect(snapshot?.segments.map((item) => item.segment_index_in_shot)).toEqual([1, 2, 1, 1]);
    expect(snapshot?.segments.map((item) => item.shot_segment_count)).toEqual([2, 2, 1, 1]);
    expect(snapshot).toMatchObject({
      completedShotCount: 3,
      totalShotCount: 3,
    });
    expect(videoSegmentCardsFromSegments(snapshot?.segments ?? [])).toHaveLength(4);
  });

  it("keeps each draft recipe on its stable shot segment when display numbers are rebuilt", () => {
    const laterShot = segmentPlanCheckpoint(2, 2, [
      { source_shot_id: "shot-2", title: "第二镜头" },
    ]);
    laterShot.artifacts_json = {
      segment_recipes: [{
        schema_version: "video_segment_execution_recipe_v1",
        segment_number: 1,
        workflow_mode: "storyboard",
      }],
    };

    const snapshot = segmentPlanDraftSnapshotFromCheckpoints([
      laterShot,
      segmentPlanCheckpoint(1, 2, [
        { source_shot_id: "shot-1", title: "第一镜头" },
      ]),
    ]);

    expect(snapshot?.recipes.get(1)?.workflow_mode).toBe("chat_video");
    expect(snapshot?.recipes.get(2)?.workflow_mode).toBe("storyboard");
  });

  it("rejects a persisted shot without recipes instead of inventing defaults", () => {
    const checkpoint = segmentPlanCheckpoint(1, 1, [
      { source_shot_id: "shot-1", title: "第一镜头" },
    ]);
    checkpoint.artifacts_json = null;

    expect(() => segmentPlanDraftSnapshotFromCheckpoints([checkpoint])).toThrow(
      "缺少 segment_recipes",
    );
  });

  it("restores completed shots without inventing a second progress percentage", () => {
    const checkpoints = Array.from({ length: 95 }, (_, index) =>
      segmentPlanCheckpoint(index + 1, 100, [
        { source_shot_id: `shot-${index + 1}`, title: `镜头 ${index + 1}` },
      ]),
    );

    expect(segmentPlanDraftSnapshotFromCheckpoints(checkpoints)).toMatchObject({
      completedShotCount: 95,
      totalShotCount: 100,
    });
    expect(segmentPlanDraftSnapshotFromCheckpoints(checkpoints)).not.toHaveProperty("progressCurrent");
  });

  it("ignores internal schema repair checkpoints and restores the final shot result", () => {
    const finalShot = segmentPlanCheckpoint(1, 2, [
      { source_shot_id: "unit_2_shot_1", title: "已校验片段" },
    ]);
    const repairCheckpoint: WorkflowCheckpoint = {
      ...finalShot,
      id: 99,
      step_key: `${finalShot.step_key}:schema_repair`,
      checkpoint_json: {
        parsed_output: {
          segments: [{ source_shot_id: "unit_2_shot_1", title: "结构修复输出" }],
        },
      },
    };

    expect(
      segmentPlanDraftSnapshotFromCheckpoints([
        repairCheckpoint,
        finalShot,
      ]),
    ).toMatchObject({
      completedShotCount: 1,
      totalShotCount: 2,
      segments: [{ title: "已校验片段" }],
    });
  });
});

describe("videoGuidanceConfigurationError", () => {
  it("checks workflow structure without guessing provider capabilities", () => {
    expect(videoGuidanceConfigurationError("boundary_frames", 1, "none")).toBeNull();
    expect(videoGuidanceConfigurationError("boundary_frames", 1, "system_frames")).toBeNull();
    expect(videoGuidanceConfigurationError("boundary_frames", 1, "keyframes")).toBeNull();
    expect(videoGuidanceConfigurationError("first_frame_chain", 2, "continuity")).toBeNull();
    expect(videoGuidanceConfigurationError("boundary_frames", 1, "continuity")).toBe(
      "续接引导只适用于首帧续接的后续片段",
    );
    expect(videoGuidanceConfigurationError("first_frame_chain", 2, "system_frames")).toBe(
      "首帧续接的后续片段不能使用系统首帧",
    );
  });

  it("preserves structurally valid guidance and falls back without a capability gate", () => {
    expect(guidanceModeForWorkflowChange("storyboard", 1, "keyframes", false)).toBe("keyframes");
    expect(guidanceModeForWorkflowChange("boundary_frames", 1, "system_frames", false)).toBe(
      "system_frames",
    );
    expect(
      guidanceModeForWorkflowChange("first_frame_chain", 2, "system_frames", true),
    ).toBe("continuity");
    expect(guidanceModeForWorkflowChange("first_frame_chain", 2, "system_frames", false)).toBe(
      "none",
    );
    expect(guidanceModeForWorkflowChange("chat_video", 1, "keyframes", false)).toBe("none");
  });
});

describe("video execution generation chain", () => {
  const identity = {
    contract_epoch: 5,
    shooting_script_identity: "script-1",
    projection_sha256: "projection-1",
    segment_plan_sha256: "plan-1",
    unit_plan_sha256: "unit-1",
    source_content_sha256: "source-1",
    style_sha256: "style-1",
    duration_secs: 6,
    aspect_ratio: "16:9",
  };
  const prompt = (
    id: number,
    promptType: string,
    workflowMode: ChapterVideoPrompt["workflow_mode"],
    executionIdentity: Record<string, unknown>,
  ): ChapterVideoPrompt => ({
    id,
    project_id: 1,
    chapter_id: 2,
    style: "cinematic",
    prompt_type: promptType,
    workflow_mode: workflowMode,
    shot_number: 1,
    title: null,
    scene_summary: null,
    visual_prompt: "prompt",
    motion_prompt: null,
    negative_prompt: null,
    duration_secs: 6,
    aspect_ratio: "16:9",
    reference_image_ids_json: null,
    status: "completed",
    generated_video_id: null,
    metadata_json: { video_execution_identity: executionIdentity },
    created_at: null,
    updated_at: null,
  });

  it("requires plan, segment, and recipe identity instead of the old weak workflow match", () => {
    const plan = prompt(10, "frame_segment_plan", "shared_segment_plan", identity);
    const segmentOne = prompt(11, "frame_prompt", "boundary_frames", {
      ...identity,
      segment_plan_prompt_id: 10,
      segment_number: 1,
      segment_recipe_fingerprint: "recipe-1",
    });
    const sameSegment = prompt(12, "video_prompt", "boundary_frames", {
      ...identity,
      segment_plan_prompt_id: 10,
      segment_number: 1,
      segment_recipe_fingerprint: "recipe-1",
    });
    const otherRecipe = prompt(13, "video_prompt", "boundary_frames", {
      ...identity,
      segment_plan_prompt_id: 10,
      segment_number: 1,
      segment_recipe_fingerprint: "recipe-2",
    });

    expect(videoExecutionPromptsShareGenerationChain(plan, segmentOne)).toBe(true);
    expect(videoExecutionPromptsShareGenerationChain(segmentOne, sameSegment)).toBe(true);
    expect(videoExecutionPromptsShareGenerationChain(segmentOne, otherRecipe)).toBe(false);
  });

  it("shows only the current final-prompt contract for the selected segment recipe", () => {
    const plan = prompt(20, "frame_segment_plan", "shared_segment_plan", identity);
    const finalIdentity = {
      ...identity,
      segment_plan_prompt_id: 20,
      segment_number: 1,
      segment_recipe_fingerprint: "recipe-1",
      frame_set_sha256: "frames-1",
      prompt_output_language: "zh_cn",
      generation_asset_fingerprint: {},
    };
    const finalPrompt = (
      metadataOverrides: Record<string, unknown> = {},
      identityOverrides: Record<string, unknown> = {},
    ): ChapterVideoPrompt => ({
      ...prompt(21, "video_prompt", "chat_video", finalIdentity),
      metadata_json: {
        contract_version: "chat_video_execution_v4",
        source_prompt_type: "frame_segment_plan",
        prompt_output_language: "zh_cn",
        ...metadataOverrides,
        video_execution_identity: {
          ...finalIdentity,
          ...identityOverrides,
        },
      },
    });
    const matches = (candidate: ChapterVideoPrompt) =>
      currentFinalVideoPromptMatchesSegment(
        candidate,
        plan,
        1,
        "recipe-1",
        "chat_video",
        "zh_cn",
      );

    expect(matches(finalPrompt())).toBe(true);
    expect(matches(finalPrompt({ contract_version: "chat_video_execution_v3" }))).toBe(false);
    expect(matches(finalPrompt({ source_prompt_type: "frame_prompt" }))).toBe(false);
    expect(
      matches(
        finalPrompt(
          { prompt_output_language: "en" },
          { prompt_output_language: "en" },
        ),
      ),
    ).toBe(false);
    expect(matches({
      ...finalPrompt({}, { segment_plan_prompt_id: 19 }),
      generated_video_id: 99,
    })).toBe(false);
    expect(matches({
      ...finalPrompt({}, { segment_recipe_fingerprint: "recipe-stale" }),
      generated_video_id: 100,
    })).toBe(false);
  });

  it("keeps candidate videos scoped to the current plan and recipe", () => {
    const plan = prompt(30, "frame_segment_plan", "shared_segment_plan", identity);
    const video = (planId: number): GeneratedVideo => ({
      id: planId,
      project_id: 1,
      chapter_id: 2,
      video_type: "chapter_video",
      workflow_mode: "chat_video",
      file_path: "video.mp4",
      thumbnail_path: null,
      prompt: "prompt",
      negative_prompt: null,
      model: "test",
      width: 1280,
      height: 720,
      duration_secs: 6,
      fps: 24,
      file_size: 1,
      source_video_id: null,
      source_image_id: null,
      status: "completed",
      provider_job_id: null,
      error_message: null,
      metadata_json: {
        shot_number: 1,
        video_execution_identity: {
          ...identity,
          segment_plan_prompt_id: planId,
          segment_number: 1,
          segment_recipe_fingerprint: "recipe-1",
        },
      },
      is_favorite: false,
      created_at: null,
      updated_at: null,
    });

    expect(videoExecutionSegmentIdentityMatchesVideo(video(30), plan, 1, "recipe-1")).toBe(true);
    expect(videoExecutionSegmentIdentityMatchesVideo(video(29), plan, 1, "recipe-1")).toBe(false);
  });
});

describe("segment cards and recipes", () => {
  const planPrompt = (): ChapterVideoPrompt => ({
    id: 41,
    project_id: 1,
    chapter_id: 2,
    style: "cinematic",
    prompt_type: "frame_segment_plan",
    workflow_mode: "shared_segment_plan",
    shot_number: 0,
    title: null,
    scene_summary: null,
    visual_prompt: "plan",
    motion_prompt: null,
    negative_prompt: null,
    duration_secs: 6,
    aspect_ratio: "16:9",
    reference_image_ids_json: null,
    status: "completed",
    generated_video_id: null,
    metadata_json: {
      contract_version: "video_execution_contract_v5",
      source: { content_sha256: "source-current" },
      video_execution_identity: {
        contract_epoch: 5,
        shooting_script_identity: "script-1",
        projection_sha256: "projection-1",
        segment_plan_sha256: "plan-1",
        unit_plan_sha256: "unit-1",
        source_content_sha256: "source-current",
        style_sha256: "style-1",
        duration_secs: 6,
        aspect_ratio: "16:9",
      },
      segments: [
        { segment_number: 2, title: "片段二", duration_secs: 6, aspect_ratio: "16:9" },
        { segment_number: 1, title: "片段一", duration_secs: 6, aspect_ratio: "16:9" },
      ],
      segment_recipes: [
        {
          schema_version: "video_segment_execution_recipe_v1",
          segment_number: 1,
          workflow_mode: "boundary_frames",
          style: "cinematic",
          prompt_output_language: "zh_cn",
          duration_secs: 6,
          duration_secs_options: [6],
          aspect_ratio: "16:9",
          reference_image_ids: [],
          video_library_image_ids: [],
          guidance_mode: "system_frames",
          guidance_reference_image_ids: [],
          continuity_source_video_id: null,
          reference_media: null,
          adopted_video_id: null,
        },
        {
          schema_version: "video_segment_execution_recipe_v1",
          segment_number: 2,
          workflow_mode: "storyboard",
          style: "cinematic",
          prompt_output_language: "zh_cn",
          duration_secs: 6,
          duration_secs_options: [6],
          aspect_ratio: "16:9",
          reference_image_ids: [],
          video_library_image_ids: [],
          guidance_mode: "system_frames",
          guidance_reference_image_ids: [],
          continuity_source_video_id: null,
          reference_media: null,
          adopted_video_id: null,
        },
      ],
    },
    created_at: null,
    updated_at: null,
  });

  it("builds cards from the shared plan before any final prompt exists", () => {
    const prompt = planPrompt();
    expect(videoSegmentCardsFromPlanPrompt(prompt).map((card) => card.segmentNumber)).toEqual([1, 2]);

    const recipes = readVideoSegmentRecipes(prompt);
    expect(recipes.error).toBeNull();
    expect(recipes.recipes.get(1)?.workflow_mode).toBe("boundary_frames");
    expect(recipes.recipes.get(2)?.workflow_mode).toBe("storyboard");
  });

  it("builds an incremental card as soon as a validated draft segment arrives", () => {
    const draft = [
      JSON.stringify({
        shot_index: 1,
        segments: [{
          source_shot_id: "shot-1",
          segment_number: 1,
          title: "刚生成的片段",
          duration_secs: 6,
          aspect_ratio: "16:9",
        }],
      }),
      JSON.stringify({
        shot_index: 2,
        segments: [{
          source_shot_id: "shot-2",
          segment_number: 1,
          title: "第二个镜头片段",
          duration_secs: 6,
          aspect_ratio: "16:9",
        }],
      }),
    ].join("\n");

    expect(
      videoSegmentCardsFromSegments(completedSegmentPlanPreviewItems(draft)),
    ).toMatchObject([
      { segmentNumber: 1, title: "刚生成的片段" },
      { segmentNumber: 2, title: "第二个镜头片段" },
    ]);
  });

  it("does not expose a plan until the current chapter hash is known and matches", () => {
    const prompt = planPrompt();
    expect(latestVideoSegmentPlanPrompt([prompt], null)).toBeNull();
    expect(latestVideoSegmentPlanPrompt([prompt], "source-stale")).toBeNull();
    expect(latestVideoSegmentPlanPrompt([prompt], "source-current")?.id).toBe(41);
  });
});
