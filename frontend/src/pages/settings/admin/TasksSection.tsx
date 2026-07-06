import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import { cancelAdminTask, listPendingTasks, type AdminTaskInfo } from "../../../api/admin";
import { useToast } from "../../../components/feedback/Toast";
import { BookIconButton } from "../../../components/ui/BookButton";
import { useAdminSectionLoader } from "./useAdminSectionLoader";

export const TasksSection: React.FC<{ callerUserId: number }> = ({ callerUserId }) => {
  const { addToast } = useToast();
  const [tasks, setTasks] = useState<AdminTaskInfo[]>([]);
  const [cancellingIds, setCancellingIds] = useState<Set<string>>(new Set());
  const activeCallerUserIdRef = useRef(callerUserId);
  const mountedRef = useRef(true);
  const tasksRef = useRef(tasks);
  const cancellingIdsRef = useRef(cancellingIds);
  const loadSeqRef = useRef(0);

  activeCallerUserIdRef.current = callerUserId;
  tasksRef.current = tasks;
  cancellingIdsRef.current = cancellingIds;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const isCurrentCaller = useCallback((requestCallerUserId: number) => {
    return mountedRef.current && activeCallerUserIdRef.current === requestCallerUserId;
  }, []);

  const isCurrentLoad = useCallback((requestCallerUserId: number, seq: number) => {
    return isCurrentCaller(requestCallerUserId) && loadSeqRef.current === seq;
  }, [isCurrentCaller]);

  const loadTasks = useCallback(async () => {
    const requestCallerUserId = callerUserId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    try {
      const nextTasks = await listPendingTasks(requestCallerUserId);
      if (!isCurrentLoad(requestCallerUserId, requestSeq)) return;
      setTasks(nextTasks);
    } catch (err) {
      if (!isCurrentLoad(requestCallerUserId, requestSeq)) return;
      throw err;
    }
  }, [callerUserId, isCurrentLoad]);
  const { loading, load } = useAdminSectionLoader(loadTasks, "加载失败", {
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
      addToast(`已取消任务 ${id.slice(0, 8)}…`, "success");
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
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-serif text-book-text-main">
          全局待处理任务（{tasks.length}，每 5s 刷新）
        </h4>
        <BookIconButton label="刷新" onClick={load} variant="secondary" size="sm" disabled={loading}>
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      {loading && tasks.length === 0 ? (
        <div className="inline-flex items-center justify-center gap-1.5 py-4 text-xs text-book-text-muted pyqt-card">
          <Loader2 size={12} className="animate-spin" />
          正在加载待处理任务…
        </div>
      ) : tasks.length === 0 ? (
        <div className="text-xs text-book-text-muted py-4 text-center pyqt-card">
          无待处理任务
        </div>
      ) : (
        <ul className="pyqt-card divide-y divide-book-border/30">
          {tasks.map((t) => (
            <li key={t.id} className="p-2.5 flex items-center gap-2 text-xs">
              <span className="font-mono text-[10px] text-book-text-muted shrink-0 w-16 truncate">
                {t.id.slice(0, 12)}
              </span>
              <span className="pyqt-pill !text-[10px]">{t.task_type}</span>
              <span className="pyqt-pill !text-[10px]">{t.priority}</span>
              <span className="pyqt-pill !text-[10px]">{t.status}</span>
              {t.project_id != null && (
                <span className="text-book-text-muted">proj #{t.project_id}</span>
              )}
              {t.task_scope && (
                <span className="pyqt-pill !text-[10px]">{t.task_scope}</span>
              )}
              {t.root_project_id != null && (
                <span className="text-book-text-muted">root #{t.root_project_id}</span>
              )}
              {t.source_snapshot_id != null && (
                <span className="text-book-text-muted">snap #{t.source_snapshot_id}</span>
              )}
              {t.dag_id && (
                <span className="text-book-text-muted">DAG {t.dag_id.slice(0, 8)}</span>
              )}
              <span className="flex-1" />
              <button
                type="button"
                onClick={() => void handleCancel(t.id)}
                disabled={cancellingIds.has(t.id)}
                className="text-[11px] text-error px-2 py-0.5 rounded hover:bg-error-bg"
              >
                {cancellingIds.has(t.id) ? "…" : "取消"}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
