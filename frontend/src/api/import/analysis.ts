import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  ImportAnalysisStatus,
  ImportProjectAnalysisSummary,
  ImportReview,
  ProjectImportAnalysisProgress,
  StartImportAnalysisInput,
} from "./types";
import { type ApiId as Id, idNum } from "../shared";

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

/**
 * 应用导入分析阶段已经保存的蓝图候选。
 * 后端只从项目进度读取候选，前端不上传可篡改的蓝图 JSON。
 */
export async function applyImportAnalysisBlueprintCandidate(
  projectId: Id,
): Promise<unknown> {
  return apiInvoke<unknown>("apply_import_analysis_blueprint_candidate", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

/** 确认服务端保存的分部大纲候选；不上传候选 JSON。 */
export async function applyImportAnalysisPartOutlineCandidate(
  projectId: Id,
): Promise<unknown> {
  return apiInvoke<unknown>("apply_import_analysis_part_outline_candidate", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}
