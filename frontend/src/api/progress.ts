import { apiInvoke } from "./index";
import type {
  ProgressMeta as HostProgressMeta,
  WorkflowProgressEventsInput,
  WorkflowProgressEvent as HostWorkflowProgressEvent,
  WorkflowProgressEventsResponse as HostWorkflowProgressEventsResponse,
} from "./generated/host";

/**
 * 展示层进度阶段。
 *
 * Host 与 generation 传输合同均由 `a4n-domain::progress::ProgressPhaseKind`
 * 生成，前端不再维护第二套阶段语义。
 */
export type ProgressDisplayPhase = HostProgressMeta["phaseKind"];

export type ProgressMeta = HostProgressMeta;

export interface ProgressSnapshot {
  current: number;
  total: number;
  message: string;
  percent?: number | null;
  meta?: ProgressMeta | null;
  updated_at?: string | null;
}

export type ProgressStatus = "idle" | "running" | "success" | "error" | "cancelled";

export interface ProgressTimelineItem extends ProgressSnapshot {
  id: string;
  taskId?: string;
  status: ProgressStatus;
  timestamp: number;
}

export type ProgressEventRecord = HostWorkflowProgressEvent;

/**
 * `get_workflow_progress_events` 的响应。
 *
 * `events` 是去重后的历史里程碑；`stream_text` 是当前快照里的活体信号
 * （模型输出尾巴），每次轮询取最新值覆盖，不进事件历史。
 */
export type WorkflowProgressEventsResponse = HostWorkflowProgressEventsResponse;

export async function getWorkflowProgressEvents(
  input: WorkflowProgressEventsInput,
): Promise<WorkflowProgressEventsResponse> {
  return apiInvoke<WorkflowProgressEventsResponse>("get_workflow_progress_events", { input });
}
