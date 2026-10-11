import {
  IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE,
  IMAGE_TYPE_OBJECT_MATERIAL,
  IMAGE_TYPE_SCENE_MATERIAL,
  IMAGE_TYPE_VIDEO_TAIL_FRAME,
  VIDEO_FRAME_IMAGE_TYPES,
  VIDEO_FRAME_PROMPT_IMAGE_TYPES,
  VIDEO_FRAME_PROMPT_IMAGE_GENERATION_SOURCES,
  type GeneratedImage,
  type VideoFramePromptImageGenerationSource,
} from "../../../api/image";
import {
  VIDEO_SEGMENT_WORKFLOW_MODES,
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type ChapterVideoPrompt,
  type PromptOutputLanguage,
  type VideoSegmentWorkflowMode,
} from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { isPositiveSafeInteger } from "../../../utils/numberRange";
import type { ChapterRow } from "../shared";
import { formatDateTime, formatNumber, type Locale, type MessageTranslator } from "../../../i18n";
import {
  GENERIC_VIDEO_DURATION_LIMITS,
  PROMPT_OUTPUT_LANGUAGES,
  type VideoDurationLimits,
} from "./videoTabConstants";

export const CHAT_VIDEO_DURATION_LIMITS: VideoDurationLimits = {
  ...GENERIC_VIDEO_DURATION_LIMITS,
};

const VIDEO_FRAME_IMAGE_TYPE_SET = new Set<string>(VIDEO_FRAME_IMAGE_TYPES);
const VIDEO_FRAME_PROMPT_IMAGE_TYPE_SET = new Set<string>(
  VIDEO_FRAME_PROMPT_IMAGE_TYPES,
);

const VIDEO_FRAME_PROMPT_IMAGE_GENERATION_SOURCE_SET = new Set<string>(
  VIDEO_FRAME_PROMPT_IMAGE_GENERATION_SOURCES,
);

export const isVideoFramePromptImageGenerationSource = (
  source: string | null | undefined,
): source is VideoFramePromptImageGenerationSource =>
  typeof source === "string" &&
  VIDEO_FRAME_PROMPT_IMAGE_GENERATION_SOURCE_SET.has(source);

const videoImageGenerationContext = (
  img: GeneratedImage,
): Record<string, unknown> | null =>
  asRecord(asRecord(img.metadata_json)?.generation_context);

export const videoImageGenerationPromptId = (
  img: GeneratedImage,
): number | null => {
  const raw = videoImageGenerationContext(img)?.prompt_id;
  return isPositiveSafeInteger(raw)
    ? raw
    : null;
};

const videoImageGenerationShotNumber = (img: GeneratedImage): number | null => {
  const raw = videoImageGenerationContext(img)?.shot_number;
  return isPositiveSafeInteger(raw)
    ? raw
    : null;
};

export const videoImageGenerationSource = (
  img: GeneratedImage,
): string | null => {
  const raw = videoImageGenerationContext(img)?.source;
  return typeof raw === "string" && raw.length > 0 && raw === raw.trim()
    ? raw
    : null;
};

export const videoImageGenerationSourceHash = (img: GeneratedImage): string | null => {
  const raw = videoImageGenerationContext(img)?.source_content_sha256;
  return typeof raw === "string" && raw.length > 0 && raw === raw.trim()
    ? raw
    : null;
};

export const videoTailFrameSourceVideoId = (
  img: GeneratedImage,
): number | null => {
  if (
    img.image_type !== IMAGE_TYPE_VIDEO_TAIL_FRAME ||
    videoImageGenerationSource(img) !==
    IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE
  )
    return null;
  const raw = videoImageGenerationContext(img)?.video_id;
  return isPositiveSafeInteger(raw)
    ? raw
    : null;
};

export function videoTailFrameBoundaryNumber(
  img: GeneratedImage,
): number | null {
  if (videoTailFrameSourceVideoId(img) == null) return null;
  const shotNumber = videoImageGenerationShotNumber(img);
  return shotNumber == null ? null : shotNumber + 1;
}

