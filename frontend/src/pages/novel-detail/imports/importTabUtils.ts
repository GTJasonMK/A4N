import { isJsonRecord } from "../../../utils/jsonRecord";
import type {
  ImportBlueprintCandidate,
  ImportBlueprintCandidateApplyResult,
  ImportCandidateIdentity,
  ImportCandidateSourceScope,
  ImportPartOutlineCandidate,
  ImportPartOutlineCandidateApplyResult,
  ImportPartOutlineCandidateItem,
  ParsedChapter,
  ProjectImportAnalysisProgress,
} from "../../../api/import";
import type { WorkflowCheckpoint } from "../../../api/streaming";
import type { ProgressDialogStatus } from "../../../components/feedback/ProgressDialog";

export const dedupStats = (
  hashes: Array<ParsedChapter["content_hash"]>,
): { dedup_paragraphs: number; dedup_rate: number } => {
  const known = hashes.filter((hash): hash is string => Boolean(hash));
  if (known.length === 0) return { dedup_paragraphs: hashes.length, dedup_rate: 0 };
  const unique = new Set(known);
  return {
    dedup_paragraphs: known.length,
    dedup_rate: (known.length - unique.size) / known.length,
  };
};

export type ReverseType = "blueprint" | "outline" | "part_outline";
export type ReversePromptName =
  | "reverse_blueprint"
  | "reverse_outline"
  | "reverse_part_outline";

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
    !["proposal", "confirmed", "already_confirmed"].includes(String(value.state)) ||
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

export const reverseCandidateFromCheckpoint = (
  checkpoint: WorkflowCheckpoint,
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
  if (!candidate || candidate.workflow_run_id !== checkpoint.workflow_run_id) return null;
  if (!isJsonRecord(checkpoint.checkpoint_json)) return null;
  const result =
    checkpoint.checkpoint_json.parsed_output ?? checkpoint.checkpoint_json.output;
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
  typeof value === "number" && Number.isFinite(value) && Number.isInteger(value);

const isPositiveInteger = (value: unknown): value is number =>
  isFiniteInteger(value) && value > 0;

const isNonNegativeInteger = (value: unknown): value is number =>
  isFiniteInteger(value) && value >= 0;

const isNonEmptyText = (value: unknown): value is string =>
  typeof value === "string" && value.trim().length > 0;

const isCandidateApplyResult = (
  value: unknown,
): value is Record<string, unknown> => isJsonRecord(value);

const isBlueprintCandidateApplyResult = (
  value: unknown,
): value is ImportBlueprintCandidateApplyResult =>
  isJsonRecord(value) &&
  isNonEmptyText(value.version) &&
  (value.updated_fields === undefined ||
    (Array.isArray(value.updated_fields) &&
      value.updated_fields.every(isNonEmptyText)));

const isPartCandidateApplyResult = (
  value: unknown,
): value is ImportPartOutlineCandidateApplyResult =>
  isJsonRecord(value) &&
  isPositiveInteger(value.part_count) &&
  isNonNegativeInteger(value.inserted_count) &&
  (value.updated_count === undefined ||
    isNonNegativeInteger(value.updated_count)) &&
  isNonNegativeInteger(value.reused_count);

export const isImportCandidateIdentity = (
  value: unknown,
  projectId: number,
): value is ImportCandidateIdentity =>
  isJsonRecord(value) &&
  value.project_id === projectId &&
  value.origin === "import_analysis" &&
  (value.state === "proposal" || value.state === "confirmed");

export const isImportCandidateSourceScope = (
  value: unknown,
): value is ImportCandidateSourceScope => {
  if (!isJsonRecord(value)) return false;
  return (
    isPositiveInteger(value.chapter_start) &&
    isPositiveInteger(value.chapter_end) &&
    isPositiveInteger(value.chapter_count) &&
    value.chapter_start <= value.chapter_end
  );
};

export const formatImportCandidateSourceScope = (
  scope: ImportCandidateSourceScope,
): string =>
  `第 ${scope.chapter_start}–${scope.chapter_end} 章（${scope.chapter_count} 章）`;

export const parseImportBlueprintCandidate = (
  value: unknown,
  projectId: number,
): ImportBlueprintCandidate | null => {
  if (!isImportCandidateIdentity(value, projectId)) return null;
  const candidate = value as unknown as Record<string, unknown>;
  if (
    !isNonEmptyText(candidate.title) ||
    !isNonEmptyText(candidate.genre) ||
    !isNonEmptyText(candidate.one_sentence_summary) ||
    !isNonEmptyText(candidate.full_synopsis) ||
    !isJsonRecord(candidate.world_setting) ||
    !isPositiveInteger(candidate.total_chapters) ||
    typeof candidate.needs_part_outlines !== "boolean" ||
    !(candidate.chapters_per_part === null ||
      isPositiveInteger(candidate.chapters_per_part)) ||
    !isImportCandidateSourceScope(candidate.source_scope)
  ) {
    return null;
  }
  if (candidate.state === "proposal") {
    if (candidate.apply_result != null && !isCandidateApplyResult(candidate.apply_result)) {
      return null;
    }
    return candidate as unknown as ImportBlueprintCandidate;
  }
  if (
    !isNonEmptyText(candidate.confirmed_at) ||
    !isBlueprintCandidateApplyResult(candidate.apply_result)
  ) {
    return null;
  }
  return candidate as unknown as ImportBlueprintCandidate;
};

const isImportPartOutlineCandidateItem = (
  value: unknown,
): value is ImportPartOutlineCandidateItem => {
  if (!isJsonRecord(value)) return false;
  return (
    isPositiveInteger(value.part_number) &&
    isNonEmptyText(value.title) &&
    isPositiveInteger(value.start_chapter) &&
    isPositiveInteger(value.end_chapter) &&
    value.start_chapter <= value.end_chapter &&
    isNonEmptyText(value.summary) &&
    isNonEmptyText(value.theme) &&
    Array.isArray(value.key_events) &&
    Array.isArray(value.character_focus) &&
    isImportCandidateSourceScope(value.source_scope)
  );
};

export const parseImportPartOutlineCandidate = (
  value: unknown,
  projectId: number,
): ImportPartOutlineCandidate | null => {
  if (!isImportCandidateIdentity(value, projectId)) return null;
  const candidate = value as unknown as Record<string, unknown>;
  if (
    !Array.isArray(candidate.parts) ||
    candidate.parts.length === 0 ||
    !candidate.parts.every(isImportPartOutlineCandidateItem)
  ) {
    return null;
  }
  if (candidate.state === "proposal") {
    if (candidate.apply_result != null && !isCandidateApplyResult(candidate.apply_result)) {
      return null;
    }
    return candidate as unknown as ImportPartOutlineCandidate;
  }
  if (
    !isNonEmptyText(candidate.confirmed_at) ||
    !isPartCandidateApplyResult(candidate.apply_result)
  ) {
    return null;
  }
  return candidate as unknown as ImportPartOutlineCandidate;
};
