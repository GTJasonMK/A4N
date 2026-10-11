import type {
  ProgressMeta,
  ProgressDisplayPhase,
  ProgressSnapshot,
  ProgressStatus,
} from "../api/progress";
import {
  formatNumber,
  type Locale,
  type MessageKey,
  type MessageTranslator,
} from "../i18n";
import { progressDisplayText } from "./progressDisplay";
import { TASK_TYPE_LABEL_KEYS } from "./workflowPresentation";

export interface ProgressDisplayModel {
  stage: string;
  detail: string | null;
  percent: number | null;
  countLabel: string | null;
  etaMode: "hidden" | "rough" | "linear";
  tone: "default" | "success" | "warning" | "danger";
  phaseKind: ProgressDisplayPhase | null;
}

interface ProgressPresentationInput extends Partial<ProgressSnapshot> {
  locale: Locale;
  t: MessageTranslator;
  status?: ProgressStatus;
  stageHint?: string | null;
  fallbackStage?: string;
  streamText?: string | null;
}

const LINEAR_PHASES = new Set<ProgressDisplayPhase>(["transfer"]);
const MODEL_PHASES = new Set<ProgressDisplayPhase>([
  "model_connect",
  "model_wait",
  "model_stream",
  "structured_output",
  "parse",
  "validate",
]);

const STAGE_LABEL_KEYS: Record<string, MessageKey> = {
  manga_preprocessing: "comic.pipelineStages.preprocessing",
  manga_page_slots: "comic.pipelineStages.pagination",
  manga_adaptation: "comic.pipelineStages.pageAdaptation",
  manga_storyboard: "comic.pipelineStages.storyboard",
  manga_reference_discovery: "comic.pipelineStages.referenceDiscovery",
  manga_drawing_prompt: "comic.pipelineStages.pagePrompt",
  manga_image: "comic.pipelineStages.pageImage",
  manga_image_progress: "comic.pipelineStages.pageImage",
  manga_completed: "comic.pipelineStages.completed",
  provider_connect: "feedback.progressStage.modelConnect",
  provider_wait: "feedback.progressStage.modelWait",
  provider_stream: "feedback.progressStage.modelStream",
  shooting_script_projection: "workflow.shootingScriptProjection",
  script_adaptation: "workflow.scriptAdaptation",
  character_staging: "workflow.characterStaging",
  visual_planning: "workflow.visualPlanning",
  branch_design: "workflow.branchDesign",
  persist_script: "workflow.persistScript",
  asset_generation: "workflow.assetGeneration",
  video_prompt_generation: "feedback.progressStage.modelStream",
  video_prompt_item_committed: "feedback.progressStage.save",
  video_prompt_checkpoint: "feedback.progressStage.save",
};

const PHASE_LABEL_KEYS: Record<ProgressDisplayPhase, MessageKey> = {
  prepare: "feedback.progressPreparing",
  context: "feedback.progressStage.context",
  model_connect: "feedback.progressStage.modelConnect",
  model_wait: "feedback.progressStage.modelWait",
  model_stream: "feedback.progressStage.modelStream",
  structured_output: "feedback.progressStage.structuredOutput",
  parse: "feedback.progressStage.parse",
  validate: "feedback.progressStage.validate",
  save: "feedback.progressStage.save",
  asset_generation: "feedback.progressStage.assetGeneration",
  transfer: "feedback.progressStage.transfer",
  queue: "feedback.progressStage.queue",
  cleanup: "feedback.progressStage.cleanup",
  complete: "feedback.progressCompleted",
};

const PHASE_DETAIL_KEYS: Record<ProgressDisplayPhase, MessageKey> = {
  prepare: "feedback.progressPreparing",
  context: "feedback.progressDetail.context",
  model_connect: "feedback.progressDetail.modelConnect",
  model_wait: "feedback.progressDetail.modelWait",
  model_stream: "feedback.progressDetail.modelStream",
  structured_output: "feedback.progressDetail.structuredOutput",
  parse: "feedback.progressDetail.parse",
  validate: "feedback.progressDetail.validate",
  save: "feedback.progressDetail.save",
  asset_generation: "feedback.progressDetail.assetGeneration",
  transfer: "feedback.progressDetail.transfer",
  queue: "feedback.progressDetail.queue",
  cleanup: "feedback.progressDetail.cleanup",
  complete: "feedback.progressDone",
};

