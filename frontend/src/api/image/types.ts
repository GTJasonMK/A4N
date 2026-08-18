export const IMAGE_TYPE_CANVAS = "canvas_image" as const;
export const IMAGE_TYPE_CANVAS_FRAME = "canvas_frame" as const;
export const IMAGE_TYPE_OBJECT_MATERIAL = "object_material" as const;
export const IMAGE_TYPE_SCENE_MATERIAL = "scene_material" as const;
export const IMAGE_TYPE_VIDEO_KEY_FRAME = "video_key_frame" as const;
export const IMAGE_TYPE_VIDEO_START_FRAME = "video_start_frame" as const;
export const IMAGE_TYPE_VIDEO_STORYBOARD_FRAME = "video_storyboard_frame" as const;
export const IMAGE_TYPE_VIDEO_TAIL_FRAME = "video_tail_frame" as const;

export const VIDEO_FRAME_PROMPT_IMAGE_TYPES = [
  IMAGE_TYPE_VIDEO_KEY_FRAME,
  IMAGE_TYPE_VIDEO_START_FRAME,
  IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
] as const;

export const VIDEO_FRAME_IMAGE_TYPES = [
  ...VIDEO_FRAME_PROMPT_IMAGE_TYPES,
  IMAGE_TYPE_VIDEO_TAIL_FRAME,
] as const;

export type VideoFramePromptImageType =
  (typeof VIDEO_FRAME_PROMPT_IMAGE_TYPES)[number];

export const IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT =
  "video_boundary_frame_prompt" as const;
export const IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT =
  "video_first_frame_prompt" as const;
export const IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT =
  "video_storyboard_prompt" as const;
export const IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE =
  "video_tail_frame_capture" as const;

export const VIDEO_FRAME_PROMPT_IMAGE_GENERATION_SOURCES = [
  IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
  IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
  IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
] as const;

export type VideoFramePromptImageGenerationSource =
  (typeof VIDEO_FRAME_PROMPT_IMAGE_GENERATION_SOURCES)[number];

export type { GeneratedImage } from "../generated/host";

/** Host 摘要不携带密钥；本机运行时设置额外保留本地密钥。 */
export type ImageGenConfig = ImageConfigSummary & {
  api_key_encrypted?: string | null;
};
export type ImageConfigInput = ApiImageConfigInput;
export type UpdateImageConfigInput = ApiUpdateImageConfigInput;
import type {
  ImageConfigInput as ApiImageConfigInput,
  ImageConfigSummary,
  UpdateImageConfigInput as ApiUpdateImageConfigInput,
} from "../generated/config";
