import { apiInvoke } from "../index";
import type { PendingTaskIndexSummary, TaskQueueSummary } from "../generated/host";

export async function listPendingTasks(): Promise<TaskQueueSummary[]> {
  return apiInvoke<TaskQueueSummary[]>("list_pending_tasks");
}

export async function listPendingTaskIndex(): Promise<PendingTaskIndexSummary[]> {
  return apiInvoke<PendingTaskIndexSummary[]>("list_pending_task_index");
}

export async function cancelAdminTask(taskId: string): Promise<boolean> {
  return apiInvoke<boolean>("cancel_admin_task", {
    taskId,
  });
}
