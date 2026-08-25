import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
  IMAGE_TYPE_VIDEO_KEY_FRAME,
  type GeneratedImage,
} from "../../../api/image";
import {
  VIDEO_TYPE_CHAPTER,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type SegmentGuidanceMode,
  type VideoSegmentWorkflowMode,
} from "../../../api/video";
import { useVideoTabDerivedState } from "./useVideoTabDerivedState";
import { translateMessage, type MessageTranslator } from "../../../i18n";

const t: MessageTranslator = (key, params) =>
  translateMessage("zh-CN", key, params);

const executionIdentity = {
  contract_epoch: 5,
  shooting_script_identity: "script-1",
  projection_sha256: "projection-1",
  segment_plan_sha256: "plan-1",
  unit_plan_sha256: "unit-1",
  source_content_sha256: "source-1",
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
};

const plan: ChapterVideoPrompt = {
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
    source: { content_sha256: "source-1" },
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
    video_execution_identity: executionIdentity,
  },
  created_at: "2026-07-29T00:00:00Z",
  updated_at: "2026-07-29T00:00:00Z",
};

const video = (
  id: number,
  segmentNumber: number,
  workflowMode: VideoSegmentWorkflowMode,
  recipeFingerprint: string,
): GeneratedVideo => ({
  id,
  project_id: 1,
  chapter_id: 2,
  video_type: VIDEO_TYPE_CHAPTER,
  workflow_mode: workflowMode,
  file_path: `video-${id}.mp4`,
  thumbnail_path: null,
  prompt: `prompt-${id}`,
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
    shot_number: segmentNumber,
    video_execution_identity: {
      ...executionIdentity,
      source_shot_id: `shot-${segmentNumber}`,
      segment_index_in_shot: 1,
      segment_sha256: `segment-${segmentNumber}`,
      segment_plan_prompt_id: plan.id,
      segment_number: segmentNumber,
      segment_recipe_fingerprint: recipeFingerprint,
    },
  },
  is_favorite: false,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:01Z",
});

const videos = [
  video(101, 1, "chat_video", "recipe-chat"),
  video(102, 2, "storyboard", "recipe-storyboard"),
];

const statusVideos: GeneratedVideo[] = [
  video(111, 1, "chat_video", "recipe-chat"),
  {
    ...video(112, 1, "chat_video", "recipe-chat"),
    status: "running",
    file_path: null,
    provider_job_id: "job-112",
  },
  {
    ...video(113, 1, "chat_video", "recipe-chat"),
    status: "failed",
    file_path: null,
    error_message: "failed",
  },
];

const framePrompt: ChapterVideoPrompt = {
  ...plan,
  id: 42,
  prompt_type: "frame_prompt",
  workflow_mode: "boundary_frames",
  shot_number: 1,
  metadata_json: {
    contract_version: "boundary_frame_segment_local_v4",
    source_prompt_type: "frame_segment_plan",
    prompt_output_language: "zh_cn",
    video_execution_identity: {
      ...executionIdentity,
      source_shot_id: "shot-1",
      segment_index_in_shot: 1,
      segment_sha256: "segment-1",
      segment_plan_prompt_id: plan.id,
      segment_number: 1,
      segment_recipe_fingerprint: "frame-recipe",
    },
  },
};

const finalPrompt = (
  id: number,
  recipeFingerprint: string,
  referenceImageIds: number[],
): ChapterVideoPrompt => ({
  ...plan,
  id,
  prompt_type: "video_prompt",
  workflow_mode: "boundary_frames",
  shot_number: 1,
  reference_image_ids_json: referenceImageIds,
  metadata_json: {
    contract_version: "video_execution_contract_v5",
    source_prompt_type: "frame_segment_plan",
    prompt_output_language: "zh_cn",
    video_execution_identity: {
      ...executionIdentity,
      source_shot_id: "shot-1",
      segment_index_in_shot: 1,
      segment_sha256: "segment-1",
      segment_plan_prompt_id: plan.id,
      segment_number: 1,
      segment_recipe_fingerprint: recipeFingerprint,
      frame_set_sha256: "frames-1",
      prompt_output_language: "zh_cn",
      generation_asset_fingerprint: {},
    },
  },
});

const image = (
  id: number,
  imageType: string,
  metadata: unknown,
): GeneratedImage => ({
  id,
  project_id: 1,
  image_type: imageType,
  file_path: `image-${id}.png`,
  prompt: "image prompt",
  model: "test",
  width: 1280,
  height: 720,
  file_size: 1,
  ai_score: null,
  is_favorite: false,
  is_system_managed: true,
  chapter_id: 2,
  panel_id: null,
  metadata_json: metadata,
  created_at: "2026-01-01T00:00:00Z",
});

const systemFrameImage = image(201, IMAGE_TYPE_VIDEO_KEY_FRAME, {
  generation_context: {
    source: IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
    prompt_id: framePrompt.id,
  },
});
const keyframeImage = image(202, "reference", {
  source: { content_sha256: "source-1" },
});

