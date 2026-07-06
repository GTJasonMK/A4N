import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  ImportAnalysisStatus,
  ImportProjectAnalysisSummary,
  ImportReview,
  ProjectImportAnalysisProgress,
  StartImportAnalysisInput,
} from "./types";
import { type ApiId as Id, idNum } from "../shared";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问导入分析");
  return id;
};

export async function startImportAnalysis(
  projectId: Id,
  input: StartImportAnalysisInput,
): Promise<ImportReview[]> {
  return apiInvoke<ImportReview[]>("start_import_analysis", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input,
  });
}

export async function getImportAnalysisStatus(
  projectId: Id,
): Promise<ImportAnalysisStatus[]> {
  return apiInvoke<ImportAnalysisStatus[]>("get_import_analysis_status", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function getProjectImportAnalysisProgress(
  projectId: Id,
): Promise<ProjectImportAnalysisProgress | null> {
  return apiInvoke<ProjectImportAnalysisProgress | null>(
    "get_project_import_analysis_progress",
    { userId: currentUserId(), projectId: idNum(projectId) },
  );
}

export async function runImportProjectAnalysis(
  projectId: Id,
): Promise<ImportProjectAnalysisSummary> {
  return apiInvoke<ImportProjectAnalysisSummary>("run_import_project_analysis", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function previewImportProjectAnalysisRequest(
  projectId: Id,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_import_project_analysis_request", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function cancelImportAnalysis(projectId: Id): Promise<number> {
  return apiInvoke<number>("cancel_import_analysis", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}
