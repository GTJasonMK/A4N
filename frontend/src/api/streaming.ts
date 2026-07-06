import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import type { ProgressEventRecord, ProgressMeta, ProgressSnapshot } from "./progress";

/**
 * Streaming workflow API wrapper —— 对齐 crates/a4n-tauri/src/commands/streaming.rs
 *
 * 后端工作流状态来自 task_queue 的 running/completed/failed 任务子集。
 */

interface WorkflowProgress extends ProgressSnapshot {
  task_id: string;
  stage?: string | null;
}

export interface WorkflowState {
  task_id: string;
  status: string; // idle / running / paused / completed / failed
  project_id: number | null;
  root_project_id: number | null;
  source_snapshot_id: number | null;
  task_scope: string | null;
  stage: string | null;
  progress: WorkflowProgress | null;
  progress_events?: ProgressEventRecord[];
  started_at: string | null;
  completed_at: string | null;
}

export interface WorkflowRunProgress {
  current: number;
  total: number;
  message: string;
  percent: number;
  meta: ProgressMeta | null;
  updated_at: string | null;
}

export interface WorkflowRun {
  id: number;
  business_key: string;
  workflow_kind: string;
  project_id: number | null;
  root_project_id: number | null;
  source_snapshot_id: number | null;
  scope_table: string | null;
  scope_id: number | null;
  chapter_id: number | null;
  mode: string | null;
  status: string;
  current_step_key: string | null;
  progress: WorkflowRunProgress | null;
  active_task_id: string | null;
  can_resume: boolean;
  resume_block_reason: string | null;
  error_message: string | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

export interface WorkflowCheckpoint {
  id: number;
  workflow_run_id: number;
  step_key: string;
  step_order: number;
  status: string;
  input_fingerprint_json: unknown | null;
  checkpoint_json: unknown | null;
  artifacts_json: unknown | null;
  generation_run_ids_json: unknown | null;
  error_message: string | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

export interface GenerationRun {
  id: number;
  workflow_run_id: number | null;
  workflow: string;
  step_key: string;
  call_key: string | null;
  title: string;
  model_kind: string;
  provider: string | null;
  model: string | null;
  status: string;
  input_fingerprint_sha256: string | null;
  input_fingerprint_json: unknown | null;
  raw_response_json: unknown | null;
  parsed_output_json: unknown | null;
  validation_json: unknown | null;
  artifact_refs_json: unknown | null;
  attempt: number;
  response_summary_json: unknown | null;
  error_message: string | null;
  created_at: string;
  completed_at: string | null;
}

export interface WorkflowRunDetail {
  run: WorkflowRun;
  checkpoints: WorkflowCheckpoint[];
  generation_runs: GenerationRun[];
}

export interface ListWorkflowRunsInput {
  project_id?: number | null;
  workflow_kind?: string | null;
  status?: string | null;
  resumable_only?: boolean | null;
  limit?: number | null;
}

export async function listActiveWorkflows(): Promise<WorkflowState[]> {
  return apiInvoke<WorkflowState[]>("list_active_workflows", { userId: currentUserId() });
}

export async function listWorkflowRuns(
  input: ListWorkflowRunsInput = {},
): Promise<WorkflowRun[]> {
  return apiInvoke<WorkflowRun[]>("list_workflow_runs", {
    userId: currentUserId(),
    input,
  });
}

export async function getWorkflowRunDetail(workflowRunId: number): Promise<WorkflowRunDetail> {
  return apiInvoke<WorkflowRunDetail>("get_workflow_run_detail", {
    userId: currentUserId(),
    input: { workflow_run_id: workflowRunId },
  });
}

export async function resumeWorkflowRun(workflowRunId: number): Promise<WorkflowRun> {
  return apiInvoke<WorkflowRun>("resume_workflow_run", {
    userId: currentUserId(),
    input: { workflow_run_id: workflowRunId },
  });
}

export async function getWorkflowRunProgressEvents(
  workflowRunId: number,
  limit = 50,
): Promise<ProgressEventRecord[]> {
  return apiInvoke<ProgressEventRecord[]>("get_workflow_progress_events", {
    userId: currentUserId(),
    input: { workflow_run_id: workflowRunId, limit },
  });
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问工作流");
  return id;
};

export async function cancelWorkflow(input: string | {
  taskId?: string | null;
  workflowRunId?: number | null;
}): Promise<void> {
  const payload = typeof input === "string" ? { taskId: input, workflowRunId: null } : input;
  return apiInvoke<void>("cancel_workflow", {
    userId: currentUserId(),
    input: {
      task_id: payload.taskId ?? null,
      workflow_run_id: payload.workflowRunId ?? null,
    },
  });
}
