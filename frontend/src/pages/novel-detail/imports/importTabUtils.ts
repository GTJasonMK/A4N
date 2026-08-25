import type { ProjectImportAnalysisProgress, ReverseEngineeringTarget } from "../../../api/import";
import type { ProgressDialogStatus } from "../../../components/feedback/ProgressDialog";

export const formalReverseNotice = (
  successMessage: string,
  warnings: string[],
  separator: string,
): { message: string; tone: "info" | "success" } => ({
  message: warnings.length > 0 ? `${successMessage}${separator}${warnings.join(separator)}` : successMessage,
  tone: warnings.length > 0 ? "info" : "success",
});

export const REVERSE_CONTEXT_LIMITS: Record<ReverseEngineeringTarget, number> = {
  blueprint: 50,
  outline: 100,
  part_outline: 200,
};

export interface ImportAnalysisDialogState {
  open: boolean;
  status: ProgressDialogStatus;
  current: number;
  total: number;
  message: string;
  error: string | null;
}

const IMPORT_ANALYSIS_DIALOG_STATUS: Record<
  ProjectImportAnalysisProgress["status"],
  ProgressDialogStatus
> = {
  analyzing: "running",
  completed: "success",
  failed: "error",
  cancelled: "cancelled",
};

export const importAnalysisDialogFromProgress = (
  progress: ProjectImportAnalysisProgress,
): ImportAnalysisDialogState => {
  const stage = progress.stages.generating_analysis_data;
  const status = IMPORT_ANALYSIS_DIALOG_STATUS[progress.status];
  return {
    open: true,
    status,
    current: status === "success" ? stage.total : stage.completed,
    total: stage.total,
    message: progress.message,
    error: progress.error,
  };
};

export const delay = (ms: number): Promise<void> =>
  new Promise((resolve) => window.setTimeout(resolve, ms));
