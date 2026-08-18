import { describe, expect, it } from "vitest";
import {
  VIDEO_TYPE_CHAPTER,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type VideoGenConfig,
  type VideoSegmentExecutionRecipe,
} from "../../../api/video";
import {
  assemblyPushIndex,
  assemblyPushSourceIds,
  assemblyPushStatus,
  completedSegmentPlanPreviewItems,
  currentFinalVideoPromptMatchesSegment,
  guidanceModeForWorkflowChange,
  isVideoFramePromptType,
  isSegmentPlanDraftCheckpointKey,
  latestVideoSegmentPlanPrompt,
  parseVideoOptions,
  readVideoSegmentRecipes,
  serializeVideoOptions,
  videoSegmentCardsFromPlanPrompt,
  videoSegmentCardsFromSegments,
  videoExecutionPromptsShareGenerationChain,
  videoExecutionSegmentIdentityMatchesVideo,
  videoGuidanceConfigurationError,
  videoPromptSourceFrameNumbers,
  videoDurationLimitsFromConfig,
  videoSegmentRecipeGenerationFingerprintJson,
} from "./videoTabUtils";

const videoConfigBase = {
  api_key_encrypted: null,
  backend: "openai-compatible-video",
  base_url: "https://video.example/v1",
  created_at: null,
  default_fps: 24,
  default_height: 720,
  default_width: 1280,
  has_api_key: false,
  id: 1,
  is_default: true,
  max_concurrent: 1,
  model: "video-model",
  name: "视频配置",
  updated_at: null,
};

it("maps validated range and fixed video duration capabilities", () => {
  const rangeConfig: VideoGenConfig = {
    ...videoConfigBase,
    default_duration_secs: 6,
    min_duration_secs: 1,
    max_duration_secs: 12,
    duration_mode: "range",
    supported_durations_json: null,
  };
  const fixedConfig: VideoGenConfig = {
    ...videoConfigBase,
    default_duration_secs: 10,
    min_duration_secs: 5,
    max_duration_secs: 15,
    duration_mode: "fixed",
    supported_durations_json: [5, 10, 15],
  };

  expect(videoDurationLimitsFromConfig(rangeConfig)).toEqual({
    model: "video-model",
    durationMode: "range",
    defaultDurationSecs: 6,
    minDurationSecs: 1,
    maxDurationSecs: 12,
    supportedDurations: [],
    configError: null,
  });
  expect(videoDurationLimitsFromConfig(fixedConfig)).toEqual({
    model: "video-model",
    durationMode: "fixed",
    defaultDurationSecs: 10,
    minDurationSecs: 5,
    maxDurationSecs: 15,
    supportedDurations: [5, 10, 15],
    configError: null,
  });
});

it("rejects damaged persisted planning options instead of repairing them", () => {
  const valid = {
    style: "cinematic",
    promptOutputLanguage: "zh_cn" as const,
    durationSecs: 6,
    durationOptions: [6, 10],
    aspectRatio: "16:9" as const,
  };
  const raw = JSON.stringify(valid);

  expect(parseVideoOptions(raw)).toEqual(valid);
  expect(serializeVideoOptions(valid)).toBe(raw);
  expect(() =>
    parseVideoOptions(
      JSON.stringify({
        style: valid.style,
        durationSecs: valid.durationSecs,
        durationOptions: valid.durationOptions,
        aspectRatio: valid.aspectRatio,
      }),
    ),
  ).toThrow("promptOutputLanguage");
  expect(() =>
    parseVideoOptions(
      JSON.stringify({
        style: valid.style,
        promptOutputLanguage: valid.promptOutputLanguage,
        durationSecs: valid.durationSecs,
        aspectRatio: valid.aspectRatio,
      }),
    ),
  ).toThrow("durationOptions");
  expect(() =>
    parseVideoOptions(JSON.stringify({ ...valid, durationOptions: [10, 6] })),
  ).toThrow("严格递增");
  expect(() =>
    parseVideoOptions(JSON.stringify({ ...valid, aspectRatio: "16 / 9" })),
  ).toThrow("aspectRatio");
  expect(() =>
    parseVideoOptions(JSON.stringify({ ...valid, legacyDuration: 6 })),
  ).toThrow("未知字段");
  expect(() =>
    serializeVideoOptions({ ...valid, durationOptions: [6, 6] }),
  ).toThrow("严格递增");
});

