import {
  IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
  IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
  IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
  IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE,
  IMAGE_TYPE_OBJECT_MATERIAL,
  IMAGE_TYPE_SCENE_MATERIAL,
  IMAGE_TYPE_VIDEO_KEY_FRAME,
  IMAGE_TYPE_VIDEO_START_FRAME,
  IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
  IMAGE_TYPE_VIDEO_TAIL_FRAME,
  VIDEO_FRAME_IMAGE_TYPES,
  VIDEO_FRAME_PROMPT_IMAGE_TYPES,
  VIDEO_FRAME_PROMPT_IMAGE_GENERATION_SOURCES,
  type GeneratedImage,
  type ImageSize,
  type VideoFramePromptImageType,
  type VideoFramePromptImageGenerationSource,
} from "../../../api/image";
import { extractErrorMessage } from "../../../api";
import {
  VIDEO_TYPE_CHAPTER,
  VIDEO_TYPE_REFERENCE_MEDIA,
  VIDEO_SEGMENT_WORKFLOW_MODES,
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type PromptOutputLanguage,
  type SegmentGuidanceMode,
  type VideoGenConfig,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentWorkflowMode,
  type VideoDurationMode,
} from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import type { ChapterRow } from "../shared";
import { DEFAULT_VIDEO_STYLE } from "../styleTemplates";
import {
  formatDateTime,
  formatNumber,
  type Locale,
  type MessageTranslator,
} from "../../../i18n";

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
const PROMPT_OUTPUT_LANGUAGES: readonly PromptOutputLanguage[] = [
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
const VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION = "video_execution_contract_v5";
export const VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION =
  "boundary_frame_segment_local_v4";
export const VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION =
  "first_frame_chain_execution_v4";
const VIDEO_CHAT_VIDEO_CONTRACT_VERSION = "chat_video_execution_v4";
export const VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION =
  "storyboard_execution_unit_v2";
const VIDEO_STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION =
  "storyboard_execution_video_v2";
const VIDEO_EXECUTION_CONTRACT_EPOCH = 5;
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

const VIDEO_SEGMENT_EXECUTION_RECIPE_SCHEMA_VERSION =
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

type AssemblyPushStatus = "pending" | "accepted" | "rejected";

export const assemblyPushMetadata = (
  video: GeneratedVideo,
): Record<string, unknown> | null =>
  asRecord(asRecord(video.metadata_json)?.assembly_push);

export const assemblyPushStatus = (
  video: GeneratedVideo,
): AssemblyPushStatus | null => {
  const value = assemblyPushMetadata(video)?.status;
  return value === "pending" || value === "accepted" || value === "rejected"
    ? value
    : null;
};

export const assemblyPushIndex = (video: GeneratedVideo): number | null => {
  const value = assemblyPushMetadata(video)?.push_index;
  return Number.isInteger(value) && Number(value) > 0 ? Number(value) : null;
};

export const assemblyPushSourceIds = (
  video: GeneratedVideo,
): number[] | null => {
  const value = assemblyPushMetadata(video)?.source_video_ids;
  if (!Array.isArray(value) || value.length === 0) return null;
  const ids = value.filter(
    (item): item is number => Number.isInteger(item) && Number(item) > 0,
  );
  return ids.length === value.length && new Set(ids).size === ids.length
    ? ids
    : null;
};

export const latestNonRejectedAssemblyPush = (
  pushes: readonly GeneratedVideo[],
): GeneratedVideo | null => {
  for (let index = pushes.length - 1; index >= 0; index -= 1) {
    const push = pushes[index];
    if (assemblyPushStatus(push) !== "rejected") return push;
  }
  return null;
};

export function videoWorkflowModeOfVideo(
  video: GeneratedVideo,
): VideoSegmentWorkflowMode | null {
  const raw = video.workflow_mode;
  if (
    typeof raw === "string" &&
    VIDEO_SEGMENT_WORKFLOW_MODES.includes(raw as VideoSegmentWorkflowMode)
  ) {
    return raw as VideoSegmentWorkflowMode;
  }
  return null;
}

const videoProviderRequestSnapshotFromRecord = (
  video: GeneratedVideo,
): Record<string, unknown> | null =>
  asRecord(asRecord(video.metadata_json)?.provider_request_snapshot);

const snapshotTextField = (
  snapshot: Record<string, unknown>,
  field: string,
): string | null => {
  const value = snapshot[field];
  return typeof value === "string" && value.trim() ? value.trim() : null;
};

const videoProviderRequestSnapshotBlockedReason = (
  video: GeneratedVideo,
  t: MessageTranslator,
): string | null => {
  const snapshot = videoProviderRequestSnapshotFromRecord(video);
  if (!snapshot) {
    return t("video.chapter.providerSnapshotMissing");
  }
  if (snapshot.schema_version !== 1) {
    return t("video.chapter.providerSnapshotVersionInvalid");
  }
  for (const field of ["key", "title", "kind", "state"]) {
    if (!snapshotTextField(snapshot, field)) {
      return t("video.chapter.providerSnapshotFieldMissing", { field });
    }
  }
  if (!asRecord(snapshot.request)) {
    return t("video.chapter.providerSnapshotRequestMissing");
  }
  return null;
};

export const videoProviderRequestPreviewBlockedReason = (
  video: GeneratedVideo,
  t: MessageTranslator,
): string | null => {
  const workflowMode = videoWorkflowModeOfVideo(video);
  if (workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO) {
    return t("video.chapter.externalChatNoProvider");
  }
  if (
    video.video_type !== VIDEO_TYPE_CHAPTER &&
    video.video_type !== VIDEO_TYPE_REFERENCE_MEDIA
  ) {
    return t("video.chapter.trimmedNoProvider");
  }
  const snapshotBlockedReason =
    videoProviderRequestSnapshotBlockedReason(video, t);
  if (snapshotBlockedReason) {
    return snapshotBlockedReason;
  }
  if (workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA) {
    if (video.video_type !== VIDEO_TYPE_REFERENCE_MEDIA) {
      return t("video.chapter.trimmedNoProvider");
    }
    return null;
  }
  if (
    workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES ||
    workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN ||
    workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD
  ) {
    if (video.video_type !== VIDEO_TYPE_CHAPTER) {
      return t("video.chapter.trimmedNoProvider");
    }
    const snapshot = videoProviderRequestSnapshotFromRecord(video);
    const identity = asRecord(snapshot?.video_execution_identity);
    if (identity?.contract_epoch !== VIDEO_EXECUTION_CONTRACT_EPOCH) {
      return t("video.chapter.staleExecutionIdentity");
    }
    return null;
  }
  return t("video.chapter.invalidGenerationMode");
};

const GENERIC_VIDEO_DURATION_LIMITS: VideoDurationLimits = {
  model: null,
  durationMode: "range",
  defaultDurationSecs: 6,
  minDurationSecs: 1,
  maxDurationSecs: 60,
  supportedDurations: [],
  configError: null,
};

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
  return typeof raw === "number" && Number.isInteger(raw) && raw > 0
    ? raw
    : null;
};

