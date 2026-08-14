import type {
  ProgressMeta,
  ProgressDisplayPhase,
  ProgressSnapshot,
  ProgressStatus,
} from "../api/progress";
import { progressDisplayText } from "./progressDisplay";

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

export function progressDisplayModel({
  current = 0,
  total = 1,
  message,
  percent,
  meta,
  status = "running",
  stageHint,
  fallbackStage = "任务执行中",
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
  const stage = normalizedMeta?.stageLabel?.trim() || fallback.stage;
  const detailCandidate =
    normalizedMeta?.detail?.trim() ||
    fallback.detail ||
    progressStatusDetail(status);
  const detail =
    detailCandidate && detailCandidate.trim() !== stage.trim() ? detailCandidate : null;
  const phaseKind = normalizedMeta?.phaseKind ?? null;
  const countLabel = progressCountLabel(boundedCurrent, boundedTotal, normalizedMeta);
  const etaMode =
    status !== "running"
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
): string | null {
  if (meta?.unitLabel && meta.itemIndex != null && meta.itemTotal != null) {
    if (meta.unitLabel === "字节") return null;
    return `第 ${meta.itemIndex}/${meta.itemTotal} 个${meta.unitLabel}`;
  }
  if (total === 100) return null;
  return `${current}/${total}`;
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

function progressStatusDetail(status: ProgressStatus): string | null {
  switch (status) {
    case "success":
      return "已完成";
    case "error":
      return "请查看错误信息";
    case "cancelled":
      return "已停止";
    case "idle":
      return "等待开始";
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
