import React, { useCallback, useRef, useState } from "react";
import { RefreshCw, X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  cancelAdminTask,
  listPendingTasks,
  type TaskQueueSummary,
} from "../../../api/admin";
import { useToast } from "../../../components/feedback/Toast";
import { BookIconButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import {
  getTaskPriorityLabel,
  getTaskTypeLabel,
  getOperationStatusLabel,
} from "../../../utils/workflowPresentation";
import { useAdminSectionLoader } from "./useAdminSectionLoader";
import { formatDateTime, useI18n } from "../../../i18n";

export const TasksSection: React.FC = () => {
  const { addToast } = useToast();
  const { t, locale } = useI18n();
  const [tasks, setTasks] = useState<TaskQueueSummary[]>([]);
  const [cancellingIds, setCancellingIds] = useState<Set<string>>(new Set());
  const tasksRef = useRef(tasks);
  const cancellingIdsRef = useRef(cancellingIds);

  tasksRef.current = tasks;
  cancellingIdsRef.current = cancellingIds;

  const loadTasks = useCallback(() => listPendingTasks(), []);
  const { loading, error, load, reload } =
    useAdminSectionLoader("admin-tasks", loadTasks, setTasks, "settingsAdmin.loadFailed", {
      intervalMs: 5000,
    });

  const setTaskCancelling = (id: string, value: boolean) => {
    const next = new Set(cancellingIdsRef.current);
    if (value) next.add(id);
    else next.delete(id);
    cancellingIdsRef.current = next;
    setCancellingIds(next);
  };

  const handleCancel = async (id: string) => {
    if (cancellingIdsRef.current.has(id)) return;
    if (!tasksRef.current.some((task) => task.id === id)) {
      addToast(t("settingsAdmin.taskMissing"), "info");
      return;
    }
    setTaskCancelling(id, true);
    try {
      await cancelAdminTask(id);
      const refreshed = await reload();
      if (!refreshed) addToast(t("settingsAdmin.loadFailed"), "error");
      addToast(t("settingsAdmin.taskCancelled"), "success");
    } catch (err) {
      addToast(extractErrorMessage(err, t("settingsAdmin.cancelFailed")), "error");
    } finally {
      setTaskCancelling(id, false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex min-h-8 items-center gap-2 text-xs text-book-text-muted">
        <span>{t("settingsAdmin.pendingTasks", { count: tasks.length })}</span>
        <span className="flex-1" />
        <BookIconButton label={t("settingsAdmin.refreshTasks")} onClick={load} size="sm" disabled={loading}>
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      {error ? <p role="alert" className="text-xs text-error">{error}</p> : null}
      {loading && tasks.length === 0 ? (
        <div className="a4n-flat-surface inline-flex items-center justify-center gap-1.5 py-4 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" />
          {t("settingsAdmin.loadPendingTasks")}
        </div>
      ) : error && tasks.length === 0 ? null : tasks.length === 0 ? (
        <div className="a4n-empty-state a4n-empty-state--framed text-xs">
          {t("settingsAdmin.noPendingTasks")}
        </div>
      ) : (
        <ul className="space-y-2.5">
          {tasks.map((task) => (
            <li key={task.id} className="a4n-flat-surface p-3 text-xs">
              <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3">
                <div className="min-w-0">
                  <div className="truncate font-medium text-book-text-main">
                    {getTaskTypeLabel(task.task_type, locale)}
                  </div>
                  <div className="mt-1 truncate text-small text-book-text-muted">
                    {[getOperationStatusLabel(task.status, locale), getTaskPriorityLabel(task.priority, locale), formatDateTime(task.created_at, locale, t("settingsAdmin.unknownDate"))]
                      .filter(Boolean)
                      .join(" · ")}
                  </div>
                </div>
                <span className="ml-auto">
                  {cancellingIds.has(task.id) ? (
                    <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
                      <LoadingSpinner size="sm" />
                    </span>
                  ) : (
                    <Dropdown
                      items={[
                        {
                          label: t("settingsQueue.cancelTask"),
                          icon: <X size={12} />,
                          danger: true,
                          onClick: () => void handleCancel(task.id),
                        },
                      ]}
                    />
                  )}
                </span>
              </div>
              <details className="mt-1 text-small text-book-text-muted">
                <summary className="w-fit cursor-pointer select-none rounded px-1 py-0.5 hover:bg-book-bg-tertiary hover:text-book-text-main">
                  {t("settingsAdmin.diagnostics")}
                </summary>
                <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1 rounded-md bg-book-bg-tertiary/45 p-2 font-mono">
                  <span title={task.id}>ID {task.id}</span>
                  {task.project_id != null ? <span>{t("settingsAdmin.project", { id: task.project_id })}</span> : null}
                  {task.task_scope ? <span>{t("settingsAdmin.scope", { scope: task.task_scope })}</span> : null}
                  {task.root_project_id != null ? <span>{t("settingsAdmin.taskRootProject", { id: task.root_project_id })}</span> : null}
                  {task.source_snapshot_id != null ? <span>{t("settingsAdmin.taskSourceSnapshot", { id: task.source_snapshot_id })}</span> : null}
                  {task.dag_id ? <span>DAG {task.dag_id}</span> : null}
                </div>
              </details>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
