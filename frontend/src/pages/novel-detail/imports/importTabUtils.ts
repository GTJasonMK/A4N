import { isJsonRecord } from "../../../utils/jsonRecord";
import type { ProjectImportAnalysisProgress } from "../../../api/import";
import type { WorkflowCheckpointSummary } from "../../../api/streaming";
import type { ProgressDialogStatus } from "../../../components/feedback/ProgressDialog";

export type ReverseType = "blueprint" | "outline" | "part_outline";
export type ReversePromptName =
  "reverse_blueprint" | "reverse_outline" | "reverse_part_outline";

const REVERSE_PROMPT_NAMES: Record<ReverseType, ReversePromptName> = {
  blueprint: "reverse_blueprint",
  outline: "reverse_outline",
  part_outline: "reverse_part_outline",
};

export const reversePromptName = (target: ReverseType): ReversePromptName =>
  REVERSE_PROMPT_NAMES[target];

export interface ReverseCandidateEnvelope {
  candidate_id: number;
  workflow_run_id: number;
  origin: "import_reverse";
  target: ReversePromptName;
  state: "proposal" | "confirmed" | "already_confirmed";
  warnings: string[];
  result?: unknown;
  applied?: unknown;
}

export const parseReverseCandidateEnvelope = (
  value: unknown,
  target: ReverseType,
): ReverseCandidateEnvelope | null => {
  if (!isJsonRecord(value)) return null;
  const candidateId = value.candidate_id;
  const workflowRunId = value.workflow_run_id;
  const warnings = value.warnings;
  if (
    !isPositiveInteger(candidateId) ||
    !isPositiveInteger(workflowRunId) ||
    candidateId !== workflowRunId ||
    value.origin !== "import_reverse" ||
    value.target !== reversePromptName(target) ||
    !["proposal", "confirmed", "already_confirmed"].includes(
      String(value.state),
    ) ||
    (warnings != null &&
      (!Array.isArray(warnings) ||
        !warnings.every((warning) => typeof warning === "string")))
  ) {
    return null;
  }
  return {
    ...value,
    warnings: warnings ?? [],
  } as unknown as ReverseCandidateEnvelope;
};

export const formalReverseNotice = (
  successMessage: string,
  warnings: string[],
): { message: string; tone: "info" | "success" } => ({
  message: warnings.length > 0 ? `${successMessage}；${warnings.join("；")}` : successMessage,
  tone: warnings.length > 0 ? "info" : "success",
});

export const reverseCandidateFromCheckpoint = (
  checkpoint: WorkflowCheckpointSummary,
  target: ReverseType,
): ReverseCandidateEnvelope | null => {
  if (
    checkpoint.status !== "completed" ||
    checkpoint.step_key !== reversePromptName(target)
  ) {
    return null;
  }
  if (!isJsonRecord(checkpoint.artifacts_json)) return null;
  const candidate = parseReverseCandidateEnvelope(
    checkpoint.artifacts_json.candidate,
    target,
  );
  if (!candidate || candidate.workflow_run_id !== checkpoint.workflow_run_id)
    return null;
  if (!isJsonRecord(checkpoint.checkpoint_json)) return null;
  const result =
    checkpoint.checkpoint_json.parsed_output ??
    checkpoint.checkpoint_json.output;
  if (result == null) return null;
  const applyResult = isJsonRecord(checkpoint.artifacts_json.candidate)
    ? checkpoint.artifacts_json.candidate.apply_result
    : undefined;
  if (candidate.state === "confirmed" && applyResult == null) return null;
  return {
    ...candidate,
    result,
    ...(applyResult == null ? {} : { applied: applyResult }),
  };
};

export const REVERSE_CONTEXT_LIMITS: Record<ReverseType, number> = {
  blueprint: 50,
  outline: 100,
  part_outline: 200,
};

export const projectProgressPercent = (
  progress: ProjectImportAnalysisProgress,
): number | null => {
  const value = progress.overall_progress;
  if (typeof value !== "number" || !Number.isFinite(value)) return null;
  return Math.min(100, Math.max(0, value));
};

export interface ImportAnalysisDialogState {
  open: boolean;
  status: ProgressDialogStatus;
  current: number;
  total: number;
  message: string;
  error: string | null;
}

export const emptyImportAnalysisDialog = (): ImportAnalysisDialogState => ({
  open: false,
  status: "idle",
  current: 0,
  total: 1,
  message: "",
  error: null,
});

const importAnalysisStatus = (status?: string): ProgressDialogStatus => {
  switch (status) {
    case "completed":
      return "success";
    case "failed":
      return "error";
    case "cancelled":
      return "cancelled";
    case "analyzing":
      return "running";
    default:
      return "idle";
  }
};

const importProgressTotals = (
  progress: ProjectImportAnalysisProgress,
): { current: number; total: number } => {
  const stages = progress.stages ? Object.values(progress.stages) : [];
  const totals = stages.reduce(
    (acc, stage) => ({
      current: acc.current + Math.max(0, stage.completed || 0),
      total: acc.total + Math.max(0, stage.total || 0),
    }),
    { current: 0, total: 0 },
  );
  if (totals.total > 0) return totals;
  const percent = projectProgressPercent(progress) ?? 0;
  return { current: percent, total: 100 };
};

export const importAnalysisDialogFromProgress = (
  progress: ProjectImportAnalysisProgress,
): ImportAnalysisDialogState => {
  const totals = importProgressTotals(progress);
  const status = importAnalysisStatus(progress.status);
  return {
    open: true,
    status,
    current: status === "success" ? totals.total : totals.current,
    total: Math.max(1, totals.total),
    message: progress.message || "正在分析导入项目",
    error: progress.error || null,
  };
};

export const delay = (ms: number): Promise<void> =>
  new Promise((resolve) => window.setTimeout(resolve, ms));

const isFiniteInteger = (value: unknown): value is number =>
  typeof value === "number" &&
  Number.isFinite(value) &&
  Number.isInteger(value);

const isPositiveInteger = (value: unknown): value is number =>
  isFiniteInteger(value) && value > 0;
