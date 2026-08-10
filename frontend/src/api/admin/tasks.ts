import { apiInvoke } from "../index";
import type { TaskQueueSummary } from "../generated/host";
import { type ApiId as Id, idNum } from "../shared";

export async function listPendingTasks(callerUserId: Id): Promise<TaskQueueSummary[]> {
  return apiInvoke<TaskQueueSummary[]>("list_pending_tasks", {
    callerUserId: idNum(callerUserId),
  });
}

export async function cancelAdminTask(callerUserId: Id, taskId: string): Promise<void> {
  return apiInvoke<void>("cancel_admin_task", {
    callerUserId: idNum(callerUserId),
    taskId,
  });
}