const Probe = ({
  segmentNumber,
  workflowMode,
  recipeFingerprint,
  selectedVideoId,
}: {
  segmentNumber: number;
  workflowMode: VideoSegmentWorkflowMode;
  recipeFingerprint: string;
  selectedVideoId: number;
}) => {
  const state = useVideoTabDerivedState({
    prompts: [plan],
    videos,
    images: [],
    chapter: { id: 2, chapter_number: 1, title: "第一章" },
    selectedPromptId: null,
    selectedVideoId,
    workflowMode,
    promptOutputLanguage: "zh_cn",
    currentContentHash: "source-1",
    frameSegmentPlanPrompt: plan,
    activeVideoConfig: null,
    selectedSegmentNumber: segmentNumber,
    guidanceMode: "none",
    recipeFingerprint,
    frameRecipeFingerprint: null,
    continuitySourceVideoId: null,
    t,
  });
  return (
    <div
      data-video-ids={state.workflowVideos.map((item) => item.id).join(",")}
      data-selected-video-id={state.selectedVideo?.id ?? ""}
      data-selected-video-path={state.selectedVideoPath}
    />
  );
};

const ReferenceProbe = ({
  guidanceMode,
  recipeFingerprint,
  promptId,
}: {
  guidanceMode: SegmentGuidanceMode;
  recipeFingerprint: string;
  promptId: number;
}) => {
  const prompts = [
    plan,
    framePrompt,
    finalPrompt(51, "recipe-system", [systemFrameImage.id, keyframeImage.id]),
    finalPrompt(52, "recipe-keyframes", [
      systemFrameImage.id,
      keyframeImage.id,
    ]),
  ];
  const state = useVideoTabDerivedState({
    prompts,
    videos: [],
    images: [systemFrameImage, keyframeImage],
    chapter: { id: 2, chapter_number: 1, title: "第一章" },
    selectedPromptId: promptId,
    selectedVideoId: null,
    workflowMode: "boundary_frames",
    promptOutputLanguage: "zh_cn",
    currentContentHash: "source-1",
    frameSegmentPlanPrompt: plan,
    activeVideoConfig: null,
    selectedSegmentNumber: 1,
    guidanceMode,
    recipeFingerprint,
    frameRecipeFingerprint: "frame-recipe",
    continuitySourceVideoId: null,
    t,
  });
  return (
    <div
      data-valid-reference-ids={[...state.validPromptReferenceIds]
        .sort()
        .join(",")}
      data-stale-reference-count={state.staleSavedReferenceCount}
    />
  );
};

const StatusProbe = () => {
  const state = useVideoTabDerivedState({
    prompts: [plan],
    videos: statusVideos,
    images: [],
    chapter: { id: 2, chapter_number: 1, title: "第一章" },
    selectedPromptId: null,
    selectedVideoId: null,
    workflowMode: "chat_video",
    promptOutputLanguage: "zh_cn",
    currentContentHash: "source-1",
    frameSegmentPlanPrompt: plan,
    activeVideoConfig: null,
    selectedSegmentNumber: 1,
    guidanceMode: "none",
    recipeFingerprint: "recipe-chat",
    frameRecipeFingerprint: null,
    continuitySourceVideoId: null,
    t,
  });
  return (
    <div
      data-selected-video-status={state.selectedVideo?.status ?? ""}
      data-video-statuses={state.workflowVideos
        .map((item) => item.status)
        .join(",")}
    />
  );
};

describe("useVideoTabDerivedState", () => {
  it("switches preview and adoption candidates between per-segment workflow modes", () => {
    const chat = renderToStaticMarkup(
      <Probe
        segmentNumber={1}
        workflowMode="chat_video"
        recipeFingerprint="recipe-chat"
        selectedVideoId={102}
      />,
    );
    const storyboard = renderToStaticMarkup(
      <Probe
        segmentNumber={2}
        workflowMode="storyboard"
        recipeFingerprint="recipe-storyboard"
        selectedVideoId={101}
      />,
    );

    expect(chat).toContain('data-video-ids="101"');
    expect(chat).toContain('data-selected-video-id="101"');
    expect(chat).toContain('data-selected-video-path="video-101.mp4"');
    expect(storyboard).toContain('data-video-ids="102"');
    expect(storyboard).toContain('data-selected-video-id="102"');
    expect(storyboard).toContain('data-selected-video-path="video-102.mp4"');
  });

  it("keeps system frame references out of keyframe guidance state", () => {
    const systemFrames = renderToStaticMarkup(
      <ReferenceProbe
        guidanceMode="system_frames"
        recipeFingerprint="recipe-system"
        promptId={51}
      />,
    );
    const keyframes = renderToStaticMarkup(
      <ReferenceProbe
        guidanceMode="keyframes"
        recipeFingerprint="recipe-keyframes"
        promptId={52}
      />,
    );

    expect(systemFrames).toContain('data-valid-reference-ids="201,202"');
    expect(systemFrames).toContain('data-stale-reference-count="0"');
    expect(keyframes).toContain('data-valid-reference-ids="202"');
    expect(keyframes).toContain('data-stale-reference-count="1"');
  });

  it("keeps running and failed records in the current recipe result set", () => {
    const html = renderToStaticMarkup(<StatusProbe />);
    expect(html).toContain('data-selected-video-status="completed"');
    expect(html).toContain('data-video-statuses="completed,running,failed"');
  });
});
