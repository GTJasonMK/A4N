import {
  type GeneratedImage,
  type VideoFramePromptImageGenerationSource,
  type VideoFramePromptImageType,
} from "../../../api/image";
import {
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  type ChapterVideoPrompt,
  type SegmentGuidanceMode,
  type VideoSegmentWorkflowMode,
} from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { isPositiveSafeInteger } from "../../../utils/numberRange";
import { parseApiTimestampMs } from "../../../utils/timestamp";
import type { MessageTranslator } from "../../../i18n";
import {
  VIDEO_ASPECT_RATIOS,
  VIDEO_FRAME_PROMPT_TYPE,
  videoFrameImageContractForPromptType,
  type VideoAspectRatio,
} from "./videoTabConstants";
import {
  readSavedReferenceImageIds,
  videoImageGenerationPromptId,
  videoImageGenerationSource,
  videoImageGenerationSourceHash,
  videoTailFrameBoundaryNumber,
} from "./videoImageUtils";
import { extractSourceContentHash, positiveIntegerOrNull, trimmedText } from "./videoTabShared";

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
  return parseApiTimestampMs(prompt.updated_at) ?? 0;
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
    return isPositiveSafeInteger(index)
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
    if (!isPositiveSafeInteger(index) || index > referenceCount) {
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
  return {
    prompt: promptWithReferenceNote,
    negative_prompt: asRecord(metadata.prompt_snapshot)?.negative_prompt ?? null,
    image_type: imageType,
    chapter_id: chapterId,
    aspect_ratio: aspectRatio,
    metadata_json: metadata,
    reference_image_ids: referenceImageIds,
  };
}
