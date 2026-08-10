import type { TaskInfoSummary } from "../../../api/task";
import {
  progressDisplayModel,
  type ProgressDisplayModel,
} from "../../../utils/progressPresentation";

export const taskProgressDisplay = (task: TaskInfoSummary): ProgressDisplayModel => {
  if (task.progress?.message?.trim()) {
    return progressDisplayModel({
      ...task.progress,
      status: task.status === "completed" ? "success" : task.status === "failed" ? "error" : "running",
      fallbackStage: task.status === "completed" ? "任务已完成" : "任务执行中",
    });
  }
  return progressDisplayModel({
    current: 0,
    total: 1,
    message: formatTaskCreatedAt(task.created_at),
    status: task.status === "cancelled" ? "cancelled" : "idle",
    fallbackStage: "任务已创建",
  });
};

const formatTaskCreatedAt = (value: string): string => {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString([], {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
};
