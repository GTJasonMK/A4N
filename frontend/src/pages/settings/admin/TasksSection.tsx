import React, { useCallback, useEffect, useRef, useState } from "react";
import { RefreshCw, X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import { cancelAdminTask, listPendingTasks, type AdminTaskInfo } from "../../../api/admin";
import { useToast } from "../../../components/feedback/Toast";
import { BookIconButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import {
  getTaskPriorityLabel,
  getTaskTypeLabel,
  getOperationStatusLabel,
} from "../../../utils/workflowPresentation";
import { useAdminSectionLoader } from "./useAdminSectionLoader";

const formatTaskDate = (value: string | null): string => {
  if (!value) return "";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return "时间未知";
  return parsed.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
};

export const TasksSection: React.FC<{ callerUserId: number }> = ({ callerUserId }) => {
  const { addToast } = useToast();
  const [tasks, setTasks] = useState<AdminTaskInfo[]>([]);
  const [cancellingIds, setCancellingIds] = useState<Set<string>>(new Set());
  const tasksRef = useRef(tasks);
  const cancellingIdsRef = useRef(cancellingIds);

  tasksRef.current = tasks;
  cancellingIdsRef.current = cancellingIds;

  const loadTasks = useCallback(() => listPendingTasks(callerUserId), [callerUserId]);
  const { loading, load, isCurrentScope: isCurrentCaller } =
    useAdminSectionLoader(callerUserId, loadTasks, setTasks, "加载失败", {
      intervalMs: 5000,
    });

  useEffect(() => {
    setTasks([]);
    cancellingIdsRef.current = new Set();
    setCancellingIds(new Set());
  }, [callerUserId]);

  const setTaskCancelling = (id: string, value: boolean) => {
    const next = new Set(cancellingIdsRef.current);
    if (value) next.add(id);
    else next.delete(id);
    cancellingIdsRef.current = next;
    setCancellingIds(next);
  };

  const handleCancel = async (id: string) => {
    const requestCallerUserId = callerUserId;
    if (cancellingIdsRef.current.has(id)) return;
    if (!tasksRef.current.some((task) => task.id === id)) {
      addToast("任务已不在待处理列表中", "info");
      return;
    }
    setTaskCancelling(id, true);
    try {
      await cancelAdminTask(requestCallerUserId, id);
      if (!isCurrentCaller(requestCallerUserId)) return;
      addToast("已取消任务", "success");
      await load();
    } catch (err) {
      if (!isCurrentCaller(requestCallerUserId)) return;
      addToast(extractErrorMessage(err, "取消失败"), "error");
    } finally {
      if (isCurrentCaller(requestCallerUserId)) setTaskCancelling(id, false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex min-h-8 items-center gap-2 text-xs text-book-text-muted">
        <span>{tasks.length} 个待处理任务 · 每 5 秒更新</span>
        <span className="flex-1" />
        <BookIconButton label="立即刷新任务" onClick={load} size="sm" disabled={loading}>
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      {loading && tasks.length === 0 ? (
        <div className="a4n-flat-surface inline-flex items-center justify-center gap-1.5 py-4 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" />
          正在加载待处理任务…
        </div>
      ) : tasks.length === 0 ? (
        <div className="a4n-empty-state a4n-empty-state--framed text-xs">
          无待处理任务
        </div>
      ) : (
        <ul className="space-y-2.5">
          {tasks.map((t) => (
            <li key={t.id} className="a4n-flat-surface p-3 text-xs">
              <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3">
                <div className="min-w-0">
                  <div className="truncate font-medium text-book-text-main">
                    {getTaskTypeLabel(t.task_type)}
                  </div>
                  <div className="mt-1 truncate text-small text-book-text-muted">
                    {[getOperationStatusLabel(t.status), getTaskPriorityLabel(t.priority), formatTaskDate(t.created_at)]
                      .filter(Boolean)
                      .join(" · ")}
                  </div>
                </div>
                <span className="ml-auto">
                  {cancellingIds.has(t.id) ? (
                    <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
                      <LoadingSpinner size="sm" />
                    </span>
                  ) : (
                    <Dropdown
                      items={[
                        {
                          label: "取消任务",
                          icon: <X size={12} />,
                          danger: true,
                          onClick: () => void handleCancel(t.id),
                        },
                      ]}
                    />
                  )}
                </span>
              </div>
              <details className="mt-1 text-small text-book-text-muted">
                <summary className="w-fit cursor-pointer select-none rounded px-1 py-0.5 hover:bg-book-bg-tertiary hover:text-book-text-main">
                  诊断信息
                </summary>
                <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1 rounded-md bg-book-bg-tertiary/45 p-2 font-mono">
                  <span title={t.id}>ID {t.id}</span>
                  {t.project_id != null ? <span>项目 {t.project_id}</span> : null}
                  {t.task_scope ? <span>范围 {t.task_scope}</span> : null}
                  {t.root_project_id != null ? <span>根项目 {t.root_project_id}</span> : null}
                  {t.source_snapshot_id != null ? <span>源快照 {t.source_snapshot_id}</span> : null}
                  {t.dag_id ? <span>DAG {t.dag_id}</span> : null}
                </div>
              </details>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
