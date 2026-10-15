import { apiInvoke } from "./index";
import type {
  CancelTaskInput,
  GetTaskHistoryInput,
  QueueStatsSummary,
  TaskInfoSummary,
  WorkflowProgressEventsInput,
} from "./generated/host";
import { idNum, type ApiId as Id } from "./shared";
import {
  getWorkflowProgressEvents,
  type WorkflowProgressEventsResponse,
} from "./progress";

export type { QueueStatsSummary, TaskInfoSummary };

/**
 * Task API wrapper —— 对齐 crates/host/a4n-tauri/src/commands/task.rs.
 *
 * 关键约定：
 *  - task_type: "llm_call" / "image_generation" / "video_generation" / "video_poll" / "source_sync" / "assembly" / "embedding_call" / "rag_ingestion" / "import_analysis" / "export_generation" / "embedding_download"
 *  - status: "pending" / "running" / "completed" / "failed" / "cancelled"
 *  - get_task_history 用 (project_id?, offset, limit) 完全平铺
 */

/** 后端 cancel_task(task_id) */
export async function cancelTask(taskId: string): Promise<boolean> {
  return apiInvoke<boolean>("cancel_task", { taskId } satisfies CancelTaskInput);
}

// ============================================================================
// 统计与历史
// ============================================================================

export async function getQueueStats(): Promise<QueueStatsSummary> {
  return apiInvoke<QueueStatsSummary>("get_queue_stats");
}

/** 后端 get_task_history(project_id?, offset, limit) —— 完全平铺 */
export async function getTaskHistory(
  projectId: Id | null,
  offset: number,
  limit: number,
): Promise<TaskInfoSummary[]> {
  return apiInvoke<TaskInfoSummary[]>("get_task_history", {
    projectId: projectId == null ? null : idNum(projectId),
    offset,
    limit,
  } satisfies GetTaskHistoryInput);
}

export async function getTaskProgressEvents(
  taskId: string,
  limit: number,
  afterEventId?: string,
): Promise<WorkflowProgressEventsResponse> {
  const input: WorkflowProgressEventsInput = {
    task_id: taskId,
    workflow_run_id: null,
    limit,
  };
  if (afterEventId !== undefined) input.after_event_id = afterEventId;
  return getWorkflowProgressEvents(input);
}
