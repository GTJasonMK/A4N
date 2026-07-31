import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import { idNum, type ApiId as Id } from "./shared";
import type { ProgressEventRecord, ProgressSnapshot } from "./progress";

/**
 * Task API wrapper —— 对齐 crates/a4n-tauri/src/commands/task.rs.
 *
 * 关键约定：
 *  - task_type: "llm_call" / "image_generation" / "video_generation" / "video_poll" / "source_sync" / "assembly" / "embedding_call" / "rag_ingestion" / "import_analysis" / "export_generation" / "embedding_download"
 *  - status: "pending" / "running" / "completed" / "failed" / "cancelled"
 *  - get_task_history 用 (project_id?, offset, limit) 完全平铺
 */

interface TaskProgress extends ProgressSnapshot {}

export interface TaskInfo {
  id: string;
  task_type: string;
  priority: string;
  status: string;
  project_id: number | null;
  root_project_id: number | null;
  source_snapshot_id: number | null;
  task_scope: string | null;
  progress: TaskProgress | null;
  progress_events?: ProgressEventRecord[];
  created_at: string | null;
  started_at: string | null;
  completed_at: string | null;
}

interface QueueSlotStats {
  max_concurrent: number;
  running: number;
  waiting: number;
}

export interface QueueStats {
  llm_queue: QueueSlotStats;
  image_queue: QueueSlotStats;
  video_queue: QueueSlotStats;
  embedding_queue: QueueSlotStats;
  background_queue: QueueSlotStats;
  high_priority_waiting: number;
  medium_priority_waiting: number;
  low_priority_waiting: number;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问任务");
  return id;
};

/** 后端 cancel_task(task_id) */
export async function cancelTask(taskId: string): Promise<void> {
  return apiInvoke<void>("cancel_task", { userId: currentUserId(), taskId });
}

// ============================================================================
// 统计与历史
// ============================================================================

export async function getQueueStats(): Promise<QueueStats> {
  return apiInvoke<QueueStats>("get_queue_stats", { userId: currentUserId() });
}

/** 后端 get_task_history(project_id?, offset, limit) —— 完全平铺 */
export async function getTaskHistory(
  projectId: Id | null,
  offset: number,
  limit: number,
): Promise<TaskInfo[]> {
  return apiInvoke<TaskInfo[]>("get_task_history", {
    userId: currentUserId(),
    projectId: projectId == null ? null : idNum(projectId),
    offset,
    limit,
  });
}

export async function getTaskProgressEvents(
  taskId: string,
  limit = 50,
): Promise<ProgressEventRecord[]> {
  return apiInvoke<ProgressEventRecord[]>("get_workflow_progress_events", {
    userId: currentUserId(),
    input: { task_id: taskId, workflow_run_id: null, limit },
  });
}
