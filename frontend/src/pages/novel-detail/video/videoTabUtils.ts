import type { GeneratedImage } from "../../../api/image";
import { extractErrorMessage } from "../../../api";
import type {
  ChapterVideoPrompt,
  GeneratedVideo,
  PromptOutputLanguage,
  VideoDurationMode,
  VideoGenConfig,
  VideoWorkflowMode,
} from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import type { ChapterRow } from "../shared";
import { DEFAULT_VIDEO_STYLE } from "../styleTemplates";

export type ChapterVideoWorkflowMode = Extract<
  VideoWorkflowMode,
  | "boundary_frames"
  | "first_frame_chain"
  | "storyboard"
  | "chat_video"
  | "reference_media"
>;

export interface PromptDraft {
  visual_prompt: string;
  motion_prompt: string;
  negative_prompt: string;
  duration_secs: number;
}

export type VideoAspectRatio = "21:9" | "16:9" | "4:3" | "1:1" | "3:4" | "9:16";

export interface VideoGenOptions {
  style: string;
  promptOutputLanguage: PromptOutputLanguage;
  promptBatchSize: number;
  durationSecs: number;
  durationOptions: number[];
  aspectRatio: VideoAspectRatio;
  storageError?: string | null;
}

interface PersistedVideoImageIdSelection {
  ids: number[];
  storageError: string | null;
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

export interface ChapterAnalysisSummary {
  summaries?: {
    compressed?: string;
    one_line?: string;
  };
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
export const NEWAPI_VIDEO_BACKEND = "newapi-video";
export const CLIPROXY_OPENAI_VIDEO_BACKEND = "cliproxy-openai-video";
export const MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES = 7;
// Rust command contract uses i32; UI business limits are derived from remaining prompt count.
export const MAX_VIDEO_PROMPT_BATCH_SIZE = 2_147_483_647;
export const VIDEO_ASPECT_RATIOS: readonly VideoAspectRatio[] = [
  "21:9",
  "16:9",
  "4:3",
  "1:1",
  "3:4",
  "9:16",
];
export const PROMPT_OUTPUT_LANGUAGES: readonly PromptOutputLanguage[] = [
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
export const VIDEO_FINAL_PROMPT_TYPE = "video_prompt";
export const VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION =
  "video_execution_contract_v5";
export const VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION =
  "boundary_frame_shared_chain_v3";
export const VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION = "first_frame_chain_execution_v4";
export const VIDEO_CHAT_VIDEO_CONTRACT_VERSION = "chat_video_execution_v4";
export const VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION = "storyboard_execution_unit_v2";
export const VIDEO_STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION =
  "storyboard_execution_video_v2";
export const VIDEO_EXECUTION_CONTRACT_EPOCH = 5;
const VIDEO_FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT = 1;
export const VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS = 80;

export type VideoFrameImageRole = "key_frame" | "start_frame" | "storyboard_frame";

const VIDEO_FRAME_IMAGE_TYPES: Array<{
  role: VideoFrameImageRole;
  imageType: string;
  label: string;
}> = [
  { role: "key_frame", imageType: "video_key_frame", label: "边界帧" },
  { role: "start_frame", imageType: "video_start_frame", label: "首帧" },
  {
    role: "storyboard_frame",
    imageType: "video_storyboard_frame",
    label: "分镜板",
  },
];

export const DEFAULT_VIDEO_OPTIONS: VideoGenOptions = {
  style: DEFAULT_VIDEO_STYLE,
  promptOutputLanguage: "zh_cn",
  promptBatchSize: 1,
  durationSecs: 6,
  durationOptions: [6],
  aspectRatio: "16:9",
  storageError: null,
};

export const EMPTY_VIDEO_IMAGE_ID_SELECTION: PersistedVideoImageIdSelection = {
  ids: [],
  storageError: null,
};

export const VIDEO_WORKFLOW_MODES: readonly ChapterVideoWorkflowMode[] = [
  "boundary_frames",
  "first_frame_chain",
  "storyboard",
  "chat_video",
  "reference_media",
];

export const DEFAULT_VIDEO_WORKFLOW_MODE: ChapterVideoWorkflowMode = "chat_video";

export const VIDEO_WORKFLOW_MODE_LABELS: Record<ChapterVideoWorkflowMode, string> = {
  boundary_frames: "首尾帧生成",
  first_frame_chain: "首帧生成",
  storyboard: "分镜生成",
  chat_video: "Chat 生视频",
  reference_media: "参考媒体生成",
};

export function videoWorkflowModeOfVideo(
  video: GeneratedVideo,
): ChapterVideoWorkflowMode | null {
  const raw = video.workflow_mode;
  if (
    typeof raw === "string" &&
    VIDEO_WORKFLOW_MODES.includes(raw as ChapterVideoWorkflowMode)
  ) {
    return raw as ChapterVideoWorkflowMode;
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
  if (workflowMode === "chat_video") {
    return "Chat 生视频模式使用外部网站生成，没有应用内 provider 请求";
  }
  if (
    video.video_type !== "chapter_video" &&
    video.video_type !== "reference_media_video"
  ) {
    return "裁剪片段没有独立的视频模型 provider 请求";
  }
  const snapshotBlockedReason =
    videoProviderRequestSnapshotBlockedReason(video);
  if (snapshotBlockedReason) {
    return snapshotBlockedReason;
  }
  if (workflowMode === "reference_media") {
    if (video.video_type !== "reference_media_video") {
      return "裁剪片段没有独立的视频模型 provider 请求";
    }
    return null;
  }
  if (
    workflowMode === "boundary_frames" ||
    workflowMode === "first_frame_chain" ||
    workflowMode === "storyboard"
  ) {
    if (video.video_type !== "chapter_video") {
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

const VIDEO_FRAME_IMAGE_TYPE_BY_ROLE: Record<VideoFrameImageRole, string> =
  Object.fromEntries(
    VIDEO_FRAME_IMAGE_TYPES.map((item) => [item.role, item.imageType]),
  ) as Record<VideoFrameImageRole, string>;

const VIDEO_FRAME_IMAGE_LABEL_BY_ROLE: Record<VideoFrameImageRole, string> =
  Object.fromEntries(
    VIDEO_FRAME_IMAGE_TYPES.map((item) => [item.role, item.label]),
  ) as Record<VideoFrameImageRole, string>;

const VIDEO_FRAME_IMAGE_ROLE_BY_TYPE: Record<string, VideoFrameImageRole> =
  Object.fromEntries(
    VIDEO_FRAME_IMAGE_TYPES.map((item) => [item.imageType, item.role]),
  ) as Record<string, VideoFrameImageRole>;

const VIDEO_FRAME_GENERATION_SOURCES = new Set([
  "video_boundary_frame_prompt",
  "video_first_frame_prompt",
  "video_storyboard_prompt",
  "video_tail_frame_capture",
]);

function parseVideoFrameImageRole(value: unknown): VideoFrameImageRole | null {
  if (typeof value !== "string") return null;
  const normalized = value.trim();
  return VIDEO_FRAME_IMAGE_TYPES.some((item) => item.role === normalized)
    ? (normalized as VideoFrameImageRole)
    : null;
}

export function videoImageTypeForFrameRole(role: VideoFrameImageRole): string {
  return VIDEO_FRAME_IMAGE_TYPE_BY_ROLE[role];
}

function videoFrameImageRoleFromImageType(
  imageType: string | null | undefined,
): VideoFrameImageRole | null {
  const normalized = imageType?.trim();
  return normalized
    ? (VIDEO_FRAME_IMAGE_ROLE_BY_TYPE[normalized] ?? null)
    : null;
}

export function videoFrameImageRoleOfImage(
  img: GeneratedImage,
): VideoFrameImageRole | null {
  const metadataRole = parseVideoFrameImageRole(
    asRecord(img.metadata_json)?.video_frame_role,
  );
  return metadataRole ?? videoFrameImageRoleFromImageType(img.image_type);
}

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
  return typeof raw === "string" && raw.trim() ? raw.trim() : null;
};

export const videoImageGenerationSourceHash = (
  img: GeneratedImage,
): string | null => {
  const raw = videoImageGenerationContext(img)?.source_content_sha256;
  return typeof raw === "string" && raw.trim() ? raw.trim() : null;
};

export function videoTailFrameBoundaryNumber(
  img: GeneratedImage,
): number | null {
  if (videoImageGenerationSource(img) !== "video_tail_frame_capture")
    return null;
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
    source === "video_boundary_frame_prompt" ||
    source === "video_first_frame_prompt" ||
    source === "video_storyboard_prompt"
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
  const source = videoImageGenerationSource(img);
  return (
    (typeof source === "string" &&
      VIDEO_FRAME_GENERATION_SOURCES.has(source.trim())) ||
    videoFrameImageRoleOfImage(img) != null ||
    img.image_type.trim() === "video_tail_frame"
  );
}

export function isChapterVideoMaterialImage(
  img: GeneratedImage,
  chapter: ChapterRow | null,
): boolean {
  if (!chapter || img.chapter_id !== chapter.id) return false;
  const metadata = asRecord(img.metadata_json);
  if (isVideoFrameInputImage(img)) return false;
  return (
    metadata?.chapter_video_material === true ||
    metadata?.video_library_member === true
  );
}

export function isProjectReusableVideoMaterialImage(
  img: GeneratedImage,
): boolean {
  if (isVideoFrameInputImage(img)) return false;
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
  if (type === "scene_material") {
    if (explicitKind != null && explicitKind !== "scene") return null;
    const id =
      explicitKind === "scene" ? metadataNumber(metadata?.profile_id) : null;
    return id == null ? null : "scene";
  }
  if (type === "object_material") {
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

export function videoFrameImageRoleLabel(
  role: VideoFrameImageRole | null,
  fallback = "素材",
): string {
  return role ? VIDEO_FRAME_IMAGE_LABEL_BY_ROLE[role] : fallback;
}

export function isFinalVideoPrompt(
  promptType: string | null | undefined,
): boolean {
  return promptType === VIDEO_FINAL_PROMPT_TYPE;
}

export function promptOutputLanguageOfPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
): PromptOutputLanguage {
  const raw = asRecord(prompt?.metadata_json)?.prompt_output_language;
  return typeof raw === "string" &&
    PROMPT_OUTPUT_LANGUAGES.includes(raw as PromptOutputLanguage)
    ? (raw as PromptOutputLanguage)
    : DEFAULT_VIDEO_OPTIONS.promptOutputLanguage;
}

type VideoFrameImageGenerationSource =
  | "video_boundary_frame_prompt"
  | "video_first_frame_prompt"
  | "video_storyboard_prompt";

export function promptSourcePromptType(
  prompt: ChapterVideoPrompt | null | undefined,
): string | null {
  const raw = asRecord(prompt?.metadata_json)?.source_prompt_type;
  return typeof raw === "string" && raw.trim() ? raw.trim() : null;
}

export function videoWorkflowModeOfPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
): ChapterVideoWorkflowMode | null {
  const raw = prompt?.workflow_mode;
  return VIDEO_WORKFLOW_MODES.includes(raw as ChapterVideoWorkflowMode)
    ? (raw as ChapterVideoWorkflowMode)
    : null;
}

export const providerIsCliProxyVideo = (
  backend: string | null | undefined,
): boolean => backend?.trim() === CLIPROXY_OPENAI_VIDEO_BACKEND;

export const providerIsNewApiVideo = (
  backend: string | null | undefined,
): boolean => backend?.trim() === NEWAPI_VIDEO_BACKEND;

export const cliproxyGrokReferenceModeFromProviderOptions = (
  value: unknown,
): "reference" | "first_frame" | null => {
  const mode = asRecord(value)?.grok_reference_mode;
  if (mode == null) return "reference";
  if (typeof mode !== "string") return null;
  const normalized = mode.trim();
  if (normalized === "reference" || normalized === "first_frame") return normalized;
  return null;
};

export const videoPromptSupportsProviderGeneration = (
  prompt: ChapterVideoPrompt,
): boolean => {
  const workflowMode = videoWorkflowModeOfPrompt(prompt);
  return (
    workflowMode === "boundary_frames" ||
    workflowMode === "first_frame_chain" ||
    workflowMode === "storyboard"
  );
};

export const videoPromptProviderMediaBlockedReason = (
  prompt: ChapterVideoPrompt,
  referenceImageCount: number,
  materialReferenceImageCount: number,
  providerBackend: string | null | undefined,
  providerOptionsJson?: unknown,
): string | null => {
  const workflowMode = videoWorkflowModeOfPrompt(prompt);
  if (!videoPromptSupportsProviderGeneration(prompt)) return null;
  if (providerIsNewApiVideo(providerBackend)) {
    if (workflowMode === "storyboard") {
      return `视频提示词 #${prompt.id} 当前是分镜模式；New API 视频接口不支持 reference_images，不能发送分镜板，请切换视频 provider`;
    }
    if (workflowMode === "boundary_frames") {
      return `视频提示词 #${prompt.id} 当前是首尾帧模式；New API 视频接口只有首帧 image 输入槽，不能发送尾帧，请改用首帧串联模式或切换视频 provider`;
    }
    if (materialReferenceImageCount > 0) {
      return `视频提示词 #${prompt.id} 当前绑定了 ${materialReferenceImageCount} 张素材参考图；New API 视频接口不支持素材参考图输入，请移除素材参考图或切换视频 provider`;
    }
    return null;
  }
  if (!providerIsCliProxyVideo(providerBackend)) return null;
  const grokMode = cliproxyGrokReferenceModeFromProviderOptions(providerOptionsJson);
  if (grokMode == null) {
    return "当前 CLIProxy/Grok 视频 provider_options_json.grok_reference_mode 无效，请到视频模型配置中修正";
  }
  if (grokMode === "reference") {
    if (referenceImageCount > MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES) {
      return `视频提示词 #${prompt.id} 当前绑定了 ${referenceImageCount} 张参考图；CLIProxy/Grok 参考模式最多支持 ${MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES} 张`;
    }
    if (referenceImageCount > 0 && prompt.duration_secs > 10) {
      return `视频提示词 #${prompt.id} 当前使用 CLIProxy/Grok 参考模式和 ${referenceImageCount} 张参考图；该模式最大时长为 10 秒，请降低时长或移除参考图`;
    }
    return null;
  }
  if (workflowMode === "storyboard") {
    return `视频提示词 #${prompt.id} 当前是分镜模式；CLIProxy/Grok 首帧模式不能发送分镜板 reference_images，请切换为参考模式或切换视频 provider`;
  }
  if (workflowMode === "boundary_frames") {
    return `视频提示词 #${prompt.id} 当前是首尾帧模式；CLIProxy/Grok 首帧模式只有 input_reference 输入槽，不能发送尾帧，请切换为参考模式、改用首帧串联模式或切换视频 provider`;
  }
  if (materialReferenceImageCount > 0) {
    return `视频提示词 #${prompt.id} 当前绑定了 ${materialReferenceImageCount} 张素材参考图；CLIProxy/Grok 首帧模式不能同时发送素材参考图，请移除素材参考图或切换为参考模式`;
  }
  return null;
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
  "shooting_script_identity",
  "projection_sha256",
  "segment_plan_sha256",
  "unit_plan_sha256",
  "source_content_sha256",
  "style_sha256",
  "duration_secs",
  "aspect_ratio",
] as const;

export function videoExecutionIdentityOfPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
): Record<string, unknown> | null {
  const identity = asRecord(asRecord(prompt?.metadata_json)?.video_execution_identity);
  if (identity?.contract_epoch !== VIDEO_EXECUTION_CONTRACT_EPOCH) return null;
  return identity;
}

export function videoExecutionIdentityMatches(
  prompt: ChapterVideoPrompt | null | undefined,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
): boolean {
  const actual = videoExecutionIdentityOfPrompt(prompt);
  const expected = videoExecutionIdentityOfPrompt(segmentPlanPrompt);
  if (!actual || !expected) return false;
  return VIDEO_EXECUTION_BASE_IDENTITY_FIELDS.every(
    (field) => JSON.stringify(actual[field]) === JSON.stringify(expected[field]),
  );
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

export function expectedFrameCountFromSegmentPlanPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
  workflowMode: ChapterVideoWorkflowMode,
): number | null {
  const metadata = asRecord(prompt?.metadata_json);
  if (!metadata) return null;
  const segments = Array.isArray(metadata.segments) ? metadata.segments : null;
  if (!segments) return null;
  if (workflowMode === "first_frame_chain") {
    return VIDEO_FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT;
  }
  const unitCount = videoExecutionUnitCount(segments);
  if (workflowMode === "storyboard") return unitCount;
  return unitCount === 0 ? 0 : unitCount + 1;
}

export function expectedVideoPromptCountFromSegmentPlanPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
): number | null {
  const metadata = asRecord(prompt?.metadata_json);
  if (!metadata) return null;
  const segments = Array.isArray(metadata.segments) ? metadata.segments : null;
  if (!segments) return null;
  return videoExecutionUnitCount(segments);
}

function videoExecutionUnitCount(segments: unknown[]): number {
  let actionBridgeCount = 0;
  for (let index = 0; index + 1 < segments.length; index += 1) {
    const transition = asRecord(asRecord(segments[index])?.transition_to_next);
    const preservedAction = transition?.preserved_action;
    if (typeof preservedAction === "string" && trimmedText(preservedAction)) {
      actionBridgeCount += 1;
    }
  }
  return segments.length + actionBridgeCount;
}

export function videoPromptUnitFrameNumbers(
  prompt: ChapterVideoPrompt | null | undefined,
): { startFrameNumber: number | null; endFrameNumber: number | null } {
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

export function boundaryFramePromptsAreComplete(
  prompts: Array<Pick<ChapterVideoPrompt, "shot_number">>,
  expectedCount: number | null,
): boolean {
  if (
    !Number.isInteger(expectedCount) ||
    expectedCount == null ||
    expectedCount <= 0
  ) {
    return false;
  }
  if (prompts.length !== expectedCount) return false;
  const shotNumbers = new Set(prompts.map((prompt) => prompt.shot_number));
  for (let shotNumber = 1; shotNumber <= expectedCount; shotNumber += 1) {
    if (!shotNumbers.has(shotNumber)) return false;
  }
  return true;
}

export function boundaryFrameStageMessage(
  segmentPlanPrompt: ChapterVideoPrompt | null,
  framePromptCount: number,
  expectedCount: number | null,
  workflowMode: ChapterVideoWorkflowMode = "boundary_frames",
): string | null {
  const label =
    workflowMode === "first_frame_chain"
      ? "首帧提示词"
      : workflowMode === "storyboard"
        ? "分镜提示词"
        : "边界帧提示词";
  if (!segmentPlanPrompt) return `请先生成${label}`;
  if (expectedCount == null) return `片段规划缺少片段数量，请重新生成${label}`;
  if (framePromptCount !== expectedCount) {
    return `当前章节${label}不完整：需要 ${expectedCount} 条，当前 ${framePromptCount} 条`;
  }
  return null;
}

export function videoFrameImageMetadata(
  role: VideoFrameImageRole,
  metadata: unknown = null,
): Record<string, unknown> {
  const base = asRecord(metadata);
  return {
    ...(base ?? (metadata == null ? {} : { value: metadata })),
    video_frame_role: role,
  };
}

export function buildVideoFrameImageTraceMetadata(
  prompt: ChapterVideoPrompt,
  source: VideoFrameImageGenerationSource,
  context: {
    projectId: number;
    chapter: ChapterRow | null;
    currentContentHash: string | null;
    requestPreviewNote?: string | null;
  },
): Record<string, unknown> {
  const requestPreviewNote = context.requestPreviewNote?.trim();
  return {
    generation_context: {
      source,
      surface: "novel_detail.video",
      project_id: context.projectId,
      chapter_id: context.chapter?.id ?? prompt.chapter_id ?? null,
      chapter_number: context.chapter?.chapter_number ?? null,
      prompt_id: prompt.id,
      prompt_type: prompt.prompt_type,
      shot_number: prompt.shot_number,
      title: prompt.title,
      scene_summary: prompt.scene_summary,
      source_content_sha256:
        extractPromptSourceHash(prompt.metadata_json) ??
        context.currentContentHash,
      ...(requestPreviewNote
        ? { request_preview_note: requestPreviewNote }
        : {}),
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

export function buildPreviousFrameReferencePromptNote(
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

export function mergeFrameReferenceIds(
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

export function referenceLabelOffsetForPreviousFrame(
  savedReferenceIds: number[],
  previousFrameImageId: number | null,
): number {
  return previousFrameImageId != null && savedReferenceIds[0] !== previousFrameImageId ? 1 : 0;
}

export interface VideoFrameImageReferences {
  ids: number[];
  note: string;
  promptReferenceOffset: number;
  error?: string;
}

export function isBoundaryFramePrompt(
  prompt: Pick<ChapterVideoPrompt, "prompt_type">,
): boolean {
  return (
    prompt.prompt_type === VIDEO_FRAME_PROMPT_TYPE ||
    prompt.prompt_type === VIDEO_CHAT_FRAME_PROMPT_TYPE
  );
}

function promptSourceContentHash(prompt: ChapterVideoPrompt): string | null {
  const source = asRecord(prompt.metadata_json)?.source;
  const hash = asRecord(source)?.content_sha256;
  return typeof hash === "string" && hash.trim() ? hash.trim() : null;
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
  const promptHash = promptSourceContentHash(prompt);
  const matchingImages = images.filter((image) => {
    if (videoImageGenerationPromptId(image) === prompt.id) return true;
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
  const savedReferences = readSavedReferenceImageIds(prompt.reference_image_ids_json);
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
    promptReferenceOffset: referenceLabelOffsetForPreviousFrame(ids, previousImage.id),
    error: previousIndex > 0
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
  for (let shotNumber = 1; shotNumber <= targetPrompt.shot_number; shotNumber += 1) {
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
  role: VideoFrameImageRole,
  prompt: string,
  chapterId: number,
  metadata: unknown = null,
  referenceImageIds: number[] = [],
  referencePromptNote = "",
): Record<string, unknown> {
  const trimmedPrompt = prompt.trim();
  const promptWithReferenceNote = [referencePromptNote.trim(), trimmedPrompt]
    .filter(Boolean)
    .join("\n\n");
  const promptGuard =
    role === "storyboard_frame"
      ? VIDEO_STORYBOARD_FRAME_IMAGE_PROMPT_GUARD
      : VIDEO_REAL_FRAME_IMAGE_PROMPT_GUARD;
  return {
    prompt: `${promptWithReferenceNote}\n\n${promptGuard}`,
    image_type: videoImageTypeForFrameRole(role),
    chapter_id: chapterId,
    metadata_json: videoFrameImageMetadata(role, metadata),
    reference_image_ids: referenceImageIds,
  };
}

export const trimmedText = (value: string | null | undefined): string => {
  return value == null ? "" : value.trim();
};

export const staleVideoPromptMessage = (frameStageLabel: string): string =>
  `当前提示词基于旧正文，请先到${frameStageLabel}里生成/续跑提示词`;

export const firstNonEmptyText = (
  ...values: Array<string | null | undefined>
): string => {
  for (const value of values) {
    const text = value?.trim();
    if (text) return text;
  }
  return "";
};

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

export const videoOptionsValidationError = (
  options: VideoGenOptions,
): string | null => {
  if (
    !Number.isInteger(options.promptBatchSize) ||
    options.promptBatchSize < 1 ||
    options.promptBatchSize > MAX_VIDEO_PROMPT_BATCH_SIZE
  ) {
    return "视频生成选项 promptBatchSize 必须是正整数";
  }
  if (
    !Number.isInteger(options.durationSecs) ||
    options.durationSecs < 1 ||
    options.durationSecs > 60
  ) {
    return "视频生成选项 durationSecs 必须是 1-60 之间的整数";
  }
  if (
    !Array.isArray(options.durationOptions) ||
    options.durationOptions.some(
      (duration) =>
        !Number.isInteger(duration) || duration < 1 || duration > 60,
    )
  ) {
    return "视频生成选项 durationOptions 必须全部是 1-60 之间的整数";
  }
  const durationOptions = normalizeDurationOptions(
    options.durationSecs,
    options.durationOptions,
  );
  if (
    durationOptions.length === 0 ||
    durationOptions.length > MAX_VIDEO_DURATION_OPTION_COUNT
  ) {
    return "视频生成选项 durationOptions 必须选择 1-12 个候选时长";
  }
  if (!VIDEO_ASPECT_RATIOS.includes(options.aspectRatio)) {
    return "视频生成选项 aspectRatio 无效";
  }
  if (!PROMPT_OUTPUT_LANGUAGES.includes(options.promptOutputLanguage)) {
    return "视频生成选项 promptOutputLanguage 无效";
  }
  return null;
};

const parseVideoOptionsStrict = (raw: string): VideoGenOptions => {
  const parsed = asRecord(JSON.parse(raw));
  if (!parsed) throw new Error("视频生成选项必须是对象");

  if (typeof parsed.style !== "string") {
    throw new Error("视频生成选项 style 必须是字符串");
  }
  const style = parsed.style;
  const promptOutputLanguage =
    parsed.promptOutputLanguage === undefined
      ? DEFAULT_VIDEO_OPTIONS.promptOutputLanguage
      : typeof parsed.promptOutputLanguage === "string" &&
          PROMPT_OUTPUT_LANGUAGES.includes(
            parsed.promptOutputLanguage as PromptOutputLanguage,
          )
        ? (parsed.promptOutputLanguage as PromptOutputLanguage)
        : null;
  if (!promptOutputLanguage) {
    throw new Error("视频生成选项 promptOutputLanguage 无效");
  }

  const promptBatchSize =
    parsed.promptBatchSize === undefined
      ? DEFAULT_VIDEO_OPTIONS.promptBatchSize
      : strictVideoOptionInt(
          parsed.promptBatchSize,
          "promptBatchSize",
          1,
          MAX_VIDEO_PROMPT_BATCH_SIZE,
        );
  const durationSecs = strictVideoOptionInt(
    parsed.durationSecs,
    "durationSecs",
    1,
    60,
  );
  const durationOptions = normalizeDurationOptions(
    durationSecs,
    Array.isArray(parsed.durationOptions)
      ? parsed.durationOptions.map((value) =>
          strictVideoOptionInt(value, "durationOptions", 1, 60),
        )
      : [durationSecs],
  );

  if (!VIDEO_ASPECT_RATIOS.includes(parsed.aspectRatio as VideoAspectRatio)) {
    throw new Error("视频生成选项 aspectRatio 无效");
  }

  return {
    style,
    promptOutputLanguage,
    promptBatchSize,
    durationSecs,
    durationOptions,
    aspectRatio: parsed.aspectRatio as VideoAspectRatio,
    storageError: null,
  };
};

export const parseVideoOptions = (raw: string): VideoGenOptions => {
  try {
    return parseVideoOptionsStrict(raw);
  } catch (err) {
    return {
      ...DEFAULT_VIDEO_OPTIONS,
      storageError: `视频生成选项本地存储损坏：${extractErrorMessage(err, "")}`,
    };
  }
};

export const serializeVideoOptions = (options: VideoGenOptions): string => {
  return JSON.stringify({
    style: options.style,
    promptOutputLanguage: options.promptOutputLanguage,
    promptBatchSize: options.promptBatchSize,
    durationSecs: options.durationSecs,
    durationOptions: normalizeDurationOptions(
      options.durationSecs,
      options.durationOptions,
    ),
    aspectRatio: options.aspectRatio,
  });
};

export function normalizeDurationOptions(
  defaultDurationSecs: number,
  options: number[],
): number[] {
  const seen = new Set<number>();
  const normalized: number[] = [];
  for (const duration of [...options, defaultDurationSecs]) {
    if (
      !Number.isInteger(duration) ||
      duration < 1 ||
      duration > 60 ||
      seen.has(duration)
    ) {
      continue;
    }
    seen.add(duration);
    normalized.push(duration);
  }
  return normalized.sort((a, b) => a - b);
}

export function durationChoicesForLimits(
  limits: VideoDurationLimits,
): number[] {
  if (limits.configError) return [];
  if (limits.durationMode === "fixed") return limits.supportedDurations;
  const baseChoices = [5, 10, 15];
  return normalizeDurationOptions(
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
  const normalized = normalizeDurationOptions(defaultDurationSecs, options);
  return (
    normalized.length > 0 &&
    normalized.every((duration) => isDurationAllowed(duration, limits))
  );
}

export const parseVideoWorkflowMode = (
  raw: string,
): ChapterVideoWorkflowMode => {
  const parsed = JSON.parse(raw) as unknown;
  if (
    typeof parsed === "string" &&
    VIDEO_WORKFLOW_MODES.includes(parsed as ChapterVideoWorkflowMode)
  ) {
    return parsed as ChapterVideoWorkflowMode;
  }
  throw new Error("视频生成模式无效");
};

export const serializeVideoWorkflowMode = (
  mode: ChapterVideoWorkflowMode,
): string => JSON.stringify(mode);

export function videoDurationLimitsFromConfig(
  config: VideoGenConfig | null,
): VideoDurationLimits {
  if (!config) return GENERIC_VIDEO_DURATION_LIMITS;
  const invalid = (message: string): VideoDurationLimits => ({
    ...GENERIC_VIDEO_DURATION_LIMITS,
    model: config.model,
    configError: message,
  });
  type DurationIntRead =
    { ok: true; value: number } | { ok: false; error: string };
  const readDurationInt = (value: unknown, field: string): DurationIntRead => {
    if (
      typeof value !== "number" ||
      !Number.isInteger(value) ||
      value < 1 ||
      value > 60
    ) {
      return { ok: false, error: `视频配置 ${field} 必须在 1-60 之间` };
    }
    return { ok: true, value };
  };
  const rawDurationMode = (config as { duration_mode?: unknown }).duration_mode;
  if (rawDurationMode !== "range" && rawDurationMode !== "fixed") {
    return invalid(
      `视频配置 duration_mode 无效：${String(rawDurationMode ?? "<missing>")}`,
    );
  }
  const defaultDurationSecsRead = readDurationInt(
    (config as { default_duration_secs?: unknown }).default_duration_secs,
    "default_duration_secs",
  );
  if (!defaultDurationSecsRead.ok)
    return invalid(defaultDurationSecsRead.error);
  const defaultDurationSecs = defaultDurationSecsRead.value;
  const minDurationSecsRead = readDurationInt(
    (config as { min_duration_secs?: unknown }).min_duration_secs,
    "min_duration_secs",
  );
  if (!minDurationSecsRead.ok) return invalid(minDurationSecsRead.error);
  const minDurationSecs = minDurationSecsRead.value;
  const maxDurationSecsRead = readDurationInt(
    (config as { max_duration_secs?: unknown }).max_duration_secs,
    "max_duration_secs",
  );
  if (!maxDurationSecsRead.ok) return invalid(maxDurationSecsRead.error);
  const maxDurationSecs = maxDurationSecsRead.value;

  if (rawDurationMode === "range") {
    if (config.supported_durations_json != null) {
      return invalid("视频配置 range 模式不应包含 supported_durations_json");
    }
    if (minDurationSecs > maxDurationSecs) {
      return invalid("视频配置 min_duration_secs 不能大于 max_duration_secs");
    }
    if (
      defaultDurationSecs < minDurationSecs ||
      defaultDurationSecs > maxDurationSecs
    ) {
      return invalid(
        `视频配置 default_duration_secs 必须在模型支持时长 ${minDurationSecs}-${maxDurationSecs} 秒之间`,
      );
    }
    return {
      model: config.model,
      durationMode: "range",
      defaultDurationSecs,
      minDurationSecs,
      maxDurationSecs,
      supportedDurations: [],
      configError: null,
    };
  }

  if (!Array.isArray(config.supported_durations_json)) {
    return invalid(
      "视频配置 fixed 模式必须提供 supported_durations_json 正整数数组",
    );
  }
  const supportedDurations: number[] = [];
  const seen = new Set<number>();
  for (const [index, value] of config.supported_durations_json.entries()) {
    const duration = readDurationInt(
      value,
      `supported_durations_json[${index}]`,
    );
    if (!duration.ok) return invalid(duration.error);
    const durationSecs = duration.value;
    if (seen.has(durationSecs)) {
      return invalid(
        `视频配置 supported_durations_json 包含重复时长：${durationSecs}`,
      );
    }
    const previous = supportedDurations[supportedDurations.length - 1];
    if (previous !== undefined && durationSecs <= previous) {
      return invalid(
        `视频配置 supported_durations_json 必须按从小到大填写；第 ${index + 1} 项 ${durationSecs} 不大于前一项 ${previous}`,
      );
    }
    seen.add(durationSecs);
    supportedDurations.push(durationSecs);
  }
  if (supportedDurations.length === 0) {
    return invalid(
      "视频配置 fixed 模式必须提供 supported_durations_json 正整数数组",
    );
  }
  const supportedMin = supportedDurations[0];
  const supportedMax = supportedDurations[supportedDurations.length - 1];
  if (minDurationSecs !== supportedMin || maxDurationSecs !== supportedMax) {
    return invalid(
      "视频配置 fixed 模式的 min/max 时长必须等于 supported_durations_json 首尾项",
    );
  }
  if (!supportedDurations.includes(defaultDurationSecs)) {
    return invalid(
      "视频配置 default_duration_secs 必须包含在 supported_durations_json 中",
    );
  }
  return {
    model: config.model,
    durationMode: "fixed",
    defaultDurationSecs,
    minDurationSecs: supportedMin,
    maxDurationSecs: supportedMax,
    supportedDurations,
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

const strictPersistedVideoImageIds = (
  value: unknown,
  field: string,
  maxCount: number | null,
): number[] => {
  if (!Array.isArray(value)) throw new Error(`${field} 必须是数组`);
  if (maxCount != null && value.length > maxCount) {
    throw new Error(`${field} 最多允许 ${maxCount} 个图片 ID`);
  }
  const seen = new Set<number>();
  return value.map((item, index) => {
    if (!Number.isInteger(item) || (item as number) <= 0) {
      throw new Error(`${field}[${index}] 必须是正整数`);
    }
    const id = item as number;
    if (seen.has(id)) throw new Error(`${field} 包含重复图片 ID：${id}`);
    seen.add(id);
    return id;
  });
};

const parsePersistedVideoImageIdSelection = (
  raw: string,
  field: string,
  maxCount: number | null,
): PersistedVideoImageIdSelection => {
  try {
    return {
      ids: strictPersistedVideoImageIds(
        JSON.parse(raw) as unknown,
        field,
        maxCount,
      ),
      storageError: null,
    };
  } catch (err) {
    return {
      ids: [],
      storageError: `${field} 本地存储损坏：${extractErrorMessage(err, "")}`,
    };
  }
};

export const parseGenerationReferenceImageSelection = (
  raw: string,
): PersistedVideoImageIdSelection =>
  parsePersistedVideoImageIdSelection(
    raw,
    "生成参考图片选择",
    MAX_CHAT_REFERENCE_IMAGES,
  );

export const parseVideoLibraryImageSelection = (
  raw: string,
): PersistedVideoImageIdSelection =>
  parsePersistedVideoImageIdSelection(
    raw,
    "视频素材库图片选择",
    MAX_VIDEO_LIBRARY_IMAGES,
  );

export const serializeGenerationReferenceImageSelection = (
  value: PersistedVideoImageIdSelection,
): string =>
  JSON.stringify(
    strictPersistedVideoImageIds(
      value.ids,
      "生成参考图片选择",
      MAX_CHAT_REFERENCE_IMAGES,
    ),
  );

export const serializeVideoLibraryImageSelection = (
  value: PersistedVideoImageIdSelection,
): string =>
  JSON.stringify(
    strictPersistedVideoImageIds(
      value.ids,
      "视频素材库图片选择",
      MAX_VIDEO_LIBRARY_IMAGES,
    ),
  );

export function isVideoLibraryVisibleImage(
  img: GeneratedImage,
  chapter: ChapterRow | null,
  referencedImageIds: ReadonlySet<number>,
): boolean {
  return (
    isChapterVideoMaterialImage(img, chapter) || referencedImageIds.has(img.id)
  );
}

export const extractPromptSourceHash = (metadata: unknown): string | null => {
  const source = asRecord(metadata)?.source;
  const hash = asRecord(source)?.content_sha256;
  return typeof hash === "string" && hash.trim() ? hash.trim() : null;
};

const extractImageSourceHash = (metadata: unknown): string | null => {
  const source = asRecord(metadata)?.source;
  const hash = asRecord(source)?.content_sha256;
  return typeof hash === "string" && hash.trim() ? hash.trim() : null;
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
  return extractImageSourceHash(img.metadata_json) === currentContentHash;
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
    extractImageSourceHash(img.metadata_json) !== currentContentHash
  ) {
    return "指纹不一致";
  }
  return "本章图片";
};
