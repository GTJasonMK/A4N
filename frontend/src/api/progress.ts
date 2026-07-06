export type ProgressPhaseKind =
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

export type ProgressSeverity = "info" | "success" | "warning" | "error";

export interface ProgressMeta {
  taskKind: string;
  stageCode: string;
  stageLabel: string;
  phaseKind: ProgressPhaseKind;
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
