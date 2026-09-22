import { apiInvoke } from "../index";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  ImportProjectAnalysisSummary,
  ProjectImportAnalysisProgress,
} from "../generated/host";
import { type ApiId as Id, idNum } from "../shared";

export async function getProjectImportAnalysisProgress(
  projectId: Id,
): Promise<ProjectImportAnalysisProgress | null> {
  return apiInvoke<ProjectImportAnalysisProgress | null>(
    "get_project_import_analysis_progress",
    { projectId: idNum(projectId) },
  );
}

export async function runImportProjectAnalysis(
  projectId: Id,
): Promise<ImportProjectAnalysisSummary> {
  return apiInvoke<ImportProjectAnalysisSummary>(
    "run_import_project_analysis",
    {
      projectId: idNum(projectId),
    },
  );
}

export async function previewImportProjectAnalysisRequest(
  projectId: Id,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>(
    "preview_import_project_analysis_request",
    {
      projectId: idNum(projectId),
    },
  );
}

export async function cancelImportAnalysis(projectId: Id): Promise<void> {
  return apiInvoke<void>("cancel_import_analysis", {
    projectId: idNum(projectId),
  });
}