const videoImageGenerationShotNumber = (img: GeneratedImage): number | null => {
  const raw = videoImageGenerationContext(img)?.shot_number;
  return typeof raw === "number" && Number.isInteger(raw) && raw > 0
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

const videoImageGenerationSourceHash = (img: GeneratedImage): string | null => {
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
  return typeof raw === "number" && Number.isInteger(raw) && raw > 0
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
  if (value < 60) {
    return t("video.postprocess.seconds", {
      count: formatNumber(Math.round(value), locale),
    });
  }
  const minutes = Math.floor(value / 60);
  const seconds = Math.round(value - minutes * 60);
  return t("video.postprocess.minutesSeconds", {
    minutes: formatNumber(minutes, locale),
    seconds: formatNumber(seconds, locale),
  });
};

function isVideoFrameInputImage(img: GeneratedImage): boolean {
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

function profileMaterialKindOfImage(
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
  if (typeof value === "number" && Number.isInteger(value) && value > 0)
    return value;
  if (typeof value === "string") {
    const text = value.trim();
    if (!/^\+?\d+$/.test(text)) return null;
    const parsed = Number(text);
    return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : null;
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
    if (typeof item !== "number" || !Number.isInteger(item) || item <= 0) {
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

const VIDEO_EXECUTION_BASE_IDENTITY_FIELDS = [
  "source_content_sha256",
  "style_sha256",
  "duration_secs",
  "aspect_ratio",
] as const;

const VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS = [
  "source_content_sha256",
  "style_sha256",
  "aspect_ratio",
] as const;

const VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS = [
  "source_shot_id",
  "segment_index_in_shot",
  "segment_sha256",
] as const;

export function videoExecutionIdentityOfPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
): Record<string, unknown> | null {
  const identity = asRecord(
    asRecord(prompt?.metadata_json)?.video_execution_identity,
  );
  if (identity?.contract_epoch !== VIDEO_EXECUTION_CONTRACT_EPOCH) return null;
  return identity;
}

const videoExecutionIdentityMatchesExpected = (
  actual: Record<string, unknown> | null,
  expected: Record<string, unknown> | null,
  fields: readonly string[],
): boolean =>
  Boolean(
    actual &&
    expected &&
    fields.every(
      (field) =>
        expected[field] !== undefined && actual[field] === expected[field],
    ),
  );

type SegmentIdentityIndex = Map<number, Record<string, unknown> | null>;

export interface VideoExecutionSegmentIdentityIndex {
  segments: unknown[];
  bySegmentNumber: SegmentIdentityIndex;
  byIdentity: Map<string, number>;
}

const segmentIdentityKey = (
  identity: Record<string, unknown> | null | undefined,
): string =>
  JSON.stringify(
    VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS.map((field) => identity?.[field]),
  );

export function videoExecutionSegmentIdentityIndex(
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
): VideoExecutionSegmentIdentityIndex {
  const metadata = asRecord(segmentPlanPrompt?.metadata_json);
  const segments = Array.isArray(metadata?.segments) ? metadata.segments : [];
  const bySourceAndIndex = new Map<string, Map<number, Record<string, unknown>>>();
  const identities = videoExecutionIdentityOfPrompt(segmentPlanPrompt)?.segment_identities;
  if (Array.isArray(identities)) {
    for (const value of identities) {
      const identity = asRecord(value);
      const sourceShotId = identity?.source_shot_id;
      const segmentIndexInShot = identity?.segment_index_in_shot;
      if (
        typeof sourceShotId !== "string" ||
        typeof segmentIndexInShot !== "number" ||
        typeof identity?.segment_sha256 !== "string" ||
        identity.segment_sha256.length === 0
      ) {
        continue;
      }
      const byIndex = bySourceAndIndex.get(sourceShotId) ?? new Map();
      if (!byIndex.has(segmentIndexInShot)) byIndex.set(segmentIndexInShot, identity);
      bySourceAndIndex.set(sourceShotId, byIndex);
    }
  }

  const bySegmentNumber: SegmentIdentityIndex = new Map();
  const byIdentity = new Map<string, number>();
  for (const value of segments) {
    const segment = asRecord(value);
    const segmentNumber = segment?.segment_number;
    if (
      typeof segmentNumber !== "number" ||
      !Number.isInteger(segmentNumber) ||
      segmentNumber <= 0 ||
      bySegmentNumber.has(segmentNumber)
    ) {
      continue;
    }
    const sourceShotId = segment?.source_shot_id;
    const segmentIndexInShot = segment?.segment_index_in_shot;
    if (
      typeof sourceShotId !== "string" ||
      !sourceShotId.trim() ||
      typeof segmentIndexInShot !== "number" ||
      !Number.isInteger(segmentIndexInShot) ||
      segmentIndexInShot <= 0
    ) {
      bySegmentNumber.set(segmentNumber, null);
      continue;
    }
    const expected =
      bySourceAndIndex.get(sourceShotId)?.get(segmentIndexInShot) ?? null;
    bySegmentNumber.set(segmentNumber, expected);
    if (expected) {
      const key = segmentIdentityKey(expected);
      if (!byIdentity.has(key)) byIdentity.set(key, segmentNumber);
    }
  }
  return { segments, bySegmentNumber, byIdentity };
}

const videoExecutionSegmentIdentityMatchesPlan = (
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
  fields: readonly string[],
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): boolean => {
  return (
    videoExecutionSegmentNumberInPlanMatching(
      actual,
      segmentPlanPrompt,
      fields,
      identityIndex,
    ) === segmentNumber
  );
};

const segmentPlanExecutionIdentityIsComplete = (
  prompt: ChapterVideoPrompt,
): boolean => {
  const { segments, bySegmentNumber } = videoExecutionSegmentIdentityIndex(prompt);
  const identities = videoExecutionIdentityOfPrompt(prompt)?.segment_identities;
  return (
    segments.length > 0 &&
    Array.isArray(identities) &&
    identities.length === segments.length &&
    segments.every((value) => {
      const segmentNumber = asRecord(value)?.segment_number;
      return (
        typeof segmentNumber === "number" &&
        Number.isInteger(segmentNumber) &&
        segmentNumber > 0 &&
        (bySegmentNumber.get(segmentNumber) ?? null) != null
      );
    })
  );
};

const videoExecutionSegmentNumberInPlanMatching = (
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  fields: readonly string[],
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): number | null => {
  const planIdentity = videoExecutionIdentityOfPrompt(segmentPlanPrompt);
  if (
    !videoExecutionIdentityMatchesExpected(actual, planIdentity, fields) ||
    actual?.segment_plan_prompt_id !== segmentPlanPrompt?.id
  ) {
    return null;
  }
  return (
    identityIndex ?? videoExecutionSegmentIdentityIndex(segmentPlanPrompt)
  ).byIdentity.get(segmentIdentityKey(actual)) ?? null;
};

export function videoExecutionSegmentNumberInPlan(
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): number | null {
  return videoExecutionSegmentNumberInPlanMatching(
    actual,
    segmentPlanPrompt,
    VIDEO_EXECUTION_BASE_IDENTITY_FIELDS,
    identityIndex,
  );
}

export function videoExecutionSegmentNumberInPlanByLineage(
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): number | null {
  return videoExecutionSegmentNumberInPlanMatching(
    actual,
    segmentPlanPrompt,
    VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS,
    identityIndex,
  );
}

export function videoExecutionIdentityMatches(
  prompt: ChapterVideoPrompt | null | undefined,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
): boolean {
  return videoExecutionIdentityMatchesExpected(
    videoExecutionIdentityOfPrompt(prompt),
    videoExecutionIdentityOfPrompt(segmentPlanPrompt),
    VIDEO_EXECUTION_BASE_IDENTITY_FIELDS,
  );
}

export function videoExecutionIdentityOfVideo(
  video: GeneratedVideo | null | undefined,
): Record<string, unknown> | null {
  const identity = asRecord(
    asRecord(video?.metadata_json)?.video_execution_identity,
  );
  return identity?.contract_epoch === VIDEO_EXECUTION_CONTRACT_EPOCH
    ? identity
    : null;
}

export function videoExecutionSegmentIdentityMatchesPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
  recipeFingerprint: string | null | undefined,
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): boolean {
  const identity = videoExecutionIdentityOfPrompt(prompt);
  return (
    Boolean(recipeFingerprint) &&
    videoExecutionSegmentIdentityMatchesPlan(
      identity,
      segmentPlanPrompt,
      segmentNumber,
      prompt?.prompt_type === VIDEO_FINAL_PROMPT_TYPE
        ? VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS
        : VIDEO_EXECUTION_BASE_IDENTITY_FIELDS,
      identityIndex,
    ) &&
    identity?.segment_recipe_fingerprint === recipeFingerprint
  );
}

export function videoExecutionSegmentIdentityMatchesVideo(
  video: GeneratedVideo | null | undefined,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
  recipeFingerprint: string | null | undefined,
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): boolean {
  const identity = videoExecutionIdentityOfVideo(video);
  return (
    Boolean(recipeFingerprint) &&
    videoExecutionSegmentIdentityMatchesPlan(
      identity,
      segmentPlanPrompt,
      segmentNumber,
      VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS,
      identityIndex,
    ) &&
    identity?.segment_recipe_fingerprint === recipeFingerprint
  );
}

const promptIsSharedVideoUpstream = (prompt: ChapterVideoPrompt): boolean =>
  prompt.workflow_mode === VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN &&
  prompt.prompt_type === VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE;

export function videoExecutionPromptsShareGenerationChain(
  left: ChapterVideoPrompt,
  right: ChapterVideoPrompt,
): boolean {
  if (
    left.project_id !== right.project_id ||
    left.chapter_id !== right.chapter_id ||
    left.style !== right.style ||
    left.aspect_ratio !== right.aspect_ratio
  ) {
    return false;
  }
  const leftShared = promptIsSharedVideoUpstream(left);
  const rightShared = promptIsSharedVideoUpstream(right);
  if (
    !leftShared &&
    !rightShared &&
    left.workflow_mode !== right.workflow_mode
  ) {
    return false;
  }
  if (!videoExecutionIdentityMatches(left, right)) return false;
  const leftIdentity = videoExecutionIdentityOfPrompt(left);
  const rightIdentity = videoExecutionIdentityOfPrompt(right);
  if (!leftIdentity || !rightIdentity) return false;
  const identityExistsInPlan = (
    planIdentity: Record<string, unknown>,
    segmentIdentity: Record<string, unknown>,
  ): boolean =>
    Array.isArray(planIdentity.segment_identities) &&
    planIdentity.segment_identities
      .map(asRecord)
      .some(
        (candidate) =>
          candidate != null &&
          VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS.every(
            (field) => candidate[field] === segmentIdentity[field],
          ),
      );
  if (leftShared) {
    return (
      rightIdentity.segment_plan_prompt_id === left.id &&
      identityExistsInPlan(leftIdentity, rightIdentity)
    );
  }
  if (rightShared) {
    return (
      leftIdentity.segment_plan_prompt_id === right.id &&
      identityExistsInPlan(rightIdentity, leftIdentity)
    );
  }
  const videoAndFramePromptPair =
    (left.prompt_type === VIDEO_FINAL_PROMPT_TYPE &&
      isVideoFramePromptType(right.prompt_type)) ||
    (right.prompt_type === VIDEO_FINAL_PROMPT_TYPE &&
      isVideoFramePromptType(left.prompt_type));
  return (
    leftIdentity.segment_plan_prompt_id != null &&
    leftIdentity.segment_plan_prompt_id ===
      rightIdentity.segment_plan_prompt_id &&
    VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS.every(
      (field) => leftIdentity[field] === rightIdentity[field],
    ) &&
    (videoAndFramePromptPair ||
      leftIdentity.segment_recipe_fingerprint ===
        rightIdentity.segment_recipe_fingerprint)
  );
}

export function videoSegmentRecipeGenerationFingerprintJson(
  recipe: VideoSegmentExecutionRecipe,
): string {
  return JSON.stringify(
    canonicalJsonValue(videoSegmentRecipeFingerprintValue(recipe, false)),
  );
}

export function videoSegmentRecipeFrameFingerprintJson(
  recipe: VideoSegmentExecutionRecipe,
): string {
  return JSON.stringify(
    canonicalJsonValue(videoSegmentRecipeFingerprintValue(recipe, true)),
  );
}

const canonicalJsonValue = (value: unknown): unknown => {
  if (Array.isArray(value)) return value.map(canonicalJsonValue);
  const object = asRecord(value);
  if (!object) return value;
  return Object.fromEntries(
    Object.keys(object)
      .sort()
      .map((key) => [key, canonicalJsonValue(object[key])]),
  );
};

const videoReferenceMediaBindingFingerprintValue = (
  row: NonNullable<
    VideoSegmentExecutionRecipe["reference_media"]
  >["image_rows"][number],
) => ({
  url: row.url,
  usage: row.usage,
  upload_sha256: row.upload_sha256,
  upload_file_name: row.upload_file_name,
  upload_mime_type: row.upload_mime_type,
});

function videoSegmentRecipeFingerprintValue(
  recipe: VideoSegmentExecutionRecipe,
  frameStage: boolean,
): Record<string, unknown> {
  const guidanceMode = frameStage ? "none" : recipe.guidance_mode;
  const referenceMedia =
    !frameStage &&
    recipe.workflow_mode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA &&
    recipe.reference_media
      ? {
          prompt: recipe.reference_media.prompt,
          image_rows: recipe.reference_media.image_rows.map(
            videoReferenceMediaBindingFingerprintValue,
          ),
          video_rows: recipe.reference_media.video_rows.map(
            videoReferenceMediaBindingFingerprintValue,
          ),
          audio_rows: recipe.reference_media.audio_rows.map(
            videoReferenceMediaBindingFingerprintValue,
          ),
        }
      : null;
  return {
    schema_version: recipe.schema_version,
    workflow_mode: recipe.workflow_mode,
    style: recipe.style,
    prompt_output_language: recipe.prompt_output_language,
    duration_secs: recipe.duration_secs,
    duration_secs_options: recipe.duration_secs_options,
    aspect_ratio: recipe.aspect_ratio,
    reference_image_ids: recipe.reference_image_ids,
    video_library_image_ids: recipe.video_library_image_ids,
    guidance_mode: guidanceMode,
    guidance_reference_image_ids:
      guidanceMode === "keyframes" ? recipe.guidance_reference_image_ids : [],
    continuity_source_video_id:
      guidanceMode === "continuity" ? recipe.continuity_source_video_id : null,
    reference_media: referenceMedia,
    adopted_video_id: null,
  };
}

export function latestVideoSegmentPlanPrompt(
  prompts: ChapterVideoPrompt[],
  currentContentHash: string | null,
): ChapterVideoPrompt | null {
  if (!currentContentHash) return null;
  let latest: ChapterVideoPrompt | null = null;
  for (const prompt of prompts) {
    if (
      prompt.prompt_type === VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE &&
      prompt.workflow_mode === VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN &&
      prompt.status === "completed" &&
      promptHasVideoWorkflowContract(
        prompt,
        VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
      ) &&
      segmentPlanExecutionIdentityIsComplete(prompt) &&
      extractSourceContentHash(prompt.metadata_json) === currentContentHash &&
      (latest == null || prompt.id > latest.id)
    ) {
      latest = prompt;
    }
  }
  return latest;
}

export function videoOptionsFromSegmentRecipe(
  recipe: VideoSegmentExecutionRecipe | null,
): VideoGenOptions {
  if (!recipe) return DEFAULT_VIDEO_OPTIONS;
  return {
    style: recipe.style,
    promptOutputLanguage: recipe.prompt_output_language,
    durationSecs: recipe.duration_secs,
    durationOptions: recipe.duration_secs_options,
    aspectRatio: recipe.aspect_ratio as VideoAspectRatio,
  };
}

export interface VideoSegmentCardPlan {
  segmentNumber: number;
  title: string;
  sceneSummary: string;
  durationSecs: number;
  aspectRatio: VideoAspectRatio;
  raw: Record<string, unknown>;
}

interface VideoSegmentRecipeReadResult {
  recipes: Map<number, VideoSegmentExecutionRecipe>;
  error: string | null;
}

export function videoSegmentCardsFromPlanPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
  t: MessageTranslator,
): VideoSegmentCardPlan[] {
  const metadata = asRecord(prompt?.metadata_json);
  return videoSegmentCardsFromSegments(
    metadata && Array.isArray(metadata.segments) ? metadata.segments : [],
    t,
  );
}

export function videoSegmentCardsFromSegments(
  segments: unknown[],
  t: MessageTranslator,
): VideoSegmentCardPlan[] {
  return segments
    .map(asRecord)
    .filter((segment): segment is Record<string, unknown> => segment != null)
    .map((segment) => {
      const segmentNumber = segment.segment_number;
      if (
        typeof segmentNumber !== "number" ||
        !Number.isInteger(segmentNumber) ||
        segmentNumber <= 0
      ) {
        throw new Error(t("video.chapter.invalidSegmentNumber"));
      }
      const durationSecs = segment.duration_secs;
      const aspectRatio = segment.aspect_ratio;
      if (
        typeof durationSecs !== "number" ||
        !Number.isInteger(durationSecs) ||
        durationSecs <= 0
      ) {
        throw new Error(t("video.chapter.invalidSegmentDuration", { number: segmentNumber }));
      }
      if (
        typeof aspectRatio !== "string" ||
        !VIDEO_ASPECT_RATIOS.includes(aspectRatio as VideoAspectRatio)
      ) {
        throw new Error(t("video.chapter.invalidSegmentAspectRatio", { number: segmentNumber }));
      }
      const title =
        typeof segment.title === "string" ? segment.title.trim() : "";
      if (!title) throw new Error(t("video.chapter.invalidSegmentTitle", { number: segmentNumber }));
      const sceneSummary =
        typeof segment.scene_summary === "string"
          ? segment.scene_summary.trim()
          : "";
      if (!sceneSummary)
        throw new Error(t("video.chapter.invalidSegmentSummary", { number: segmentNumber }));
      return {
        segmentNumber,
        title,
        sceneSummary,
        durationSecs,
        aspectRatio: aspectRatio as VideoAspectRatio,
        raw: segment,
      };
    })
    .sort((left, right) => left.segmentNumber - right.segmentNumber);
}

export function readVideoSegmentRecipes(
  prompt: ChapterVideoPrompt | null | undefined,
  t: MessageTranslator,
): VideoSegmentRecipeReadResult {
  const metadata = asRecord(prompt?.metadata_json);
  const raw = metadata?.segment_recipes;
  if (!prompt) return { recipes: new Map(), error: null };
  if (raw == null) {
    return {
      recipes: new Map(),
      error: t("video.chapter.missingSegmentRecipes"),
    };
  }
  if (!Array.isArray(raw)) {
    return { recipes: new Map(), error: t("video.chapter.segmentRecipesNotArray") };
  }
  try {
    const recipes = new Map(
      parseVideoSegmentRecipeArray(raw, t).map((recipe) => [
        recipe.segment_number,
        recipe,
      ]),
    );
    return { recipes, error: null };
  } catch (error) {
    return {
      recipes: new Map(),
      error: extractErrorMessage(error, t("video.chapter.segmentRecipesReadFailed")),
    };
  }
}

function parseVideoSegmentRecipeArray(
  raw: unknown[],
  t: MessageTranslator,
): VideoSegmentExecutionRecipe[] {
  const recipes = new Map<number, VideoSegmentExecutionRecipe>();
  for (const [index, value] of raw.entries()) {
    const recipe = asRecord(value);
    if (!recipe) throw new Error(t("video.chapter.recipeNotObject", { index }));
    if (
      recipe.schema_version !== VIDEO_SEGMENT_EXECUTION_RECIPE_SCHEMA_VERSION
    ) {
      throw new Error(t("video.chapter.recipeSchemaInvalid", { index }));
    }
    const segmentNumber = recipe.segment_number;
    if (
      typeof segmentNumber !== "number" ||
      !Number.isInteger(segmentNumber) ||
      segmentNumber <= 0
    ) {
      throw new Error(t("video.chapter.recipeSegmentNumberInvalid", { index }));
    }
    if (recipes.has(segmentNumber)) {
      throw new Error(t("video.chapter.recipeDuplicateSegment", { number: segmentNumber }));
    }
    if (
      typeof recipe.workflow_mode !== "string" ||
      !VIDEO_SEGMENT_WORKFLOW_MODES.includes(
        recipe.workflow_mode as VideoSegmentWorkflowMode,
      )
    ) {
      throw new Error(t("video.chapter.recipeWorkflowInvalid", { number: segmentNumber }));
    }
    recipes.set(segmentNumber, value as VideoSegmentExecutionRecipe);
  }
  return [...recipes.values()];
}

export function finalVideoExecutionIdentityIsComplete(
  prompt: ChapterVideoPrompt | null | undefined,
): boolean {
  const identity = videoExecutionIdentityOfPrompt(prompt);
  if (!identity) return false;
  return (
    typeof identity.frame_set_sha256 === "string" &&
    identity.frame_set_sha256.trim().length > 0 &&
    typeof identity.prompt_output_language === "string" &&
    identity.prompt_output_language.trim().length > 0 &&
    asRecord(identity.generation_asset_fingerprint) != null
  );
}

export function currentFinalVideoPromptMatchesSegment(
  prompt: ChapterVideoPrompt,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
  recipeFingerprint: string | null | undefined,
  workflowMode: VideoSegmentWorkflowMode | null | undefined,
  promptOutputLanguage: PromptOutputLanguage,
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): boolean {
  const contractVersion =
    workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO
      ? VIDEO_CHAT_VIDEO_CONTRACT_VERSION
      : workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD
        ? VIDEO_STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION
        : workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN
          ? VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION
          : workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
            ? VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION
            : null;
  const metadata = asRecord(prompt.metadata_json);
  return (
    contractVersion != null &&
    prompt.prompt_type === VIDEO_FINAL_PROMPT_TYPE &&
    videoWorkflowModeOfPrompt(prompt) === workflowMode &&
    promptHasVideoWorkflowContract(prompt, contractVersion) &&
    promptSourcePromptType(prompt) === VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE &&
    metadata?.prompt_output_language === promptOutputLanguage &&
    videoExecutionSegmentIdentityMatchesPrompt(
      prompt,
      segmentPlanPrompt,
      segmentNumber,
      recipeFingerprint,
      identityIndex,
    ) &&
    finalVideoExecutionIdentityIsComplete(prompt) &&
    videoExecutionIdentityOfPrompt(prompt)?.prompt_output_language ===
      promptOutputLanguage
  );
}

export function completedSegmentPlanPreviewItems(
  streamText: string,
): Record<string, unknown>[] {
  const byShot = new Map<
    string,
    {
      shotIndex: number | null;
      order: number;
      segments: Record<string, unknown>[];
    }
  >();
  const unscoped: Array<{
    shotIndex: number | null;
    order: number;
    segments: Record<string, unknown>[];
  }> = [];
  let order = 0;
  for (const line of streamText.split("\n")) {
    if (!line.trim()) continue;
    let value: Record<string, unknown> | null;
    try {
      value = asRecord(JSON.parse(line));
    } catch {
      break;
    }
    const group = Array.isArray(value?.segments)
      ? value.segments
          .map(asRecord)
          .filter((item): item is Record<string, unknown> => item != null)
      : [];
    if (group.length === 0) continue;
    const currentOrder = order;
    order += 1;
    const shotIndex = positiveIntegerOrNull(value?.shot_index);
    const shotIds = new Set(
      group
        .map((item) => item.source_shot_id)
        .filter(
          (value): value is string =>
            typeof value === "string" && value.length > 0,
        ),
    );
    if (shotIds.size === 1) {
      const sourceShotId = [...shotIds][0];
      const previous = byShot.get(sourceShotId);
      byShot.set(sourceShotId, {
        shotIndex: shotIndex ?? previous?.shotIndex ?? null,
        order: previous?.order ?? currentOrder,
        segments: group,
      });
    } else {
      unscoped.push({ shotIndex, order: currentOrder, segments: group });
    }
  }
  return [...byShot.values(), ...unscoped]
    .sort((left, right) => {
      if (left.shotIndex != null && right.shotIndex != null) {
        return left.shotIndex - right.shotIndex;
      }
      if (left.shotIndex != null) return -1;
      if (right.shotIndex != null) return 1;
      return left.order - right.order;
    })
    .flatMap((group) => group.segments)
    .map((segment, index) => ({
      ...segment,
      segment_number: index + 1,
    }));
}

const SEGMENT_PLAN_SHOT_CHECKPOINT_PREFIX = "video_segment_plan.shot.";

export function isSegmentPlanDraftCheckpointKey(value: string): boolean {
  return (
    value === "video_segment_plan.recovered" ||
    (value.startsWith(SEGMENT_PLAN_SHOT_CHECKPOINT_PREFIX) &&
      value.length > SEGMENT_PLAN_SHOT_CHECKPOINT_PREFIX.length)
  );
}

export function videoPromptSourceFrameNumbers(
  prompt: ChapterVideoPrompt | null | undefined,
  workflowMode: VideoSegmentWorkflowMode | null | undefined,
  guidanceMode?: SegmentGuidanceMode | null,
): { startFrameNumber: number | null; endFrameNumber: number | null } {
  if (workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES) {
    return { startFrameNumber: 1, endFrameNumber: 2 };
  }
  if (
    workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN &&
    guidanceMode === "system_frames"
  ) {
    return { startFrameNumber: 1, endFrameNumber: null };
  }
  const unit = asRecord(asRecord(prompt?.metadata_json)?.video_unit);
  return {
    startFrameNumber: positiveIntegerOrNull(unit?.start_frame_number),
    endFrameNumber: positiveIntegerOrNull(unit?.end_frame_number),
  };
}

function positiveIntegerOrNull(value: unknown): number | null {
  return typeof value === "number" && Number.isInteger(value) && value > 0
    ? value
    : null;
}

export function buildVideoFrameImageTraceMetadata(
  prompt: ChapterVideoPrompt,
  source: VideoFramePromptImageGenerationSource,
  t: MessageTranslator,
): Record<string, unknown> {
  const sourceContentHash = extractSourceContentHash(prompt.metadata_json);
  if (sourceContentHash == null) {
    throw new Error(
      t("video.chapter.invalidFramePromptSourceHash", { id: prompt.id }),
    );
  }
  return {
    generation_context: {
      source,
      prompt_id: prompt.id,
      shot_number: prompt.shot_number,
      title: prompt.title,
      scene_summary: prompt.scene_summary,
      source_content_sha256: sourceContentHash,
    },
    prompt_snapshot: {
      visual_prompt: prompt.visual_prompt,
      motion_prompt: prompt.motion_prompt,
      negative_prompt: prompt.negative_prompt,
      duration_secs: prompt.duration_secs,
      aspect_ratio: prompt.aspect_ratio,
      reference_image_ids: Array.isArray(prompt.reference_image_ids_json)
        ? prompt.reference_image_ids_json
        : [],
    },
  };
}

const VIDEO_REAL_FRAME_IMAGE_PROMPT_GUARD =
  "只生成单张真实小说画面。若提示词中出现参考图、素材图或参考图编号，它们只用于人物、场景、道具或风格一致性约束，不是画面内容，不是视频首帧/尾帧输入，不能放进 image 字段，不要画成参考板、拼贴图、多格排版、截图列表，也不要把任何参考图当作视频开头画面。";

const VIDEO_STORYBOARD_FRAME_IMAGE_PROMPT_GUARD =
  "生成一张完整的动态多格视频分镜板，不是单张电影剧照。画面内部必须包含 2 到 6 个清晰画格，按从左到右、从上到下阅读，用连续画格表达同一视频单元的状态变化。允许画格边界和分镜板版式；不要生成字幕、水印、乱码文字、文字编号、素材展示墙、无关拼贴或截图列表。参考图只用于人物、场景、道具、服装或风格一致性约束，不要把参考图画成画面内容。";

function buildPreviousFrameReferencePromptNote(
  previousPrompt: ChapterVideoPrompt,
): string {
  const title =
    trimmedText(previousPrompt.title) ||
    `边界帧 #${previousPrompt.shot_number}`;
  const summary = trimmedText(previousPrompt.scene_summary) || "无摘要";
  return [
    "【上一帧连续性约束】参考图 1 是上一帧实际画面，只用于保持人物身份、服装、空间锚点、左右关系、主要道具、镜头轴线和光源方向连续。",
    `上一帧状态：${title}；${summary}`,
    "请从参考图 1 读取上一帧的脸型、发色、服装、空间布局和光源方向；不要复述或重画上一帧内容，当前画面以本次帧提示词为准。其他参考图按实际输入顺序从参考图 2 开始。",
  ].join("\n");
}

function mergeFrameReferenceIds(
  savedReferenceIds: number[],
  previousFrameImageId: number | null = null,
): number[] {
  const ids: number[] = [];
  if (previousFrameImageId != null) ids.push(previousFrameImageId);
  for (const id of savedReferenceIds) {
    if (!ids.includes(id)) ids.push(id);
  }
  return ids;
}

function referenceLabelOffsetForPreviousFrame(
  savedReferenceIds: number[],
  previousFrameImageId: number | null,
): number {
  return previousFrameImageId != null &&
    savedReferenceIds[0] !== previousFrameImageId
    ? 1
    : 0;
}

interface VideoFrameImageReferences {
  ids: number[];
  note: string;
  promptReferenceOffset: number;
  error?: string;
}

export function isBoundaryFramePrompt(
  prompt: Pick<ChapterVideoPrompt, "prompt_type">,
): boolean {
  return prompt.prompt_type === VIDEO_FRAME_PROMPT_TYPE;
}

function promptUpdatedAtMillis(prompt: ChapterVideoPrompt): number {
  if (!prompt.updated_at) return 0;
  const millis = Date.parse(prompt.updated_at);
  return Number.isFinite(millis) ? millis : 0;
}

function newerPrompt(
  current: ChapterVideoPrompt | undefined,
  candidate: ChapterVideoPrompt,
): ChapterVideoPrompt {
  if (!current) return candidate;
  const currentMillis = promptUpdatedAtMillis(current);
  const candidateMillis = promptUpdatedAtMillis(candidate);
  if (candidateMillis !== currentMillis) {
    return candidateMillis > currentMillis ? candidate : current;
  }
  return candidate.id > current.id ? candidate : current;
}

export function latestVideoFrameImageForPrompt(
  prompt: ChapterVideoPrompt,
  images: GeneratedImage[],
  t: MessageTranslator,
): GeneratedImage | null {
  const frameContract = videoFrameImageContractForPromptType(prompt.prompt_type, t);
  const promptHash = extractSourceContentHash(prompt.metadata_json);
  let latest: GeneratedImage | null = null;
  for (const image of images) {
    let matches = false;
    if (videoImageGenerationPromptId(image) === prompt.id) {
      matches =
        image.image_type === frameContract.imageType &&
        videoImageGenerationSource(image) === frameContract.source;
    } else if (isBoundaryFramePrompt(prompt) && promptHash != null) {
      matches =
        videoTailFrameBoundaryNumber(image) === prompt.shot_number &&
        videoImageGenerationSourceHash(image) === promptHash;
    }
    if (matches && (latest == null || image.id > latest.id)) latest = image;
  }
  return latest;
}

export function frameImageReferencesForPrompt(
  prompt: ChapterVideoPrompt,
  images: GeneratedImage[],
  prompts: ChapterVideoPrompt[],
  t: MessageTranslator,
): VideoFrameImageReferences {
  const savedReferences = readSavedReferenceImageIds(
    prompt.reference_image_ids_json,
    t,
  );
  if (savedReferences.error) {
    return {
      ids: [],
      note: "",
      promptReferenceOffset: 0,
      error: savedReferences.error,
    };
  }
  const ids = savedReferences.ids;
  if (!isBoundaryFramePrompt(prompt) || prompt.shot_number <= 1) {
    return { ids, note: "", promptReferenceOffset: 0 };
  }
  const previousPrompt = prompts.reduce<ChapterVideoPrompt | undefined>(
    (selected, item) => {
      if (
        item.prompt_type !== prompt.prompt_type ||
        item.workflow_mode !== prompt.workflow_mode ||
        item.shot_number !== prompt.shot_number - 1
      ) {
        return selected;
      }
      return newerPrompt(selected, item);
    },
    undefined,
  );
  if (!previousPrompt) {
    return {
      ids,
      note: "",
      promptReferenceOffset: 0,
      error: t("video.chapter.missingPreviousBoundaryPrompt", {
        number: prompt.shot_number - 1,
        target: prompt.shot_number,
      }),
    };
  }
  if (previousPrompt.status === "running") {
    return {
      ids,
      note: "",
      promptReferenceOffset: 0,
      error: t("video.chapter.previousBoundaryPromptRunning", {
        number: prompt.shot_number - 1,
        target: prompt.shot_number,
      }),
    };
  }
  const previousImage = latestVideoFrameImageForPrompt(previousPrompt, images, t);
  if (!previousImage) {
    return {
      ids,
      note: "",
      promptReferenceOffset: 0,
      error: t("video.chapter.previousBoundaryImageMissing", {
        number: prompt.shot_number - 1,
        target: prompt.shot_number,
      }),
    };
  }
  const previousIndex = ids.indexOf(previousImage.id);
  return {
    ids: mergeFrameReferenceIds(ids, previousImage.id),
    note: buildPreviousFrameReferencePromptNote(previousPrompt),
    promptReferenceOffset: referenceLabelOffsetForPreviousFrame(
      ids,
      previousImage.id,
    ),
    error:
      previousIndex > 0
        ? t("video.chapter.previousFrameNotFirstReference", {
            id: previousImage.id,
          })
        : undefined,
  };
}

export function boundaryFramePromptChainTo(
  targetPrompt: ChapterVideoPrompt,
  prompts: ChapterVideoPrompt[],
  operation: string,
  t: MessageTranslator,
): ChapterVideoPrompt[] {
  const byShotNumber = new Map<number, ChapterVideoPrompt>();
  for (const item of prompts) {
    if (
      item.prompt_type !== targetPrompt.prompt_type ||
      item.workflow_mode !== targetPrompt.workflow_mode ||
      item.shot_number > targetPrompt.shot_number
    ) {
      continue;
    }
    byShotNumber.set(
      item.shot_number,
      newerPrompt(byShotNumber.get(item.shot_number), item),
    );
  }
  byShotNumber.set(targetPrompt.shot_number, targetPrompt);

  const chain: ChapterVideoPrompt[] = [];
  for (
    let shotNumber = 1;
    shotNumber <= targetPrompt.shot_number;
    shotNumber += 1
  ) {
    const item = byShotNumber.get(shotNumber);
    if (!item) {
      throw new Error(
        t("video.chapter.incompleteBoundaryChain", {
          number: shotNumber,
          operation,
          target: targetPrompt.shot_number,
        }),
      );
    }
    chain.push(item);
  }
  return chain;
}

export function shiftReferenceLabels(prompt: string, offset: number): string {
  if (offset === 0) return prompt;
  return prompt.replace(/参考图\s+(\d+)/g, (_match, raw: string) => {
    const index = Number(raw);
    return Number.isSafeInteger(index) && index > 0
      ? `参考图 ${index + offset}`
      : `参考图 ${raw}`;
  });
}

export function referenceLabelOrderError(
  prompt: string,
  referenceCount: number,
  t: MessageTranslator,
): string | null {
  const text = prompt.trim();
  const compactMatch = text.match(/参考图\d+/);
  if (compactMatch) {
    return t("video.chapter.compactReferenceLabel", { value: compactMatch[0] });
  }
  const labelRegex = /参考图\s+(\d+)/g;
  let match: RegExpExecArray | null;
  while ((match = labelRegex.exec(text)) != null) {
    const raw = match[1];
    if (raw.length > 1 && raw.startsWith("0")) {
      return t("video.chapter.leadingZeroReferenceLabel", { number: raw });
    }
    const index = Number(raw);
    if (!Number.isSafeInteger(index) || index < 1 || index > referenceCount) {
      return referenceCount === 0
        ? t("video.chapter.noReferenceLabelAllowed")
        : t("video.chapter.invalidReferenceLabel", {
            number: index,
            count: referenceCount,
          });
    }
  }
  return null;
}

export function buildVideoFrameImageRequest(
  imageType: VideoFramePromptImageType,
  prompt: string,
  chapterId: number,
  aspectRatio: string | null | undefined,
  metadata: Record<string, unknown>,
  referenceImageIds: number[] = [],
  referencePromptNote = "",
  t: MessageTranslator,
): Record<string, unknown> {
  const trimmedPrompt = prompt.trim();
  const promptWithReferenceNote = [referencePromptNote.trim(), trimmedPrompt]
    .filter(Boolean)
    .join("\n\n");
  if (
    typeof aspectRatio !== "string" ||
    !VIDEO_ASPECT_RATIOS.includes(aspectRatio as VideoAspectRatio)
  ) {
    throw new Error(t("video.chapter.invalidFrameAspectRatio"));
  }
  const promptGuard =
    imageType === IMAGE_TYPE_VIDEO_STORYBOARD_FRAME
      ? VIDEO_STORYBOARD_FRAME_IMAGE_PROMPT_GUARD
      : VIDEO_REAL_FRAME_IMAGE_PROMPT_GUARD;
  return {
    prompt: `${promptWithReferenceNote}\n\n${promptGuard}`,
    image_type: imageType,
    chapter_id: chapterId,
    aspect_ratio: aspectRatio,
    metadata_json: metadata,
    reference_image_ids: referenceImageIds,
  };
}

export const trimmedText = (value: string | null | undefined): string => {
  return value == null ? "" : value.trim();
};

export const staleVideoPromptMessage = (
  frameStageLabel: string,
  t: MessageTranslator,
): string =>
  t("video.chapter.stalePrompt", { stage: frameStageLabel });

export const promptCharCount = (value: string | null | undefined): number =>
  Array.from(value?.trim() ?? "").length;

const textEncoder = new TextEncoder();

export const promptByteCount = (value: string | null | undefined): number =>
  textEncoder.encode(value?.trim() ?? "").length;

export const promptLengthError = (
  label: string,
  value: string | null | undefined,
  minChars: number,
  t: MessageTranslator,
): string | null => {
  const text = value?.trim() ?? "";
  if (!text) return t("video.chapter.framePromptEmpty", { label });
  const charCount = Array.from(text).length;
  return charCount < minChars
    ? t("video.chapter.framePromptTooShort", {
        label,
        min: minChars,
        count: charCount,
      })
    : null;
};

export const parseNumberInput = (value: unknown): number | null => {
  if (typeof value === "number") {
    return Number.isSafeInteger(value) ? value : null;
  }
  if (typeof value !== "string") return null;
  const text = value.trim();
  if (!/^\d+$/.test(text)) return null;
  const parsed = Number(text);
  return Number.isSafeInteger(parsed) ? parsed : null;
};

const strictVideoOptionInt = (
  value: unknown,
  field: string,
  min: number,
  max: number,
  t: MessageTranslator,
): number => {
  if (
    !Number.isInteger(value) ||
    (value as number) < min ||
    (value as number) > max
  ) {
    throw new Error(t("video.chapter.invalidVideoOptionRange", { field, min, max }));
  }
  return value as number;
};

const durationOptionsValidationError = (
  defaultDurationSecs: number,
  options: number[],
  t: MessageTranslator,
): string | null => {
  if (
    !Array.isArray(options) ||
    options.some(
      (duration) =>
        !Number.isInteger(duration) || duration < 1 || duration > 60,
    )
  ) {
    return t("video.chapter.invalidDurationOptionsValues");
  }
  if (
    options.length === 0 ||
    options.length > MAX_VIDEO_DURATION_OPTION_COUNT
  ) {
    return t("video.chapter.invalidDurationOptionsCount");
  }
  if (!options.includes(defaultDurationSecs)) {
    return t("video.chapter.missingDefaultDuration");
  }
  if (
    options.some(
      (duration, index) => index > 0 && duration <= options[index - 1],
    )
  ) {
    return t("video.chapter.unorderedDurationOptions");
  }
  return null;
};

export const videoOptionsValidationError = (
  options: VideoGenOptions,
  t: MessageTranslator,
): string | null => {
  if (typeof options.style !== "string") {
    return t("video.chapter.invalidVideoOptionType", { field: "style" });
  }
  if (
    !Number.isInteger(options.durationSecs) ||
    options.durationSecs < 1 ||
    options.durationSecs > 60
  ) {
    return t("video.chapter.invalidVideoOptionRange", {
      field: "durationSecs",
      min: 1,
      max: 60,
    });
  }
  const durationOptionsError = durationOptionsValidationError(
    options.durationSecs,
    options.durationOptions,
    t,
  );
  if (durationOptionsError) return durationOptionsError;
  if (!VIDEO_ASPECT_RATIOS.includes(options.aspectRatio)) {
    return t("video.chapter.invalidAspectRatio");
  }
  if (!PROMPT_OUTPUT_LANGUAGES.includes(options.promptOutputLanguage)) {
    return t("video.chapter.invalidPromptOutputLanguage");
  }
  return null;
};

export const parseVideoOptions = (
  raw: string,
  t: MessageTranslator,
): VideoGenOptions => {
  let parsedValue: unknown;
  try {
    parsedValue = JSON.parse(raw);
  } catch {
    throw new Error(t("video.chapter.invalidVideoOptionsObject"));
  }
  const parsed = asRecord(parsedValue);
  if (!parsed) throw new Error(t("video.chapter.invalidVideoOptionsObject"));
  const allowedFields = new Set([
    "style",
    "promptOutputLanguage",
    "durationSecs",
    "durationOptions",
    "aspectRatio",
  ]);
  const unknownField = Object.keys(parsed).find(
    (field) => !allowedFields.has(field),
  );
  if (unknownField) {
    throw new Error(t("video.chapter.invalidVideoOptionField", { field: unknownField }));
  }

  if (typeof parsed.style !== "string") {
    throw new Error(t("video.chapter.invalidVideoOptionType", { field: "style" }));
  }
  const style = parsed.style;
  const promptOutputLanguage =
    typeof parsed.promptOutputLanguage === "string" &&
    PROMPT_OUTPUT_LANGUAGES.includes(
      parsed.promptOutputLanguage as PromptOutputLanguage,
    )
      ? (parsed.promptOutputLanguage as PromptOutputLanguage)
      : null;
  if (!promptOutputLanguage) {
    throw new Error(t("video.chapter.invalidPromptOutputLanguage"));
  }

  const durationSecs = strictVideoOptionInt(
    parsed.durationSecs,
    "durationSecs",
    1,
    60,
    t,
  );
  if (!Array.isArray(parsed.durationOptions)) {
    throw new Error(t("video.chapter.invalidDurationOptionsArray"));
  }
  const durationOptions = parsed.durationOptions.map((value) =>
    strictVideoOptionInt(value, "durationOptions", 1, 60, t),
  );

  if (!VIDEO_ASPECT_RATIOS.includes(parsed.aspectRatio as VideoAspectRatio)) {
    throw new Error(t("video.chapter.invalidAspectRatio"));
  }

  const options: VideoGenOptions = {
    style,
    promptOutputLanguage,
    durationSecs,
    durationOptions,
    aspectRatio: parsed.aspectRatio as VideoAspectRatio,
  };
  const validationError = videoOptionsValidationError(options, t);
  if (validationError) {
    throw new Error(validationError);
  }
  return options;
};

export const serializeVideoOptions = (
  options: VideoGenOptions,
  t: MessageTranslator,
): string => {
  const validationError = videoOptionsValidationError(options, t);
  if (validationError) throw new Error(validationError);
  return JSON.stringify({
    style: options.style,
    promptOutputLanguage: options.promptOutputLanguage,
    durationSecs: options.durationSecs,
    durationOptions: options.durationOptions,
    aspectRatio: options.aspectRatio,
  });
};

export function mergeDurationOptions(
  defaultDurationSecs: number,
  options: number[],
): number[] {
  return [...new Set([...options, defaultDurationSecs])].sort((a, b) => a - b);
}

export function durationChoicesForLimits(
  limits: VideoDurationLimits,
): number[] {
  if (limits.configError) return [];
  if (limits.durationMode === "fixed") return limits.supportedDurations;
  const baseChoices = [5, 10, 15];
  return mergeDurationOptions(
    limits.defaultDurationSecs,
    baseChoices.filter(
      (duration) =>
        duration >= limits.minDurationSecs &&
        duration <= limits.maxDurationSecs,
    ),
  );
}

export function durationOptionsAllowed(
  defaultDurationSecs: number,
  options: number[],
  limits: VideoDurationLimits,
  t: MessageTranslator,
): boolean {
  return (
    durationOptionsValidationError(defaultDurationSecs, options, t) == null &&
    options.every((duration) => isDurationAllowed(duration, limits))
  );
}

export function videoDurationLimitsFromConfig(
  config: VideoGenConfig | null,
): VideoDurationLimits {
  if (!config) return GENERIC_VIDEO_DURATION_LIMITS;
  return {
    model: config.model,
    durationMode: config.duration_mode,
    defaultDurationSecs: config.default_duration_secs,
    minDurationSecs: config.min_duration_secs,
    maxDurationSecs: config.max_duration_secs,
    supportedDurations:
      config.duration_mode === "fixed"
        ? config.supported_durations_json
        : [],
    configError: null,
  };
}

export function isDurationAllowed(
  durationSecs: number,
  limits: VideoDurationLimits,
): boolean {
  if (limits.configError) return false;
  if (!Number.isInteger(durationSecs)) return false;
  if (
    durationSecs < limits.minDurationSecs ||
    durationSecs > limits.maxDurationSecs
  )
    return false;
  return (
    limits.durationMode !== "fixed" ||
    limits.supportedDurations.includes(durationSecs)
  );
}

export function durationLimitsLabel(
  limits: VideoDurationLimits,
  t: MessageTranslator,
): string {
  if (limits.configError) {
    return t("video.chapter.invalidDurationConfig", { error: limits.configError });
  }
  if (limits.durationMode === "fixed") {
    return t("video.chapter.fixedDurations", {
      values: limits.supportedDurations.join(" / "),
    });
  }
  return t("video.chapter.durationRange", {
    min: limits.minDurationSecs,
    max: limits.maxDurationSecs,
  });
}

export const parseNullableNumber = (
  raw: string,
  t: MessageTranslator,
): number | null => {
  const parsed = JSON.parse(raw) as unknown;
  if (parsed === null) return null;
  if (typeof parsed === "number" && Number.isFinite(parsed) && parsed > 0)
    return parsed;
  throw new Error(t("video.chapter.invalidPersistedVideoId"));
};

export const serializeNullableNumber = (value: number | null): string =>
  JSON.stringify(
    Number.isFinite(value) && value != null && value > 0 ? value : null,
  );

const isTerminalVideoStatus = (status: string | null | undefined): boolean => {
  if (typeof status !== "string") return false;
  const normalized = status.trim().toLowerCase();
  if (!normalized) return false;
  return (
    normalized === "completed" ||
    normalized === "failed" ||
    normalized === "cancelled"
  );
};

export const hasPendingProviderJob = (video: GeneratedVideo): boolean =>
  Boolean(video.provider_job_id) && !isTerminalVideoStatus(video.status);

export const extractSourceContentHash = (metadata: unknown): string | null => {
  const source = asRecord(metadata)?.source;
  const hash = asRecord(source)?.content_sha256;
  return typeof hash === "string" && hash.length > 0 && hash === hash.trim()
    ? hash
    : null;
};

export const isSelectableReferenceImage = (
  img: GeneratedImage,
  chapter: ChapterRow | null,
  currentContentHash: string | null,
): boolean => {
  if (isVideoFrameInputImage(img)) return false;
  if (isProjectReusableVideoMaterialImage(img)) return true;
  if (img.chapter_id == null) return true;
  if (!chapter || img.chapter_id !== chapter.id) return false;
  if (!currentContentHash) return true;
  return extractSourceContentHash(img.metadata_json) === currentContentHash;
};

export const referenceImageScopeLabel = (
  img: GeneratedImage,
  chapter: ChapterRow | null,
  currentContentHash: string | null,
  t: MessageTranslator,
): string => {
  const profileMaterialKind = profileMaterialKindOfImage(img);
  if (profileMaterialKind === "scene") return t("video.chapter.sceneMaterial");
  if (profileMaterialKind === "object") return t("video.chapter.objectMaterial");
  if (isProjectReusableVideoMaterialImage(img)) return t("video.chapter.profileMaterial");
  if (img.chapter_id == null) return t("video.chapter.projectMaterial");
  if (!chapter || img.chapter_id !== chapter.id) return t("video.chapter.otherChapter");
  if (
    currentContentHash &&
    extractSourceContentHash(img.metadata_json) !== currentContentHash
  ) {
    return t("video.chapter.fingerprintMismatch");
  }
  return t("video.chapter.chapterImage");
};
