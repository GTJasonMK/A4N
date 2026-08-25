import type { TaskInfoSummary } from "../../../api/task";
import {
  progressDisplayModel,
  type ProgressDisplayModel,
} from "../../../utils/progressPresentation";
import type { MessageKey, MessageParams } from "../../../i18n";
import { formatDateTime, type Locale } from "../../../i18n";

type Translate = (key: MessageKey, params?: MessageParams) => string;

export const taskProgressDisplay = (
  task: TaskInfoSummary,
  t: Translate,
  locale: Locale,
): ProgressDisplayModel => {
  if (task.progress?.message?.trim()) {
    return progressDisplayModel({
      ...task.progress,
      locale,
      t,
      status: task.status === "completed" ? "success" : task.status === "failed" ? "error" : "running",
      fallbackStage: task.status === "completed"
        ? t("workflow.success")
        : t("settingsQueue.taskRunning"),
    });
  }
  return progressDisplayModel({
    current: 0,
    total: 1,
    message: formatDateTime(task.created_at, locale, task.created_at),
    locale,
    t,
    status: task.status === "cancelled" ? "cancelled" : "idle",
    fallbackStage: t("workflow.ready"),
  });
};
