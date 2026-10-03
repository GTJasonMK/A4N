import { apiInvoke } from "../index";
import type { TaskQueueSummary } from "../generated/host";

export async function listPendingTasks(): Promise<TaskQueueSummary[]> {
  return apiInvoke<TaskQueueSummary[]>("list_pending_tasks");
}

export async function cancelAdminTask(taskId: string): Promise<boolean> {
  return apiInvoke<boolean>("cancel_admin_task", {
    taskId,
  });
}
