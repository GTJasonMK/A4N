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
    return "首尾帧方案固定使用首帧和尾帧输入";
  }
  if (workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD) {
    return "分镜方案固定使用分镜板输入";
  }
  if (workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN) {
    return segmentNumber > 1
      ? "首帧续接的后续片段只能使用更早视频尾帧或独立生成首帧"
      : "首帧续接的首段固定使用首帧输入";
  }
  return "Chat 或参考媒体方案固定使用素材槽输入";
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
export const PROMPT_OUTPUT_LANGUAGE_LABELS: Record<
  PromptOutputLanguage,
  string
> = {
  zh_cn: "简体中文",
  en: "English",
};
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

export function videoFrameImageContractForPromptType(promptType: string) {
  switch (promptType) {
    case VIDEO_FRAME_PROMPT_TYPE:
      return {
        imageType: IMAGE_TYPE_VIDEO_KEY_FRAME,
        source: IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
        promptLabel: "边界帧提示词",
        frameLabel: "边界帧",
      } as const;
    case VIDEO_FIRST_FRAME_PROMPT_TYPE:
      return {
        imageType: IMAGE_TYPE_VIDEO_START_FRAME,
        source: IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
        promptLabel: "首帧提示词",
        frameLabel: "首帧",
      } as const;
    case VIDEO_STORYBOARD_PROMPT_TYPE:
      return {
        imageType: IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
        source: IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
        promptLabel: "分镜提示词",
        frameLabel: "分镜板",
      } as const;
    default:
      throw new Error(`提示词类型 ${promptType} 不支持视频帧成图`);
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

export const VIDEO_WORKFLOW_MODE_LABELS: Record<
  VideoSegmentWorkflowMode,
  string
> = {
  [VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES]: "首尾帧生成",
  [VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN]: "首帧生成",
  [VIDEO_WORKFLOW_MODE_STORYBOARD]: "分镜生成",
  [VIDEO_WORKFLOW_MODE_CHAT_VIDEO]: "Chat 生视频",
  [VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA]: "参考媒体生成",
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
): string | null => {
  const snapshot = videoProviderRequestSnapshotFromRecord(video);
  if (!snapshot) {
    return "视频记录缺少 provider 请求快照，无法直接预览生成时构造的请求";
  }
  if (snapshot.schema_version !== 1) {
    return "视频记录 provider 请求快照版本无效，无法直接预览";
  }
  for (const field of ["key", "title", "kind", "state"]) {
    if (!snapshotTextField(snapshot, field)) {
      return `视频记录 provider 请求快照缺少 ${field}，无法直接预览`;
    }
  }
  if (!asRecord(snapshot.request)) {
    return "视频记录 provider 请求快照缺少 request 对象，无法直接预览";
  }
  return null;
};

export const videoProviderRequestPreviewBlockedReason = (
  video: GeneratedVideo,
): string | null => {
  const workflowMode = videoWorkflowModeOfVideo(video);
  if (workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO) {
    return "Chat 生视频模式使用外部网站生成，没有应用内 provider 请求";
  }
  if (
    video.video_type !== VIDEO_TYPE_CHAPTER &&
    video.video_type !== VIDEO_TYPE_REFERENCE_MEDIA
  ) {
    return "裁剪片段没有独立的视频模型 provider 请求";
  }
  const snapshotBlockedReason =
    videoProviderRequestSnapshotBlockedReason(video);
  if (snapshotBlockedReason) {
    return snapshotBlockedReason;
  }
  if (workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA) {
    if (video.video_type !== VIDEO_TYPE_REFERENCE_MEDIA) {
      return "裁剪片段没有独立的视频模型 provider 请求";
    }
    return null;
  }
  if (
    workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES ||
    workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN ||
    workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD
  ) {
    if (video.video_type !== VIDEO_TYPE_CHAPTER) {
      return "裁剪片段没有独立的视频模型 provider 请求";
    }
    const snapshot = videoProviderRequestSnapshotFromRecord(video);
    const identity = asRecord(snapshot?.video_execution_identity);
    if (identity?.contract_epoch !== VIDEO_EXECUTION_CONTRACT_EPOCH) {
      return "视频记录缺少当前 v5 视频执行身份，不能按旧请求快照继续生成或预览";
    }
    return null;
  }
  return "视频记录缺少有效生成模式，无法直接预览 provider 请求";
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

export const formatVideoCreatedAt = (
  value: string | null | undefined,
): string => {
  if (!value) return "未知时间";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
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
  field = "reference_image_ids_json",
): ReferenceImageIdsState => {
  if (value == null) return { ids: [], error: null };
  if (!Array.isArray(value)) {
    return { ids: [], error: `${field} 必须是正整数数组` };
  }

  const ids: number[] = [];
  const seen = new Set<number>();
  for (const [index, item] of value.entries()) {
    if (typeof item !== "number" || !Number.isInteger(item) || item <= 0) {
      return { ids: [], error: `${field}[${index}] 必须是正整数` };
    }
    if (seen.has(item)) {
      return { ids: [], error: `${field} 包含重复 ID：${item}` };
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

const planSegmentIdentity = (
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
): Record<string, unknown> | null => {
  const metadata = asRecord(segmentPlanPrompt?.metadata_json);
  const segment = Array.isArray(metadata?.segments)
    ? metadata.segments
        .map(asRecord)
        .find((item) => item?.segment_number === segmentNumber)
    : null;
  const sourceShotId = segment?.source_shot_id;
  const segmentIndexInShot = segment?.segment_index_in_shot;
  if (
    typeof sourceShotId !== "string" ||
    !sourceShotId.trim() ||
    typeof segmentIndexInShot !== "number" ||
    !Number.isInteger(segmentIndexInShot) ||
    segmentIndexInShot <= 0
  ) {
    return null;
  }
  const identities = videoExecutionIdentityOfPrompt(segmentPlanPrompt)?.segment_identities;
  if (!Array.isArray(identities)) return null;
  return (
    identities
      .map(asRecord)
      .find(
        (identity) =>
          identity?.source_shot_id === sourceShotId &&
          identity?.segment_index_in_shot === segmentIndexInShot &&
          typeof identity?.segment_sha256 === "string" &&
          identity.segment_sha256.length > 0,
      ) ?? null
  );
};

const videoExecutionSegmentIdentityMatchesPlan = (
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
  fields: readonly string[],
): boolean => {
  return (
    videoExecutionSegmentNumberInPlanMatching(
      actual,
      segmentPlanPrompt,
      fields,
    ) ===
    segmentNumber
  );
};

const segmentPlanExecutionIdentityIsComplete = (
  prompt: ChapterVideoPrompt,
): boolean => {
  const segments = asRecord(prompt.metadata_json)?.segments;
  const identities = videoExecutionIdentityOfPrompt(prompt)?.segment_identities;
  return (
    Array.isArray(segments) &&
    segments.length > 0 &&
    Array.isArray(identities) &&
    identities.length === segments.length &&
    segments.every((value) => {
      const segmentNumber = asRecord(value)?.segment_number;
      return (
        typeof segmentNumber === "number" &&
        Number.isInteger(segmentNumber) &&
        segmentNumber > 0 &&
        planSegmentIdentity(prompt, segmentNumber) != null
      );
    })
  );
};

const videoExecutionSegmentNumberInPlanMatching = (
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  fields: readonly string[],
): number | null => {
  const planIdentity = videoExecutionIdentityOfPrompt(segmentPlanPrompt);
  if (
    !videoExecutionIdentityMatchesExpected(actual, planIdentity, fields) ||
    actual?.segment_plan_prompt_id !== segmentPlanPrompt?.id
  ) {
    return null;
  }
  const segments = asRecord(segmentPlanPrompt?.metadata_json)?.segments;
  if (!Array.isArray(segments)) return null;
  for (const value of segments) {
    const segment = asRecord(value);
    const segmentNumber = segment?.segment_number;
    if (
      typeof segmentNumber === "number" &&
      Number.isInteger(segmentNumber) &&
      segmentNumber > 0
    ) {
      const expected = planSegmentIdentity(segmentPlanPrompt, segmentNumber);
      if (
        expected &&
        VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS.every(
          (field) => actual?.[field] === expected[field],
        )
      ) {
        return segmentNumber;
      }
    }
  }
  return null;
};

export function videoExecutionSegmentNumberInPlan(
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
): number | null {
  return videoExecutionSegmentNumberInPlanMatching(
    actual,
    segmentPlanPrompt,
    VIDEO_EXECUTION_BASE_IDENTITY_FIELDS,
  );
}

export function videoExecutionSegmentNumberInPlanByLineage(
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
): number | null {
  return videoExecutionSegmentNumberInPlanMatching(
    actual,
    segmentPlanPrompt,
    VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS,
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
    ) &&
    identity?.segment_recipe_fingerprint === recipeFingerprint
  );
}

export function videoExecutionSegmentIdentityMatchesVideo(
  video: GeneratedVideo | null | undefined,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
  recipeFingerprint: string | null | undefined,
): boolean {
  const identity = videoExecutionIdentityOfVideo(video);
  return (
    Boolean(recipeFingerprint) &&
    videoExecutionSegmentIdentityMatchesPlan(
      identity,
      segmentPlanPrompt,
      segmentNumber,
      VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS,
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
): VideoSegmentCardPlan[] {
  const metadata = asRecord(prompt?.metadata_json);
  return videoSegmentCardsFromSegments(
    metadata && Array.isArray(metadata.segments) ? metadata.segments : [],
  );
}

export function videoSegmentCardsFromSegments(
  segments: unknown[],
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
        throw new Error("片段规划包含无效 segment_number");
      }
      const durationSecs = segment.duration_secs;
      const aspectRatio = segment.aspect_ratio;
      if (
        typeof durationSecs !== "number" ||
        !Number.isInteger(durationSecs) ||
        durationSecs <= 0
      ) {
        throw new Error(`片段 ${segmentNumber} duration_secs 无效`);
      }
      if (
        typeof aspectRatio !== "string" ||
        !VIDEO_ASPECT_RATIOS.includes(aspectRatio as VideoAspectRatio)
      ) {
        throw new Error(`片段 ${segmentNumber} aspect_ratio 无效`);
      }
      const title =
        typeof segment.title === "string" ? segment.title.trim() : "";
      if (!title) throw new Error(`片段 ${segmentNumber} title 无效`);
      const sceneSummary =
        typeof segment.scene_summary === "string"
          ? segment.scene_summary.trim()
          : "";
      if (!sceneSummary)
        throw new Error(`片段 ${segmentNumber} scene_summary 无效`);
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
): VideoSegmentRecipeReadResult {
  const metadata = asRecord(prompt?.metadata_json);
  const raw = metadata?.segment_recipes;
  if (!prompt) return { recipes: new Map(), error: null };
  if (raw == null) {
    return {
      recipes: new Map(),
      error: "当前片段规划缺少 segment_recipes，无法编辑或生成片段",
    };
  }
  if (!Array.isArray(raw)) {
    return { recipes: new Map(), error: "片段规划 segment_recipes 必须是数组" };
  }
  try {
    const recipes = new Map(
      parseVideoSegmentRecipeArray(raw).map((recipe) => [
        recipe.segment_number,
        recipe,
      ]),
    );
    return { recipes, error: null };
  } catch (error) {
    return {
      recipes: new Map(),
      error: extractErrorMessage(error, "读取片段执行方案失败"),
    };
  }
}

function parseVideoSegmentRecipeArray(
  raw: unknown[],
): VideoSegmentExecutionRecipe[] {
  const recipes = new Map<number, VideoSegmentExecutionRecipe>();
  for (const [index, value] of raw.entries()) {
    const recipe = asRecord(value);
    if (!recipe) throw new Error(`segment_recipes[${index}] 必须是对象`);
    if (
      recipe.schema_version !== VIDEO_SEGMENT_EXECUTION_RECIPE_SCHEMA_VERSION
    ) {
      throw new Error(`segment_recipes[${index}] schema_version 无效`);
    }
    const segmentNumber = recipe.segment_number;
    if (
      typeof segmentNumber !== "number" ||
      !Number.isInteger(segmentNumber) ||
      segmentNumber <= 0
    ) {
      throw new Error(`segment_recipes[${index}].segment_number 无效`);
    }
    if (recipes.has(segmentNumber)) {
      throw new Error(`segment_recipes 包含重复片段 ${segmentNumber}`);
    }
    if (
      typeof recipe.workflow_mode !== "string" ||
      !VIDEO_SEGMENT_WORKFLOW_MODES.includes(
        recipe.workflow_mode as VideoSegmentWorkflowMode,
      )
    ) {
      throw new Error(`片段 ${segmentNumber} workflow_mode 无效`);
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
): Record<string, unknown> {
  const sourceContentHash = extractSourceContentHash(prompt.metadata_json);
  if (sourceContentHash == null) {
    throw new Error(
      `视频帧提示词 #${prompt.id} 缺少 source.content_sha256，不能生成图片`,
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
): GeneratedImage | null {
  const frameContract = videoFrameImageContractForPromptType(prompt.prompt_type);
  const promptHash = extractSourceContentHash(prompt.metadata_json);
  const matchingImages = images.filter((image) => {
    if (videoImageGenerationPromptId(image) === prompt.id) {
      return (
        image.image_type === frameContract.imageType &&
        videoImageGenerationSource(image) === frameContract.source
      );
    }
    if (!isBoundaryFramePrompt(prompt)) return false;
    if (promptHash == null) return false;
    return (
      videoTailFrameBoundaryNumber(image) === prompt.shot_number &&
      videoImageGenerationSourceHash(image) === promptHash
    );
  });
  matchingImages.sort((a, b) => b.id - a.id);
  return matchingImages[0] ?? null;
}

export function frameImageReferencesForPrompt(
  prompt: ChapterVideoPrompt,
  images: GeneratedImage[],
  prompts: ChapterVideoPrompt[],
): VideoFrameImageReferences {
  const savedReferences = readSavedReferenceImageIds(
    prompt.reference_image_ids_json,
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
      error: `找不到上一张${prompt.shot_number - 1}号边界帧提示词，不能构造第 ${prompt.shot_number} 张边界帧图片请求`,
    };
  }
  if (previousPrompt.status === "running") {
    return {
      ids,
      note: "",
      promptReferenceOffset: 0,
      error: `第 ${prompt.shot_number - 1} 张边界帧提示词仍在运行，不能构造第 ${prompt.shot_number} 张边界帧图片请求`,
    };
  }
  const previousImage = latestVideoFrameImageForPrompt(previousPrompt, images);
  if (!previousImage) {
    return {
      ids,
      note: "",
      promptReferenceOffset: 0,
      error: `边界帧链路上下文缺少第 ${prompt.shot_number - 1} 张实际图片，不能构造第 ${prompt.shot_number} 张边界帧图片请求`,
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
        ? `上一帧图片 #${previousImage.id} 在已保存参考图中不是第 1 项，不能保证“参考图 1”语义正确`
        : undefined,
  };
}

export function boundaryFramePromptChainTo(
  targetPrompt: ChapterVideoPrompt,
  prompts: ChapterVideoPrompt[],
  operation: string,
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
        `边界帧提示词链不完整：缺少第 ${shotNumber} 张，不能${operation}到第 ${targetPrompt.shot_number} 张`,
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
): string | null {
  const text = prompt.trim();
  const compactMatch = text.match(/参考图\d+/);
  if (compactMatch) {
    return `参考图编号必须按实际输入顺序写成“参考图 1”“参考图 2”，不能写“${compactMatch[0]}”`;
  }
  const labelRegex = /参考图\s+(\d+)/g;
  let match: RegExpExecArray | null;
  while ((match = labelRegex.exec(text)) != null) {
    const raw = match[1];
    if (raw.length > 1 && raw.startsWith("0")) {
      return `参考图编号不能使用前导 0：参考图 ${raw}`;
    }
    const index = Number(raw);
    if (!Number.isSafeInteger(index) || index < 1 || index > referenceCount) {
      return referenceCount === 0
        ? "当前成图请求没有绑定参考图，提示词不能写“参考图 N”"
        : `提示词引用了不存在的参考图 ${index}；当前成图请求只会输入 ${referenceCount} 张参考图`;
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
): Record<string, unknown> {
  const trimmedPrompt = prompt.trim();
  const promptWithReferenceNote = [referencePromptNote.trim(), trimmedPrompt]
    .filter(Boolean)
    .join("\n\n");
  if (
    typeof aspectRatio !== "string" ||
    !VIDEO_ASPECT_RATIOS.includes(aspectRatio as VideoAspectRatio)
  ) {
    throw new Error("视频帧成图请求 aspect_ratio 无效");
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

export const staleVideoPromptMessage = (frameStageLabel: string): string =>
  `当前提示词基于旧正文，请先到${frameStageLabel}里生成/续跑提示词`;

export const promptCharCount = (value: string | null | undefined): number =>
  Array.from(value?.trim() ?? "").length;

const textEncoder = new TextEncoder();

export const promptByteCount = (value: string | null | undefined): number =>
  textEncoder.encode(value?.trim() ?? "").length;

export const promptLengthError = (
  label: string,
  value: string | null | undefined,
  minChars: number,
): string | null => {
  const text = value?.trim() ?? "";
  if (!text) return `${label}不能为空`;
  const charCount = Array.from(text).length;
  return charCount < minChars
    ? `${label}过短，至少需要 ${minChars} 个字符，当前 ${charCount} 个字符；请重新生成或补全后再生成`
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
): number => {
  if (
    !Number.isInteger(value) ||
    (value as number) < min ||
    (value as number) > max
  ) {
    throw new Error(`视频生成选项 ${field} 必须是 ${min}-${max} 之间的整数`);
  }
  return value as number;
};

const durationOptionsValidationError = (
  defaultDurationSecs: number,
  options: number[],
): string | null => {
  if (
    !Array.isArray(options) ||
    options.some(
      (duration) =>
        !Number.isInteger(duration) || duration < 1 || duration > 60,
    )
  ) {
    return "视频生成选项 durationOptions 必须全部是 1-60 之间的整数";
  }
  if (
    options.length === 0 ||
    options.length > MAX_VIDEO_DURATION_OPTION_COUNT
  ) {
    return "视频生成选项 durationOptions 必须选择 1-12 个候选时长";
  }
  if (!options.includes(defaultDurationSecs)) {
    return "视频生成选项 durationOptions 必须包含默认时长";
  }
  if (
    options.some(
      (duration, index) => index > 0 && duration <= options[index - 1],
    )
  ) {
    return "视频生成选项 durationOptions 必须严格递增且不能重复";
  }
  return null;
};

export const videoOptionsValidationError = (
  options: VideoGenOptions,
): string | null => {
  if (typeof options.style !== "string") {
    return "视频生成选项 style 必须是字符串";
  }
  if (
    !Number.isInteger(options.durationSecs) ||
    options.durationSecs < 1 ||
    options.durationSecs > 60
  ) {
    return "视频生成选项 durationSecs 必须是 1-60 之间的整数";
  }
  const durationOptionsError = durationOptionsValidationError(
    options.durationSecs,
    options.durationOptions,
  );
  if (durationOptionsError) return durationOptionsError;
  if (!VIDEO_ASPECT_RATIOS.includes(options.aspectRatio)) {
    return "视频生成选项 aspectRatio 无效";
  }
  if (!PROMPT_OUTPUT_LANGUAGES.includes(options.promptOutputLanguage)) {
    return "视频生成选项 promptOutputLanguage 无效";
  }
  return null;
};

export const parseVideoOptions = (raw: string): VideoGenOptions => {
  const parsed = asRecord(JSON.parse(raw));
  if (!parsed) throw new Error("视频生成选项必须是对象");
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
    throw new Error(`视频生成选项包含未知字段：${unknownField}`);
  }

  if (typeof parsed.style !== "string") {
    throw new Error("视频生成选项 style 必须是字符串");
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
    throw new Error("视频生成选项 promptOutputLanguage 无效");
  }

  const durationSecs = strictVideoOptionInt(
    parsed.durationSecs,
    "durationSecs",
    1,
    60,
  );
  if (!Array.isArray(parsed.durationOptions)) {
    throw new Error("视频生成选项 durationOptions 必须是数组");
  }
  const durationOptions = parsed.durationOptions.map((value) =>
    strictVideoOptionInt(value, "durationOptions", 1, 60),
  );

  if (!VIDEO_ASPECT_RATIOS.includes(parsed.aspectRatio as VideoAspectRatio)) {
    throw new Error("视频生成选项 aspectRatio 无效");
  }

  const options: VideoGenOptions = {
    style,
    promptOutputLanguage,
    durationSecs,
    durationOptions,
    aspectRatio: parsed.aspectRatio as VideoAspectRatio,
  };
  const validationError = videoOptionsValidationError(options);
  if (validationError) {
    throw new Error(validationError);
  }
  return options;
};

export const serializeVideoOptions = (options: VideoGenOptions): string => {
  const validationError = videoOptionsValidationError(options);
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
): boolean {
  return (
    durationOptionsValidationError(defaultDurationSecs, options) == null &&
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

export function durationLimitsLabel(limits: VideoDurationLimits): string {
  if (limits.configError) return `配置异常：${limits.configError}`;
  if (limits.durationMode === "fixed") {
    return `固定 ${limits.supportedDurations.join(" / ")} 秒`;
  }
  return `范围 ${limits.minDurationSecs}-${limits.maxDurationSecs} 秒`;
}

export const parseNullableNumber = (raw: string): number | null => {
  const parsed = JSON.parse(raw) as unknown;
  if (parsed === null) return null;
  if (typeof parsed === "number" && Number.isFinite(parsed) && parsed > 0)
    return parsed;
  throw new Error("持久化视频 ID 必须是正数或 null");
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
): string => {
  const profileMaterialKind = profileMaterialKindOfImage(img);
  if (profileMaterialKind === "scene") return "场景素材";
  if (profileMaterialKind === "object") return "物品素材";
  if (isProjectReusableVideoMaterialImage(img)) return "档案素材";
  if (img.chapter_id == null) return "项目素材";
  if (!chapter || img.chapter_id !== chapter.id) return "其他章节";
  if (
    currentContentHash &&
    extractSourceContentHash(img.metadata_json) !== currentContentHash
  ) {
    return "指纹不一致";
  }
  return "本章图片";
};
