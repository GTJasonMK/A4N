import { apiInvoke } from "./index";
import type {
  CancelWorkflowInput,
  GetWorkflowRunInput,
  ListWorkflowRunsInput,
  ResumeWorkflowRunInput,
  WorkflowCheckpointSummary,
  WorkflowRunDetailSummary,
  WorkflowRunSummary,
  WorkflowStateSummary,
} from "./generated/host";
import type {
  ProgressEventRecord,
} from "./progress";
import { getWorkflowProgressEvents } from "./progress";

export type {
  WorkflowCheckpointSummary,
  WorkflowRunDetailSummary,
  WorkflowRunSummary,
  WorkflowStateSummary,
};

/**
 * Streaming workflow API wrapper —— 对齐 crates/host/a4n-tauri/src/commands/streaming.rs
 *
 * 后端工作流状态来自 task_queue 的 running/completed/failed 任务子集。
 */

export async function listActiveWorkflows(): Promise<WorkflowStateSummary[]> {
  return apiInvoke<WorkflowStateSummary[]>("list_active_workflows");
}

export async function listWorkflowRuns(
  input: ListWorkflowRunsInput,
): Promise<WorkflowRunSummary[]> {
  return apiInvoke<WorkflowRunSummary[]>("list_workflow_runs", { input });
}

export async function getWorkflowRunDetail(
  workflowRunId: number,
  includeGenerationRuns: boolean,
): Promise<WorkflowRunDetailSummary> {
  return apiInvoke<WorkflowRunDetailSummary>("get_workflow_run_detail", {
    input: {
      workflow_run_id: workflowRunId,
      include_generation_runs: includeGenerationRuns,
    } satisfies GetWorkflowRunInput,
  });
}

export async function resumeWorkflowRun(workflowRunId: number): Promise<WorkflowRunSummary> {
  return apiInvoke<WorkflowRunSummary>("resume_workflow_run", {
    input: { workflow_run_id: workflowRunId } satisfies ResumeWorkflowRunInput,
  });
}

export async function getWorkflowRunProgressEvents(
  workflowRunId: number,
  limit: number,
): Promise<ProgressEventRecord[]> {
  const response = await getWorkflowProgressEvents({
    task_id: null,
    workflow_run_id: workflowRunId,
    limit,
  });
  return response.events;
}

export async function cancelWorkflow(input: CancelWorkflowInput): Promise<void> {
  return apiInvoke<void>("cancel_workflow", {
    input,
  });
}

export function cancelWorkflowTask(taskId: string): Promise<void> {
  return cancelWorkflow({ task_id: taskId, workflow_run_id: null });
}
