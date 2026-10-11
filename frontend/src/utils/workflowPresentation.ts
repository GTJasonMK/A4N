import type { Locale } from "../i18n/locale";
import {
  translateMessage,
  type MessageKey,
  type MessageTranslator,
} from "../i18n/translate";

const WORKFLOW_STATUS_KEYS: Record<string, MessageKey> = {
  idle: "workflow.idle",
  draft: "workflow.draft",
  imported: "workflow.imported",
  ready: "workflow.ready",
  pending: "workflow.pending",
  needs_analysis: "workflow.needsAnalysis",
  analyzing: "workflow.analyzing",
  reviewing: "workflow.reviewing",
  queued: "workflow.queued",
  running: "workflow.runningStatus",
  completed: "workflow.success",
  success: "workflow.successStatus",
  succeeded: "workflow.successStatus",
  failed: "workflow.failed",
  error: "workflow.error",
  interrupted: "workflow.interrupted",
  cancelled: "workflow.cancelled",
  canceled: "workflow.cancelled",
  paused: "workflow.paused",
  blocked: "workflow.blocked",
  generating: "workflow.generating",
  generated: "workflow.generated",
  deleted: "workflow.deleted",
  approved: "workflow.approved",
  accepted: "workflow.accepted",
  rejected: "workflow.rejected",
  modified: "workflow.modified",
  stale: "workflow.stale",
  unknown: "workflow.unknownStatus",
};

export const getOperationStatusLabel = (
  value: string | null | undefined,
  locale: Locale,
): string => {
  const normalized = String(value ?? "").trim().toLowerCase();
  const key = WORKFLOW_STATUS_KEYS[normalized] ?? "workflow.unknownStatus";
  return translateMessage(locale, key);
};

export interface ChapterSyncFailurePresentation {
  summary: string;
  detail: string | null;
  recommendedAction: "embedding_settings" | null;
}

/**
 * 将章节同步的内部错误收敛为作者能执行的恢复建议；原始错误仍作为诊断信息保留。
 */
export const getChapterSyncFailurePresentation = (
  error: string | null | undefined,
  fallback: string,
  t: MessageTranslator,
): ChapterSyncFailurePresentation => {
  const detail = String(error ?? "").trim() || null;
  if (!detail) {
    return { summary: fallback, detail: null, recommendedAction: null };
  }

  if (/vector dimension error|expected dim:\s*\d+\s*,\s*got\s*\d+/i.test(detail)) {
    return {
      summary: t("writing.syncFailureEmbeddingMismatch"),
      detail,
      recommendedAction: "embedding_settings",
    };
  }

  if (/chapter_analysis_host|状态无效|不允许操作.*更新场景档案/i.test(detail)) {
    return {
      summary: t("writing.syncFailureState"),
      detail,
      recommendedAction: null,
    };
  }

  return { summary: fallback, detail, recommendedAction: null };
};

export const TASK_TYPE_LABEL_KEYS: Readonly<Record<string, MessageKey>> = {
  llm_call: "workflow.taskLanguageCall",
  chapter_generation: "workflow.taskChapterGeneration",
  image_generation: "workflow.taskImageGeneration",
  video_generation: "workflow.taskVideoGeneration",
  video_poll: "workflow.taskVideoPoll",
  source_sync: "workflow.taskSourceSync",
  assembly: "workflow.taskAssembly",
  embedding_call: "workflow.taskEmbeddingCall",
  rag_ingestion: "workflow.taskRagIngestion",
  import_analysis: "workflow.taskImportAnalysis",
  export_generation: "workflow.taskExportGeneration",
  embedding_download: "workflow.taskEmbeddingDownload",
};

const TASK_PRIORITY_LABELS: Record<string, MessageKey> = {
  high: "workflow.priorityHigh",
  medium: "workflow.priorityMedium",
  low: "workflow.priorityLow",
};

const WORKFLOW_KIND_LABELS: Record<string, MessageKey> = {
  video_provider_job: "workflow.kindVideoProvider",
  video_canvas_graph: "workflow.kindVideoCanvas",
  chapter_analysis: "workflow.kindChapterAnalysis",
  chapter_content_sync: "workflow.kindChapterSync",
  blueprint_generation: "workflow.kindBlueprintGeneration",
  blueprint_refine: "workflow.kindBlueprintRefine",
  source_sync: "workflow.kindSourceSync",
  import_analysis: "workflow.kindImportAnalysis",
  rag_ingestion: "workflow.kindRagIngestion",
};

const WORKFLOW_MODE_LABELS: Record<string, MessageKey> = {
  full: "workflow.modeFull",
  fresh: "workflow.modeFresh",
  start: "workflow.modeStart",
  resume: "workflow.modeResume",
  retry: "workflow.modeRetry",
  incremental: "workflow.modeIncremental",
  content_change: "workflow.modeContentChange",
  boundary_frames: "workflow.modeBoundaryFrames",
  first_frame_chain: "workflow.modeFirstFrameChain",
  storyboard: "workflow.modeStoryboard",
  chat_video: "workflow.modeChatVideo",
  reference_media: "workflow.modeReferenceMedia",
};

const mappedLabel = (
  value: string | null | undefined,
  labels: Record<string, MessageKey>,
  fallback: MessageKey,
  locale: Locale,
): string => {
  const normalized = String(value ?? "").trim().toLowerCase();
  return translateMessage(locale, labels[normalized] ?? fallback);
};

export const getTaskTypeLabel = (
  value: string | null | undefined,
  locale: Locale,
): string => mappedLabel(value, TASK_TYPE_LABEL_KEYS, "workflow.taskChapterGeneration", locale);

export const getTaskPriorityLabel = (
  value: string | null | undefined,
  locale: Locale,
): string => mappedLabel(value, TASK_PRIORITY_LABELS, "workflow.priorityMedium", locale);

export const getWorkflowKindLabel = (
  value: string | null | undefined,
  locale: Locale,
): string => mappedLabel(value, WORKFLOW_KIND_LABELS, "workflow.kindChapterAnalysis", locale);

export const getWorkflowModeLabel = (
  value: string | null | undefined,
  locale: Locale,
): string => mappedLabel(value, WORKFLOW_MODE_LABELS, "workflow.modeFull", locale);