export function isVideoPromptFrameImage(
  img: GeneratedImage,
  promptIds?: Set<number>,
): boolean {
  const promptId = videoImageGenerationPromptId(img);
  if (promptIds && (promptId == null || !promptIds.has(promptId))) return false;
  const source = videoImageGenerationSource(img);
  return (
    VIDEO_FRAME_PROMPT_IMAGE_TYPE_SET.has(img.image_type) &&
    isVideoFramePromptImageGenerationSource(source)
  );
}

export interface VideoFrameImageIndexes {
  boundaryByFrameNumber: Map<number, GeneratedImage>;
  frameImageByPromptId: Map<number, GeneratedImage>;
  tailByVideoId: Map<number, GeneratedImage>;
}

export function indexVideoFrameImages(
  framePrompts: ReadonlyArray<Pick<ChapterVideoPrompt, "id" | "shot_number">>,
  frameImages: ReadonlyArray<GeneratedImage>,
): VideoFrameImageIndexes {
  const frameNumberByPromptId = new Map(
    framePrompts.map((prompt) => [prompt.id, prompt.shot_number]),
  );
  const boundaryByFrameNumber = new Map<number, GeneratedImage>();
  const frameImageByPromptId = new Map<number, GeneratedImage>();
  const tailByVideoId = new Map<number, GeneratedImage>();
  const keepLatest = (
    map: Map<number, GeneratedImage>,
    image: GeneratedImage,
    key: number,
  ) => {
    const current = map.get(key);
    if (current == null || image.id > current.id) map.set(key, image);
  };

  for (const image of frameImages) {
    const promptId = videoImageGenerationPromptId(image);
    const promptFrameImage =
      promptId != null && isVideoPromptFrameImage(image);
    if (promptFrameImage) {
      keepLatest(frameImageByPromptId, image, promptId);
      const frameNumber = frameNumberByPromptId.get(promptId);
      if (frameNumber != null)
        keepLatest(boundaryByFrameNumber, image, frameNumber);
    } else {
      const tailBoundaryNumber = videoTailFrameBoundaryNumber(image);
      if (tailBoundaryNumber != null) {
        keepLatest(boundaryByFrameNumber, image, tailBoundaryNumber);
      }
    }
    const sourceVideoId = videoTailFrameSourceVideoId(image);
    if (sourceVideoId != null) keepLatest(tailByVideoId, image, sourceVideoId);
  }
  return { boundaryByFrameNumber, frameImageByPromptId, tailByVideoId };
}

export const formatVideoCreatedAt = (
  value: string | null | undefined,
  locale: Locale,
  t: MessageTranslator,
): string => {
  return formatDateTime(value, locale, t("video.chapter.unknownTime"));
};

export const formatVideoDuration = (
  value: number,
  locale: Locale,
  t: MessageTranslator,
): string => {
  if (!Number.isFinite(value) || value <= 0) {
    return t("video.postprocess.seconds", { count: formatNumber(0, locale) });
  }
  const secondsTotal = Math.round(value);
  if (secondsTotal < 60) {
    return t("video.postprocess.seconds", {
      count: formatNumber(secondsTotal, locale),
    });
  }
  const minutes = Math.floor(secondsTotal / 60);
  const seconds = secondsTotal % 60;
  return t("video.postprocess.minutesSeconds", {
    minutes: formatNumber(minutes, locale),
    seconds: formatNumber(seconds, locale),
  });
};

export function isVideoFrameInputImage(img: GeneratedImage): boolean {
  return VIDEO_FRAME_IMAGE_TYPE_SET.has(img.image_type);
}

export function isChapterVideoMaterialImage(
  img: GeneratedImage,
  chapter: ChapterRow | null,
): boolean {
  if (!chapter || img.chapter_id !== chapter.id) return false;
  const metadata = asRecord(img.metadata_json);
  return metadata?.chapter_video_material === true;
}

