import {
  IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
  IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
  IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
  IMAGE_TYPE_VIDEO_KEY_FRAME,
  IMAGE_TYPE_VIDEO_START_FRAME,
  IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
  type ImageSize,
} from "../../../api/image";
import {
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type PromptOutputLanguage,
  type SegmentGuidanceMode,
  type VideoDurationMode,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentWorkflowMode,
} from "../../../api/video";
import { DEFAULT_VIDEO_STYLE } from "../styleTemplates";
import type { MessageTranslator } from "../../../i18n";

export interface PromptDraft {
  visual_prompt: string;
  motion_prompt: string;
  negative_prompt: string;
  duration_secs: number;
}

export const VIDEO_ASPECT_RATIOS = [
  "21:9",
  "16:9",
  "4:3",
  "1:1",
  "3:4",
  "9:16",
] as const;
export type VideoAspectRatio = (typeof VIDEO_ASPECT_RATIOS)[number];

export function videoAspectRatiosForImageSizes(
  sizes: readonly Pick<ImageSize, "width" | "height">[],
): VideoAspectRatio[] {
  const supported = new Set(
    sizes.flatMap(({ width, height }) => {
      if (width <= 0 || height <= 0) return [];
      let left = width;
      let right = height;
      while (right !== 0) {
        const remainder = left % right;
        left = right;
        right = remainder;
      }
      return [`${width / left}:${height / left}`];
    }),
  );
  return VIDEO_ASPECT_RATIOS.filter((ratio) => supported.has(ratio));
}

export interface VideoGenOptions {
  style: string;
  promptOutputLanguage: PromptOutputLanguage;
  durationSecs: number;
  durationOptions: number[];
  aspectRatio: VideoAspectRatio;
}

export interface VideoDurationLimits {
  model: string | null;
  durationMode: VideoDurationMode;
  defaultDurationSecs: number;
  minDurationSecs: number;
  maxDurationSecs: number;
  supportedDurations: number[];
  configError: string | null;
}

export const GENERIC_VIDEO_DURATION_LIMITS: VideoDurationLimits = {
  model: null,
  durationMode: "range",
  defaultDurationSecs: 6,
  minDurationSecs: 1,
  maxDurationSecs: 60,
  supportedDurations: [],
  configError: null,
};

export function videoGuidanceConfigurationError(
  workflowMode: VideoSegmentWorkflowMode,
  segmentNumber: number,
  guidanceMode: VideoSegmentExecutionRecipe["guidance_mode"],
  t: MessageTranslator,
): string | null {
  const requiredMode = guidanceModeForWorkflowChange(
    workflowMode,
    segmentNumber,
  );
  if (guidanceMode === requiredMode) return null;
  if (
    workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN &&
    segmentNumber > 1 &&
    guidanceMode === "system_frames"
  ) {
    return null;
  }
  if (workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES) {
    return t("video.chapter.boundaryFramesInputRequired");
  }
  if (workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD) {
    return t("video.chapter.storyboardInputRequired");
  }
  if (workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN) {
    return segmentNumber > 1
      ? t("video.chapter.firstFrameChainFollowupInputRequired")
      : t("video.chapter.firstFrameChainFirstInputRequired");
  }
  return t("video.chapter.mediaSlotInputRequired");
}

export function guidanceModeForWorkflowChange(
  workflowMode: VideoSegmentWorkflowMode,
  segmentNumber: number,
): SegmentGuidanceMode {
  if (
    workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO ||
    workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA
  ) {
    return "none";
  }
  return workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN &&
    segmentNumber > 1
    ? "continuity"
    : "system_frames";
}

export const DEFAULT_DRAFT: PromptDraft = {
  visual_prompt: "",
  motion_prompt: "",
  negative_prompt: "",
  duration_secs: 6,
};

