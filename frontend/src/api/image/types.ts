export const IMAGE_TYPE_CANVAS = "canvas_image" as const;
export const IMAGE_TYPE_CANVAS_FRAME = "canvas_frame" as const;
export const IMAGE_TYPE_OBJECT_MATERIAL = "object_material" as const;
export const IMAGE_TYPE_SCENE_MATERIAL = "scene_material" as const;
export const IMAGE_TYPE_VIDEO_KEY_FRAME = "video_key_frame" as const;
export const IMAGE_TYPE_VIDEO_START_FRAME = "video_start_frame" as const;
export const IMAGE_TYPE_VIDEO_STORYBOARD_FRAME = "video_storyboard_frame" as const;
export const IMAGE_TYPE_DIRECTOR_STORYBOARD_FRAME = "director_storyboard_frame" as const;
export const IMAGE_TYPE_VIDEO_TAIL_FRAME = "video_tail_frame" as const;

export const VIDEO_FRAME_PROMPT_IMAGE_TYPES = [
  IMAGE_TYPE_VIDEO_KEY_FRAME,
  IMAGE_TYPE_VIDEO_START_FRAME,
  IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
] as const;

export const VIDEO_FRAME_IMAGE_TYPES = [
  IMAGE_TYPE_DIRECTOR_STORYBOARD_FRAME,
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
  ImageSize,
} from "../generated/config";

export type { ImageSize };
export { DEFAULT_IMAGE_SIZES } from "../generated/config";

export function imageSizeLabel(size: ImageSize): string {
  return `${size.width}x${size.height}`;
}

export function normalizeImageSizes(value: unknown, label: string): ImageSize[] {
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error(`${label} supported_sizes 必须至少包含一个尺寸`);
  }
  const seen = new Set<string>();
  return value.map((item, index) => {
    if (typeof item !== "object" || item === null) {
      throw new Error(`${label} supported_sizes[${index}] 无效`);
    }
    const candidate = item as { width?: unknown; height?: unknown };
    const width = candidate.width;
    const height = candidate.height;
    if (
      typeof width !== "number" ||
      typeof height !== "number" ||
      !Number.isSafeInteger(width) ||
      !Number.isSafeInteger(height) ||
      width <= 0 ||
      height <= 0 ||
      width > 2147483647 ||
      height > 2147483647
    ) {
      throw new Error(`${label} supported_sizes[${index}] 无效`);
    }
    const key = `${width}x${height}`;
    if (seen.has(key)) throw new Error(`${label} supported_sizes 包含重复尺寸 ${key}`);
    seen.add(key);
    return { width, height };
  });
}
