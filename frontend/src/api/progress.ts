/**
 * 展示层进度阶段。
 *
 * 注意：与 `generated/generationContracts.ts` 的 `ProgressPhaseKind`（传输层，
 * 粗粒度）不是同一层概念，故不同名。传输层事件经
 * `utils/generationProgressMeta.ts` 按 stage_code 映射到本类型。
 */
export type ProgressDisplayPhase =
  | "prepare"
  | "context"
  | "model_connect"
  | "model_wait"
  | "model_stream"
  | "structured_output"
  | "parse"
  | "validate"
  | "save"
  | "asset_generation"
  | "transfer"
  | "queue"
  | "cleanup"
  | "complete";

type ProgressSeverity = "info" | "success" | "warning" | "error";

export interface ProgressMeta {
  taskKind: string;
  stageCode: string;
  stageLabel: string;
  phaseKind: ProgressDisplayPhase;
  detail?: string | null;
  unitLabel?: string | null;
  itemIndex?: number | null;
  itemTotal?: number | null;
  parentStageCode?: string | null;
  severity?: ProgressSeverity;
  estimated?: boolean;
  persistent?: boolean;
}

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

export interface ProgressEventRecord extends ProgressSnapshot {
  id: string;
  task_id: string | null;
  timestamp: string;
}

/**
 * `get_workflow_progress_events` 的响应。
 *
 * `events` 是去重后的历史里程碑；`stream_text` 是当前快照里的活体信号
 * （模型输出尾巴），每次轮询取最新值覆盖，不进事件历史。
 */
export interface WorkflowProgressEventsResponse {
  events: ProgressEventRecord[];
  stream_text?: string | null;
}