export const MAX_REFERENCE_IMAGES = 4;
export const MAX_CHAT_REFERENCE_IMAGES = 16;
export const MAX_VIDEO_LIBRARY_IMAGES = 32;
export const PROMPT_OUTPUT_LANGUAGES: readonly PromptOutputLanguage[] = [
  "zh_cn",
  "en",
];
export const promptOutputLanguageLabel = (
  value: PromptOutputLanguage,
  t: MessageTranslator,
): string => (value === "en" ? t("settings.english") : t("settings.chinese"));
export const MAX_VIDEO_DURATION_OPTION_COUNT = 12;
export const VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE = "frame_segment_plan";
export const VIDEO_FRAME_PROMPT_TYPE = "frame_prompt";
export const VIDEO_FIRST_FRAME_PROMPT_TYPE = "first_frame_prompt";
export const VIDEO_CHAT_FRAME_PROMPT_TYPE = "chat_video_frame_prompt";
export const VIDEO_STORYBOARD_PROMPT_TYPE = "storyboard_prompt";
export const isVideoFramePromptType = (promptType: string): boolean =>
  promptType === VIDEO_FRAME_PROMPT_TYPE ||
  promptType === VIDEO_FIRST_FRAME_PROMPT_TYPE ||
  promptType === VIDEO_STORYBOARD_PROMPT_TYPE ||
  promptType === VIDEO_CHAT_FRAME_PROMPT_TYPE;
export const VIDEO_FINAL_PROMPT_TYPE = "video_prompt";
export const VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION = "video_execution_contract_v5";
export const VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION =
  "boundary_frame_segment_local_v4";
export const VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION =
  "first_frame_chain_execution_v4";
export const VIDEO_CHAT_VIDEO_CONTRACT_VERSION = "chat_video_execution_v4";
export const VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION =
  "storyboard_execution_unit_v2";
export const VIDEO_STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION =
  "storyboard_execution_video_v2";
export const VIDEO_EXECUTION_CONTRACT_EPOCH = 5;
export const VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS = 80;

export function videoFrameImageContractForPromptType(
  promptType: string,
  t: MessageTranslator,
) {
  switch (promptType) {
    case VIDEO_FRAME_PROMPT_TYPE:
      return {
        imageType: IMAGE_TYPE_VIDEO_KEY_FRAME,
        source: IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
        promptLabel: t("video.actions.boundaryPromptLabel"),
        frameLabel: t("video.chapter.frameBoundary"),
      } as const;
    case VIDEO_FIRST_FRAME_PROMPT_TYPE:
      return {
        imageType: IMAGE_TYPE_VIDEO_START_FRAME,
        source: IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
        promptLabel: t("video.actions.firstFramePromptLabel"),
        frameLabel: t("video.chapter.frameInputStart"),
      } as const;
    case VIDEO_STORYBOARD_PROMPT_TYPE:
      return {
        imageType: IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
        source: IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
        promptLabel: t("video.actions.storyboardPromptLabel"),
        frameLabel: t("video.chapter.storyboardInput"),
      } as const;
    default:
      throw new Error(t("video.actions.unsupportedFramePromptType", { type: promptType }));
  }
}

export const DEFAULT_VIDEO_OPTIONS: VideoGenOptions = {
  style: DEFAULT_VIDEO_STYLE,
  promptOutputLanguage: "zh_cn",
  durationSecs: 6,
  durationOptions: [6],
  aspectRatio: "16:9",
};

export const VIDEO_SEGMENT_EXECUTION_RECIPE_SCHEMA_VERSION =
  "video_segment_execution_recipe_v1" as const;

export const videoWorkflowModeLabel = (
  mode: VideoSegmentWorkflowMode,
  t: MessageTranslator,
): string => {
  const keys: Record<VideoSegmentWorkflowMode, Parameters<MessageTranslator>[0]> = {
    [VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES]: "video.chapter.workflowBoundaryFrames",
    [VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN]: "video.chapter.workflowFirstFrameChain",
    [VIDEO_WORKFLOW_MODE_STORYBOARD]: "video.chapter.workflowStoryboard",
    [VIDEO_WORKFLOW_MODE_CHAT_VIDEO]: "video.chapter.workflowChatVideo",
    [VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA]: "video.chapter.workflowReferenceMedia",
  };
  return t(keys[mode]);
};