export function progressDisplayModel({
  current = 0,
  total = 1,
  message,
  percent,
  meta,
  status = "running",
  locale,
  t,
  stageHint,
  fallbackStage,
  streamText,
}: ProgressPresentationInput): ProgressDisplayModel {
  const boundedTotal = Math.max(1, total || 1);
  const boundedCurrent = Math.max(0, Math.min(current || 0, boundedTotal));
  const computedPercent = Number.isFinite(percent ?? NaN)
    ? normalizePercent(Number(percent))
    : Math.round((boundedCurrent / boundedTotal) * 100);
  const normalizedMeta = meta ?? null;
  const fallback = progressDisplayText({
    message,
    streamText,
    stageHint,
    fallbackStage,
  });
  const stage = normalizedMeta
    ? localizedProgressStage(normalizedMeta, t)
    : fallback.stage || t("feedback.progressRunning");
  const detailCandidate = status !== "running"
    ? progressStatusDetail(status, t)
    : normalizedMeta && locale === "en-US"
      ? t(PHASE_DETAIL_KEYS[normalizedMeta.phaseKind])
      : normalizedMeta?.detail?.trim() || fallback.detail || progressStatusDetail(status, t);
  const detail =
    detailCandidate && detailCandidate.trim() !== stage.trim() ? detailCandidate : null;
  const phaseKind = normalizedMeta?.phaseKind ?? null;
  const countLabel = progressCountLabel(
    boundedCurrent,
    boundedTotal,
    normalizedMeta,
    locale,
    t,
  );
  const etaMode =
    status !== "running" || normalizedMeta?.taskKind === "manga_pipeline"
      ? "hidden"
      : phaseKind && LINEAR_PHASES.has(phaseKind)
        ? "linear"
        : phaseKind && MODEL_PHASES.has(phaseKind)
          ? "hidden"
          : "rough";

  return {
    stage,
    detail,
    percent: status === "success" ? 100 : computedPercent,
    countLabel,
    etaMode,
    tone: progressTone(status, normalizedMeta),
    phaseKind,
  };
}

function localizedProgressStage(meta: ProgressMeta, t: MessageTranslator): string {
  const key =
    STAGE_LABEL_KEYS[meta.stageCode] ??
    TASK_TYPE_LABEL_KEYS[meta.taskKind] ??
    PHASE_LABEL_KEYS[meta.phaseKind];
  return t(key);
}

export function compactProgressEvents<T extends ProgressSnapshot & { id: string }>(
  events: T[],
  limit: number,
): T[] {
  const compacted: T[] = [];
  for (const event of events) {
    const previous = compacted[compacted.length - 1];
    if (previous && sameProgressEvent(previous, event)) {
      compacted[compacted.length - 1] = event;
    } else {
      compacted.push(event);
    }
  }
  return compacted.slice(-Math.max(1, limit));
}

function normalizePercent(value: number): number {
  const percent = value <= 1 ? value * 100 : value;
  return Math.max(0, Math.min(100, percent));
}

function progressCountLabel(
  current: number,
  total: number,
  meta: ProgressMeta | null,
  locale: Locale,
  t: MessageTranslator,
): string | null {
  if (meta?.itemIndex != null && meta.itemTotal != null) {
    if (meta.taskKind === "manga_pipeline") {
      return t("comic.prompt.pageProgress", {
        current: formatNumber(meta.itemIndex + 1, locale),
        total: formatNumber(meta.itemTotal, locale),
      });
    }
    return t("feedback.progressCount", {
      current: formatNumber(meta.itemIndex + 1, locale),
      total: formatNumber(meta.itemTotal, locale),
    });
  }
  if (total === 100) return null;
  return t("feedback.progressCount", {
    current: formatNumber(current, locale),
    total: formatNumber(total, locale),
  });
}

function progressTone(
  status: ProgressStatus,
  meta: ProgressMeta | null,
): ProgressDisplayModel["tone"] {
  if (status === "error" || meta?.severity === "error") return "danger";
  if (status === "success" || meta?.severity === "success") return "success";
  if (status === "cancelled" || meta?.severity === "warning") return "warning";
  return "default";
}

function progressStatusDetail(status: ProgressStatus, t: MessageTranslator): string | null {
  switch (status) {
    case "success":
      return t("feedback.progressDone");
    case "error":
      return t("feedback.progressCheckError");
    case "cancelled":
      return t("feedback.progressStopped");
    case "idle":
      return t("feedback.progressWaiting");
    case "running":
      return null;
  }
}

function sameProgressEvent(left: ProgressSnapshot, right: ProgressSnapshot): boolean {
  return (
    (left.meta?.stageCode ?? "") === (right.meta?.stageCode ?? "") &&
    (left.meta?.detail ?? left.message ?? "") === (right.meta?.detail ?? right.message ?? "") &&
    left.current === right.current &&
    left.total === right.total
  );
}