it("parses assembly push metadata without defaulting an invalid status", () => {
  const video = (assemblyPush: unknown) =>
    ({ metadata_json: { assembly_push: assemblyPush } }) as GeneratedVideo;

  expect(assemblyPushStatus(video({ status: "pending" }))).toBe("pending");
  expect(assemblyPushStatus(video({}))).toBeNull();
  expect(assemblyPushStatus(video({ status: "legacy" }))).toBeNull();
  expect(assemblyPushIndex(video({ push_index: 2 }))).toBe(2);
  expect(assemblyPushIndex(video({ push_index: 0 }))).toBeNull();
  expect(assemblyPushSourceIds(video({ source_video_ids: [3, 5] }))).toEqual([
    3, 5,
  ]);
  expect(
    assemblyPushSourceIds(video({ source_video_ids: [3, 0, "4", 5] })),
  ).toBeNull();
  expect(assemblyPushSourceIds(video({ source_video_ids: [3, 3] }))).toBeNull();
});

it("classifies every supported video frame prompt type", () => {
  expect(
    [
      "frame_prompt",
      "first_frame_prompt",
      "chat_video_frame_prompt",
      "storyboard_prompt",
    ].every(isVideoFramePromptType),
  ).toBe(true);
  expect(isVideoFramePromptType("video_prompt")).toBe(false);
});

