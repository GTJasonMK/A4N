import { apiInvoke } from "./index";
import type { ModelRequestPreview } from "./requestPreview";
import type { ProjectFactFeedbackSummary, RunProjectFactFeedbackInput } from "./generated/host";

export type { ProjectFactFeedbackSummary, MaterialFactChangeSummary, RunProjectFactFeedbackInput } from "./generated/host";

export function getProjectFactFeedback(projectId: number): Promise<ProjectFactFeedbackSummary> {
  return apiInvoke("get_project_fact_feedback", { projectId });
}

export function runProjectFactFeedback(input: RunProjectFactFeedbackInput, taskId: string): Promise<ProjectFactFeedbackSummary> {
  return apiInvoke("run_project_fact_feedback", { input, taskId });
}

export function previewProjectFactFeedback(input: RunProjectFactFeedbackInput): Promise<ModelRequestPreview> {
  return apiInvoke("preview_project_fact_feedback_request", { input });
}
