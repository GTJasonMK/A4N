import type { ParsedChapter, ProjectImportAnalysisProgress } from "../../../api/import";
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
