import type { TaskInfo } from "../../../api/task";
import {
  progressDisplayModel,
  type ProgressDisplayModel,
} from "../../../utils/progressPresentation";

export const taskProgressDisplay = (task: TaskInfo): ProgressDisplayModel => {
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
    message: task.created_at ?? "",
    status: task.status === "cancelled" ? "cancelled" : "idle",
    fallbackStage: task.created_at == null ? "暂无进度" : "任务已创建",
  });
};