export function isProjectReusableVideoMaterialImage(
  img: GeneratedImage,
): boolean {
  return profileMaterialKindOfImage(img) != null;
}

export function profileMaterialKindOfImage(
  img: GeneratedImage,
): "scene" | "object" | null {
  const type = img.image_type.trim();
  const metadata = asRecord(img.metadata_json);
  const explicitKind =
    typeof metadata?.profile_kind === "string" && metadata.profile_kind.trim()
      ? metadata.profile_kind.trim()
      : null;
  if (type === IMAGE_TYPE_SCENE_MATERIAL) {
    if (explicitKind != null && explicitKind !== "scene") return null;
    const id =
      explicitKind === "scene" ? metadataNumber(metadata?.profile_id) : null;
    return id == null ? null : "scene";
  }
  if (type === IMAGE_TYPE_OBJECT_MATERIAL) {
    if (explicitKind != null && explicitKind !== "object") return null;
    const id =
      explicitKind === "object" ? metadataNumber(metadata?.profile_id) : null;
    return id == null ? null : "object";
  }
  return null;
}

const metadataNumber = (value: unknown): number | null => {
  if (isPositiveSafeInteger(value)) return value;
  if (typeof value === "string") {
    const text = value.trim();
    if (!/^\+?\d+$/.test(text)) return null;
    const parsed = Number(text);
    return isPositiveSafeInteger(parsed) ? parsed : null;
  }
  return null;
};

export function promptOutputLanguageOfPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
): PromptOutputLanguage | null {
  const raw = asRecord(prompt?.metadata_json)?.prompt_output_language;
  return typeof raw === "string" &&
    PROMPT_OUTPUT_LANGUAGES.includes(raw as PromptOutputLanguage)
    ? (raw as PromptOutputLanguage)
    : null;
}

export function promptSourcePromptType(
  prompt: ChapterVideoPrompt | null | undefined,
): string | null {
  const raw = asRecord(prompt?.metadata_json)?.source_prompt_type;
  return typeof raw === "string" && raw.trim() ? raw.trim() : null;
}

export function videoWorkflowModeOfPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
): VideoSegmentWorkflowMode | null {
  const raw = prompt?.workflow_mode;
  return VIDEO_SEGMENT_WORKFLOW_MODES.includes(raw as VideoSegmentWorkflowMode)
    ? (raw as VideoSegmentWorkflowMode)
    : null;
}

export const videoPromptSupportsProviderGeneration = (
  prompt: ChapterVideoPrompt,
): boolean => {
  const workflowMode = videoWorkflowModeOfPrompt(prompt);
  return (
    workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES ||
    workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN ||
    workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD
  );
};

interface ReferenceImageIdsState {
  ids: number[];
  error: string | null;
}

export const readSavedReferenceImageIds = (
  value: unknown,
  t: MessageTranslator,
  field = "reference_image_ids_json",
): ReferenceImageIdsState => {
  if (value == null) return { ids: [], error: null };
  if (!Array.isArray(value)) {
    return { ids: [], error: t("video.chapter.invalidReferenceField", { field }) };
  }

  const ids: number[] = [];
  const seen = new Set<number>();
  for (const [index, item] of value.entries()) {
    if (!isPositiveSafeInteger(item)) {
      return {
        ids: [],
        error: t("video.chapter.invalidReferenceItem", { field, index }),
      };
    }
    if (seen.has(item)) {
      return {
        ids: [],
        error: t("video.chapter.duplicateReferenceId", { field, id: item }),
      };
    }
    seen.add(item);
    ids.push(item);
  }
  return { ids, error: null };
};

export function promptHasVideoWorkflowContract(
  prompt: ChapterVideoPrompt,
  contractVersion: string,
): boolean {
  return asRecord(prompt.metadata_json)?.contract_version === contractVersion;
}