describe("completedSegmentPlanPreviewItems", () => {
  it("ignores incomplete groups and lets a later checkpoint replace the same shot", () => {
    const partial =
      '{"segments":[{"source_shot_id":"shot-1","title":"第一段","scene_summary":"字符串里的 } 不结束对象"},{"source_shot_id":"shot-1"';
    expect(completedSegmentPlanPreviewItems(partial)).toEqual([]);

    const firstResult = `${JSON.stringify({
      shot_index: 1,
      segments: [
        { source_shot_id: "shot-1", segment_number: 1, title: "旧结果" },
      ],
    })}\n`;
    const secondShot = `${JSON.stringify({
      shot_index: 2,
      segments: [
        { source_shot_id: "shot-2", segment_number: 1, title: "第二镜头" },
      ],
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
    expect(
      isSegmentPlanDraftCheckpointKey("video_segment_plan.recovered"),
    ).toBe(true);
    expect(
      isSegmentPlanDraftCheckpointKey("video_segment_plan.shot.shot-2"),
    ).toBe(true);
    expect(isSegmentPlanDraftCheckpointKey("video_segment_plan.final")).toBe(
      false,
    );
    expect(isSegmentPlanDraftCheckpointKey("video_prompt_item_committed")).toBe(
      false,
    );
  });
});

describe("videoGuidanceConfigurationError", () => {
  it("rejects input contracts that do not match the visible workflow mode", () => {
    expect(videoGuidanceConfigurationError("boundary_frames", 1, "none")).toBe(
      "首尾帧方案固定使用首帧和尾帧输入",
    );
    expect(
      videoGuidanceConfigurationError("boundary_frames", 1, "system_frames"),
    ).toBeNull();
    expect(
      videoGuidanceConfigurationError("boundary_frames", 1, "keyframes"),
    ).toBe("首尾帧方案固定使用首帧和尾帧输入");
    expect(
      videoGuidanceConfigurationError("first_frame_chain", 2, "continuity"),
    ).toBeNull();
    expect(
      videoGuidanceConfigurationError("boundary_frames", 1, "continuity"),
    ).toBe("首尾帧方案固定使用首帧和尾帧输入");
    expect(
      videoGuidanceConfigurationError("first_frame_chain", 2, "system_frames"),
    ).toBeNull();
  });

  it("maps every visible workflow mode to one input contract", () => {
    expect(guidanceModeForWorkflowChange("storyboard", 1)).toBe(
      "system_frames",
    );
    expect(guidanceModeForWorkflowChange("boundary_frames", 1)).toBe(
      "system_frames",
    );
    expect(guidanceModeForWorkflowChange("first_frame_chain", 1)).toBe(
      "system_frames",
    );
    expect(guidanceModeForWorkflowChange("first_frame_chain", 2)).toBe(
      "continuity",
    );
    expect(guidanceModeForWorkflowChange("chat_video", 1)).toBe("none");
    expect(guidanceModeForWorkflowChange("reference_media", 1)).toBe("none");
  });
});

it("maps segment-local boundary prompts back to frame numbers 1 and 2", () => {
  const prompt = {
    metadata_json: {
      video_unit: { start_frame_number: 4, end_frame_number: 5 },
    },
  } as ChapterVideoPrompt;

  expect(videoPromptSourceFrameNumbers(prompt, "boundary_frames")).toEqual({
    startFrameNumber: 1,
    endFrameNumber: 2,
  });
  expect(videoPromptSourceFrameNumbers(prompt, "storyboard")).toEqual({
    startFrameNumber: 4,
    endFrameNumber: 5,
  });
  expect(
    videoPromptSourceFrameNumbers(prompt, "first_frame_chain", "system_frames"),
  ).toEqual({
    startFrameNumber: 1,
    endFrameNumber: null,
  });
});

describe("video execution generation chain", () => {
  const segmentIdentity = {
    source_shot_id: "shot-1",
    segment_index_in_shot: 1,
    segment_sha256: "segment-1",
  };
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
    segment_identities: [segmentIdentity],
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
    metadata_json: {
      segments: [
        {
          segment_number: 1,
          source_shot_id: "shot-1",
          segment_index_in_shot: 1,
        },
      ],
      video_execution_identity: executionIdentity,
    },
    created_at: "2026-07-29T00:00:00Z",
    updated_at: "2026-07-29T00:00:00Z",
  });

  it("requires plan, segment, and recipe identity instead of the old weak workflow match", () => {
    const plan = prompt(
      10,
      "frame_segment_plan",
      "shared_segment_plan",
      identity,
    );
    const segmentOne = prompt(11, "frame_prompt", "boundary_frames", {
      ...identity,
      ...segmentIdentity,
      segment_plan_prompt_id: 10,
      segment_number: 1,
      segment_recipe_fingerprint: "recipe-1",
    });
    const sameSegment = prompt(12, "video_prompt", "boundary_frames", {
      ...identity,
      ...segmentIdentity,
      segment_plan_prompt_id: 10,
      segment_number: 1,
      segment_recipe_fingerprint: "recipe-1",
    });
    const otherRecipe = prompt(13, "video_prompt", "boundary_frames", {
      ...identity,
      ...segmentIdentity,
      segment_plan_prompt_id: 10,
      segment_number: 1,
      segment_recipe_fingerprint: "recipe-2",
    });

    expect(videoExecutionPromptsShareGenerationChain(plan, segmentOne)).toBe(
      true,
    );
    expect(
      videoExecutionPromptsShareGenerationChain(segmentOne, sameSegment),
    ).toBe(true);
    expect(
      videoExecutionPromptsShareGenerationChain(sameSegment, otherRecipe),
    ).toBe(false);
  });

  it("shows only the current final-prompt contract for the selected segment recipe", () => {
    const plan = prompt(
      20,
      "frame_segment_plan",
      "shared_segment_plan",
      identity,
    );
    const finalIdentity = {
      ...identity,
      ...segmentIdentity,
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
    expect(
      matches(finalPrompt({ contract_version: "chat_video_execution_v3" })),
    ).toBe(false);
    expect(matches(finalPrompt({ source_prompt_type: "frame_prompt" }))).toBe(
      false,
    );
    expect(
      matches(
        finalPrompt(
          { prompt_output_language: "en" },
          { prompt_output_language: "en" },
        ),
      ),
    ).toBe(false);
    expect(
      matches({
        ...finalPrompt({}, { segment_plan_prompt_id: 19 }),
        generated_video_id: 99,
      }),
    ).toBe(false);
    expect(
      matches({
        ...finalPrompt({}, { segment_recipe_fingerprint: "recipe-stale" }),
        generated_video_id: 100,
      }),
    ).toBe(false);
  });

  it("keeps candidate videos scoped to the current plan and recipe", () => {
    const plan = prompt(
      30,
      "frame_segment_plan",
      "shared_segment_plan",
      identity,
    );
    const video = (planId: number): GeneratedVideo => ({
      id: planId,
      project_id: 1,
      chapter_id: 2,
      video_type: VIDEO_TYPE_CHAPTER,
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
      timeline_item_id: null,
      status: "completed",
      provider_job_id: null,
      error_message: null,
      metadata_json: {
        shot_number: 1,
        video_execution_identity: {
          ...identity,
          ...segmentIdentity,
          segment_plan_prompt_id: planId,
          segment_number: 1,
          segment_recipe_fingerprint: "recipe-1",
        },
      },
      is_favorite: false,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:01Z",
    });

    expect(
      videoExecutionSegmentIdentityMatchesVideo(video(30), plan, 1, "recipe-1"),
    ).toBe(true);
    expect(
      videoExecutionSegmentIdentityMatchesVideo(video(29), plan, 1, "recipe-1"),
    ).toBe(false);

    const retimed = video(30);
    retimed.duration_secs = 10;
    (
      retimed.metadata_json as {
        video_execution_identity: { duration_secs: number };
      }
    ).video_execution_identity.duration_secs = 10;
    expect(
      videoExecutionSegmentIdentityMatchesVideo(retimed, plan, 1, "recipe-1"),
    ).toBe(true);
  });

  it("keeps an existing segment valid when an earlier segment changes its display number", () => {
    const existingSegment = {
      source_shot_id: "shot-2",
      segment_index_in_shot: 1,
      segment_sha256: "stable-shot-2",
    };
    const expandedPlan: ChapterVideoPrompt = {
      ...prompt(40, "frame_segment_plan", "shared_segment_plan", {
        ...identity,
        projection_sha256: "expanded-projection",
        segment_plan_sha256: "expanded-plan",
        unit_plan_sha256: "expanded-units",
        segment_identities: [
          {
            source_shot_id: "shot-1",
            segment_index_in_shot: 1,
            segment_sha256: "new-shot-1",
          },
          existingSegment,
        ],
      }),
      metadata_json: {
        segments: [
          {
            segment_number: 1,
            source_shot_id: "shot-1",
            segment_index_in_shot: 1,
          },
          {
            segment_number: 2,
            source_shot_id: "shot-2",
            segment_index_in_shot: 1,
          },
        ],
        video_execution_identity: {
          ...identity,
          projection_sha256: "expanded-projection",
          segment_plan_sha256: "expanded-plan",
          unit_plan_sha256: "expanded-units",
          segment_identities: [
            {
              source_shot_id: "shot-1",
              segment_index_in_shot: 1,
              segment_sha256: "new-shot-1",
            },
            existingSegment,
          ],
        },
      },
    };
    const existingVideo: GeneratedVideo = {
      id: 401,
      project_id: 1,
      chapter_id: 2,
      video_type: VIDEO_TYPE_CHAPTER,
      workflow_mode: "chat_video",
      file_path: "existing.mp4",
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
      timeline_item_id: null,
      status: "completed",
      provider_job_id: null,
      error_message: null,
      metadata_json: {
        shot_number: 1,
        video_execution_identity: {
          ...identity,
          ...existingSegment,
          segment_plan_prompt_id: 40,
          segment_number: 1,
          segment_recipe_fingerprint: "recipe-2",
        },
      },
      is_favorite: false,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:01Z",
    };

    expect(
      videoExecutionSegmentIdentityMatchesVideo(
        existingVideo,
        expandedPlan,
        2,
        "recipe-2",
      ),
    ).toBe(true);
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
        segment_identities: [
          {
            source_shot_id: "shot-1",
            segment_index_in_shot: 1,
            segment_sha256: "segment-1",
          },
          {
            source_shot_id: "shot-2",
            segment_index_in_shot: 1,
            segment_sha256: "segment-2",
          },
        ],
      },
      segments: [
        {
          segment_number: 2,
          source_shot_id: "shot-2",
          segment_index_in_shot: 1,
          title: "片段二",
          scene_summary: "场景二",
          duration_secs: 6,
          aspect_ratio: "16:9",
        },
        {
          segment_number: 1,
          source_shot_id: "shot-1",
          segment_index_in_shot: 1,
          title: "片段一",
          scene_summary: "场景一",
          duration_secs: 6,
          aspect_ratio: "16:9",
        },
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
    created_at: "2026-07-29T00:00:00Z",
    updated_at: "2026-07-29T00:00:00Z",
  });

  it("builds cards from the shared plan before any final prompt exists", () => {
    const prompt = planPrompt();
    expect(
      videoSegmentCardsFromPlanPrompt(prompt).map((card) => card.segmentNumber),
    ).toEqual([1, 2]);

    const recipes = readVideoSegmentRecipes(prompt);
    expect(recipes.error).toBeNull();
    expect(recipes.recipes.get(1)?.workflow_mode).toBe("boundary_frames");
    expect(recipes.recipes.get(2)?.workflow_mode).toBe("storyboard");
  });

  it("builds an incremental card as soon as a validated draft segment arrives", () => {
    const draft = [
      JSON.stringify({
        shot_index: 1,
        segments: [
          {
            source_shot_id: "shot-1",
            segment_number: 1,
            title: "刚生成的片段",
            scene_summary: "第一个镜头场景",
            duration_secs: 6,
            aspect_ratio: "16:9",
          },
        ],
      }),
      JSON.stringify({
        shot_index: 2,
        segments: [
          {
            source_shot_id: "shot-2",
            segment_number: 1,
            title: "第二个镜头片段",
            scene_summary: "第二个镜头场景",
            duration_secs: 6,
            aspect_ratio: "16:9",
          },
        ],
      }),
    ].join("\n");

    expect(
      videoSegmentCardsFromSegments(completedSegmentPlanPreviewItems(draft)),
    ).toMatchObject([
      { segmentNumber: 1, title: "刚生成的片段" },
      { segmentNumber: 2, title: "第二个镜头片段" },
    ]);
  });

  it("rejects incomplete segment cards instead of inventing display fallbacks", () => {
    expect(() =>
      videoSegmentCardsFromSegments([
        {
          segment_number: 1,
          title: "片段一",
          duration_secs: 6,
          aspect_ratio: "16:9",
        },
      ]),
    ).toThrow("片段 1 scene_summary 无效");
  });

  it("does not expose a plan until the current chapter hash is known and matches", () => {
    const prompt = planPrompt();
    expect(latestVideoSegmentPlanPrompt([prompt], null)).toBeNull();
    expect(latestVideoSegmentPlanPrompt([prompt], "source-stale")).toBeNull();
    expect(latestVideoSegmentPlanPrompt([prompt], "source-current")?.id).toBe(
      41,
    );
  });

  it("does not expose a legacy plan without stable segment identities", () => {
    const prompt = planPrompt();
    const metadata = prompt.metadata_json as Record<string, unknown>;
    const identity = metadata.video_execution_identity as Record<
      string,
      unknown
    >;
    delete identity.segment_identities;
    expect(latestVideoSegmentPlanPrompt([prompt], "source-current")).toBeNull();
  });
});

describe("video segment recipe fingerprints", () => {
  it("canonicalizes reference-media binding fields independently of JSON key order", () => {
    const recipe: VideoSegmentExecutionRecipe = {
      schema_version: "video_segment_execution_recipe_v1",
      segment_number: 4,
      workflow_mode: "reference_media",
      style: "cinematic",
      prompt_output_language: "zh_cn",
      duration_secs: 5,
      duration_secs_options: [5],
      aspect_ratio: "16:9",
      reference_image_ids: [],
      video_library_image_ids: [],
      guidance_mode: "none",
      guidance_reference_image_ids: [],
      continuity_source_video_id: null,
      reference_media: {
        prompt: "reference media",
        image_rows: [
          {
            upload_mime_type: "image/jpeg",
            upload_file_name: "frame.jpg",
            upload_sha256: "sha256",
            usage: "lighting reference",
            url: "",
          },
        ],
        video_rows: [],
        audio_rows: [],
      },
      adopted_video_id: 159,
    };
    const canonicalOrderRecipe: VideoSegmentExecutionRecipe = {
      ...recipe,
      reference_media: {
        ...recipe.reference_media!,
        image_rows: [
          {
            url: "",
            usage: "lighting reference",
            upload_sha256: "sha256",
            upload_file_name: "frame.jpg",
            upload_mime_type: "image/jpeg",
          },
        ],
      },
    };

    expect(videoSegmentRecipeGenerationFingerprintJson(recipe)).toBe(
      videoSegmentRecipeGenerationFingerprintJson(canonicalOrderRecipe),
    );
    expect(
      videoSegmentRecipeGenerationFingerprintJson({
        ...recipe,
        segment_number: recipe.segment_number + 1,
      }),
    ).toBe(videoSegmentRecipeGenerationFingerprintJson(recipe));
  });
});
