import React from "react";
import { X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { Dropdown } from "../../../components/ui/Dropdown";
import type { TaskInfoSummary } from "../../../api/task";
import { taskProgressDisplay } from "./queueTabUtils";
import {
  getOperationStatusLabel,
  getTaskPriorityLabel,
  getTaskTypeLabel,
} from "../../../utils/workflowPresentation";
import { useI18n } from "../../../i18n";
import { formatDateTime } from "../../../i18n";

interface TaskHistoryPanelProps {
  tasks: TaskInfoSummary[];
  loading: boolean;
  cancellingTaskIds: ReadonlySet<string>;
  onCancel: (taskId: string) => void;
}

export const TaskHistoryPanel: React.FC<TaskHistoryPanelProps> = ({
  tasks,
  loading,
  cancellingTaskIds,
  onCancel,
}) => {
  const { t, locale } = useI18n();
  return (
  <section>
    <div className="flex items-baseline gap-2 px-1 pb-2">
      <h3 className="text-xs font-semibold text-book-text-main">{t("settingsQueue.recentTasks")}</h3>
      <span className="text-small text-book-text-muted">
        {tasks.length > 0 ? t("settingsQueue.recentTaskCount", { count: tasks.length }) : t("settingsQueue.executionRecords")}
      </span>
      {loading ? <LoadingSpinner size="sm" className="ml-auto  text-book-text-muted" /> : null}
    </div>
    {loading && tasks.length === 0 ? (
      <div className="a4n-flat-surface flex items-center justify-center gap-1.5 px-3 py-5 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" />
        {t("settingsQueue.loadHistory")}
      </div>
    ) : tasks.length === 0 ? (
      <div className="a4n-empty-state a4n-empty-state--framed px-3 py-5 text-center text-xs text-book-text-muted">
        {t("settingsQueue.noTasks")}
      </div>
    ) : (
      <ul className="space-y-2.5">
        {tasks.map((task) => {
          const canCancel = task.status === "pending" || task.status === "running";
          const progressDisplay = taskProgressDisplay(task, t, locale);
          const cancelling = cancellingTaskIds.has(task.id);
          const taskTime = formatDateTime(task.completed_at ?? task.started_at ?? task.created_at, locale, t("common.unknown"));
          return (
            <li key={task.id} className="a4n-flat-surface px-3 py-3 text-xs">
              <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3">
                <div className="min-w-0">
                  <div className="truncate font-medium text-book-text-main">
                    {getTaskTypeLabel(task.task_type, locale)}
                  </div>
                  <div className="mt-0.5 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-small text-book-text-muted">
                    <span className={task.priority === "high" ? "text-error" : undefined}>
                      {getTaskPriorityLabel(task.priority, locale)}
                    </span>
                    <span aria-hidden="true">·</span>
                    <span className={taskStatusTone(task.status)}>
                      {getOperationStatusLabel(task.status, locale)}
                    </span>
                    {task.progress ? (
                      <>
                        <span aria-hidden="true">·</span>
                        <span>{task.progress.current} / {task.progress.total}</span>
                      </>
                    ) : null}
                    {taskTime ? (
                      <>
                        <span aria-hidden="true">·</span>
                        <time>{taskTime}</time>
                      </>
                    ) : null}
                  </div>
                  <div className="mt-1.5 min-w-0">
                    <span className="block truncate text-small text-book-text-sub">
                      {progressDisplay.stage}
                    </span>
                    {progressDisplay.detail ? (
                      <span className="block truncate text-small leading-4 text-book-text-muted">
                        {progressDisplay.detail}
                      </span>
                    ) : null}
                  </div>
                </div>
                {cancelling ? (
                  <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
                    <LoadingSpinner size="sm" />
                  </span>
                ) : canCancel ? (
                  <Dropdown
                    items={[
                      {
                        label: t("settingsQueue.cancelTask"),
                        icon: <X size={13} />,
                        danger: true,
                        onClick: () => onCancel(task.id),
                      },
                    ]}
                  />
                ) : null}
              </div>
              <details className="mt-1 text-caption text-book-text-muted">
                <summary className="cursor-pointer">{t("settingsQueue.diagnostic")}</summary>
                <div className="mt-1 flex flex-wrap gap-x-4 gap-y-1 font-mono">
                  <span>{t("settingsQueue.taskId", { id: task.id })}</span>
                  {task.task_scope ? <span>{t("settingsQueue.taskScope", { scope: task.task_scope })}</span> : null}
                  {task.root_project_id != null ? <span>{t("settingsQueue.rootProject", { id: task.root_project_id })}</span> : null}
                  {task.source_snapshot_id != null ? <span>{t("settingsQueue.sourceSnapshot", { id: task.source_snapshot_id })}</span> : null}
                </div>
              </details>
            </li>
          );
        })}
      </ul>
    )}
  </section>
  );
};

function taskStatusTone(status: string): string {
  if (status === "completed") return "text-success";
  if (status === "failed") return "text-error";
  if (status === "running") return "text-book-primary";
  return "text-book-text-muted";
}
