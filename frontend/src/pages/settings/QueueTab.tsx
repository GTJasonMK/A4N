import React, { useCallback, useEffect, useRef, useState } from "react";
import { Activity, RefreshCw } from "lucide-react";
import { extractErrorMessage } from "../../api";
import {
  cancelTask,
  getQueueStats,
  getTaskHistory,
  type QueueStatsSummary,
  type TaskInfoSummary,
} from "../../api/task";
import {
  cancelWorkflow,
  listActiveWorkflows,
  listWorkflowRuns,
  resumeWorkflowRun,
  type WorkflowRunSummary,
  type WorkflowStateSummary,
} from "../../api/streaming";
import { useToast } from "../../components/feedback/Toast";
import { BookIconButton } from "../../components/ui/BookButton";
import { useAuthStore } from "../../store/auth";
import { useDirty } from "../../components/navigation/dirtyRegistry";
import { ActiveWorkflowsPanel } from "./queue/ActiveWorkflowsPanel";
import { QueueInfoCard } from "./queue/QueueInfoCard";
import { QueueStatsGrid } from "./queue/QueueStatsGrid";
import { TaskHistoryPanel } from "./queue/TaskHistoryPanel";
import { WorkflowRunsPanel } from "./queue/WorkflowRunsPanel";
import { useI18n, type MessageKey } from "../../i18n";

type QueueLoadError = { label: MessageKey; reason: unknown };

/**
 * 任务队列 Tab
 *
 * 页面负责轮询和取消命令；指标、任务历史、活跃工作流和手动提交器拆到
 * settings/queue 下，避免队列状态展示与高级提交表单混在一个文件。
 */
export const QueueTab: React.FC = () => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const user = useAuthStore((state) => state.user);
  const canReadQueueDiagnostics = user?.role === "admin";
  const [stats, setStats] = useState<QueueStatsSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [tasks, setTasks] = useState<TaskInfoSummary[]>([]);
  const [tasksLoading, setTasksLoading] = useState(true);
  const [cancellingTaskIds, setCancellingTaskIds] = useState<Set<string>>(() => new Set());
  const [workflows, setWorkflows] = useState<WorkflowStateSummary[]>([]);
  const [workflowRuns, setWorkflowRuns] = useState<WorkflowRunSummary[]>([]);
  const [loadErrors, setLoadErrors] = useState<QueueLoadError[]>([]);
  const [resumingWorkflowRunIds, setResumingWorkflowRunIds] = useState<Set<number>>(() => new Set());
  const mountedRef = useRef(true);
  const tasksRef = useRef(tasks);
  const workflowsRef = useRef(workflows);
  const workflowRunsRef = useRef(workflowRuns);
  const cancellingTaskIdsRef = useRef(cancellingTaskIds);
  const resumingWorkflowRunIdsRef = useRef(resumingWorkflowRunIds);
  const loadingRef = useRef(loading);
  const tasksLoadingRef = useRef(tasksLoading);
  const loadErrorsRef = useRef(loadErrors);
  const queueOperationInFlightRef = useRef(false);
  const refreshSeqRef = useRef(0);
  const refreshBusyRef = useRef(false);

  tasksRef.current = tasks;
  workflowsRef.current = workflows;
  workflowRunsRef.current = workflowRuns;
  cancellingTaskIdsRef.current = cancellingTaskIds;
  resumingWorkflowRunIdsRef.current = resumingWorkflowRunIds;
  loadingRef.current = loading;
  tasksLoadingRef.current = tasksLoading;
  loadErrorsRef.current = loadErrors;
  const queueOperationInFlight =
    cancellingTaskIds.size > 0 || resumingWorkflowRunIds.size > 0;
  queueOperationInFlightRef.current = queueOperationInFlight;
  const ensureReadModel = (label: MessageKey) => {
    if (loadingRef.current || (label === "settingsQueue.recentTasks" && tasksLoadingRef.current)) {
      addToast(t("common.operationInProgress"), "info");
      return false;
    }
    const error = loadErrorsRef.current.find((item) => item.label === label);
    if (!error) return true;
    addToast(extractErrorMessage(error.reason, t("settingsQueue.loadQueueFailed")), "error");
    return false;
  };
  useDirty(
    "settings-queue-operations",
    queueOperationInFlight,
    undefined,
    () =>
      cancellingTaskIdsRef.current.size === 0 &&
      resumingWorkflowRunIdsRef.current.size === 0,
  );

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
      refreshBusyRef.current = false;
    };
  }, []);

  const isCurrentRefresh = useCallback((seq: number) => {
    return mountedRef.current && refreshSeqRef.current === seq;
  }, []);

  const refresh = useCallback(async (options?: { force?: boolean }): Promise<boolean> => {
    if (!canReadQueueDiagnostics) {
      refreshBusyRef.current = false;
      setStats(null);
      setTasks([]);
      setWorkflows([]);
      setWorkflowRuns([]);
      setLoadErrors([]);
      setLoading(false);
      setTasksLoading(false);
      return false;
    }
    if (queueOperationInFlightRef.current && !options?.force) {
      return false;
    }
    if (refreshBusyRef.current && !options?.force) return false;
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    refreshBusyRef.current = true;
    setLoading(true);
    setTasksLoading(true);
    try {
      const [statsResult, tasksResult, workflowsResult, runsResult] = await Promise.allSettled([
        getQueueStats(),
        getTaskHistory(null, 0, 30),
        listActiveWorkflows(),
        listWorkflowRuns({ resumable_only: false, limit: 40 }),
      ]);
      if (!isCurrentRefresh(requestSeq)) return false;
      const errors: QueueLoadError[] = [];
      if (statsResult.status === "fulfilled") setStats(statsResult.value);
      else {
        errors.push({ label: "settingsQueue.queueLoad", reason: statsResult.reason });
      }
      if (tasksResult.status === "fulfilled") setTasks(tasksResult.value);
      else {
        errors.push({ label: "settingsQueue.recentTasks", reason: tasksResult.reason });
      }
      if (workflowsResult.status === "fulfilled") setWorkflows(workflowsResult.value);
      else {
        errors.push({ label: "settingsQueue.activeWorkflows", reason: workflowsResult.reason });
      }
      if (runsResult.status === "fulfilled") setWorkflowRuns(runsResult.value);
      else {
        errors.push({ label: "settingsQueue.workflowRuns", reason: runsResult.reason });
      }
      setLoadErrors(errors);
      return errors.length === 0;
    } finally {
      if (refreshSeqRef.current === requestSeq) {
        refreshBusyRef.current = false;
      }
      if (isCurrentRefresh(requestSeq)) {
        setLoading(false);
        setTasksLoading(false);
      }
    }
  }, [canReadQueueDiagnostics, isCurrentRefresh]);

  const markTaskCancelling = (taskId: string) => {
    const next = new Set(cancellingTaskIdsRef.current);
    next.add(taskId);
    cancellingTaskIdsRef.current = next;
    setCancellingTaskIds(next);
  };

  const clearTaskCancelling = (taskId: string) => {
    const next = new Set(cancellingTaskIdsRef.current);
    next.delete(taskId);
    cancellingTaskIdsRef.current = next;
    setCancellingTaskIds(next);
  };

  const markWorkflowRunResuming = (runId: number) => {
    const next = new Set(resumingWorkflowRunIdsRef.current);
    next.add(runId);
    resumingWorkflowRunIdsRef.current = next;
    setResumingWorkflowRunIds(next);
  };

  const clearWorkflowRunResuming = (runId: number) => {
    const next = new Set(resumingWorkflowRunIdsRef.current);
    next.delete(runId);
    resumingWorkflowRunIdsRef.current = next;
    setResumingWorkflowRunIds(next);
  };

  const handleCancel = async (taskId: string) => {
    if (!ensureReadModel("settingsQueue.recentTasks")) return;
    if (cancellingTaskIdsRef.current.has(taskId)) return;
    if (!tasksRef.current.some((task) => task.id === taskId)) {
      addToast(t("settingsQueue.taskMissing"), "info");
      return;
    }
    const requestRefreshSeq = refreshSeqRef.current;
    markTaskCancelling(taskId);
    try {
      const cancelled = await cancelTask(taskId);
      if (!mountedRef.current) return;
      if (refreshSeqRef.current !== requestRefreshSeq) {
        void refresh({ force: true });
        return;
      }
      if (cancelled) setTasks((current) => current.filter((task) => task.id !== taskId));
      addToast(
        cancelled
          ? t("settingsQueue.taskCancelled")
          : t("settingsQueue.alreadyFinished"),
        cancelled ? "success" : "info",
      );
      void refresh({ force: true }).then((refreshed) => {
        if (mountedRef.current && !refreshed) addToast(t("settingsQueue.loadQueueFailed"), "error");
      });
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(extractErrorMessage(err, t("settingsQueue.cancelTaskFailed")), "error");
    } finally {
      if (mountedRef.current) clearTaskCancelling(taskId);
    }
  };

  const handleCancelWorkflow = async (taskId: string) => {
    if (!ensureReadModel("settingsQueue.activeWorkflows")) return;
    if (cancellingTaskIdsRef.current.has(taskId)) return;
    if (!workflowsRef.current.some((workflow) => workflow.task_id === taskId)) {
      addToast(t("settingsQueue.workflowMissing"), "info");
      return;
    }
    const requestRefreshSeq = refreshSeqRef.current;
    markTaskCancelling(taskId);
    try {
      const cancelled = await cancelWorkflow({ task_id: taskId, workflow_run_id: null });
      if (!mountedRef.current) return;
      if (refreshSeqRef.current !== requestRefreshSeq) {
        void refresh({ force: true });
        return;
      }
      if (cancelled) {
        setWorkflows((current) => current.filter((workflow) => workflow.task_id !== taskId));
        setWorkflowRuns((current) => current.map((run) =>
          run.active_task_id === taskId
            ? { ...run, active_task_id: null, can_resume: false, status: "cancelled" }
            : run,
        ));
      }
      addToast(
        cancelled
          ? t("settingsQueue.workflowCancelled")
          : t("settingsQueue.alreadyFinished"),
        cancelled ? "success" : "info",
      );
      void refresh({ force: true }).then((refreshed) => {
        if (mountedRef.current && !refreshed) addToast(t("settingsQueue.loadQueueFailed"), "error");
      });
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(extractErrorMessage(err, t("settingsQueue.cancelWorkflowFailed")), "error");
    } finally {
      if (mountedRef.current) clearTaskCancelling(taskId);
    }
  };

  const handleResumeWorkflowRun = async (run: WorkflowRunSummary) => {
    if (!ensureReadModel("settingsQueue.workflowRuns")) return;
    if (resumingWorkflowRunIdsRef.current.has(run.id)) return;
    if (!workflowRunsRef.current.some((item) => item.id === run.id)) {
      addToast(t("settingsQueue.workflowMissing"), "info");
      return;
    }
    const requestRefreshSeq = refreshSeqRef.current;
    markWorkflowRunResuming(run.id);
    try {
      const resumed = await resumeWorkflowRun(run.id);
      if (!mountedRef.current) return;
      if (refreshSeqRef.current !== requestRefreshSeq) {
        void refresh({ force: true });
        return;
      }
      setWorkflowRuns((current) => current.map((item) => item.id === resumed.id ? resumed : item));
      addToast(t("settingsQueue.workflowResumed"), "success");
      void refresh({ force: true }).then((refreshed) => {
        if (mountedRef.current && !refreshed) addToast(t("settingsQueue.loadQueueFailed"), "error");
      });
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(extractErrorMessage(err, t("settingsQueue.resumeWorkflowFailed")), "error");
    } finally {
      if (mountedRef.current) clearWorkflowRunResuming(run.id);
    }
  };

  useEffect(() => {
    void refresh();
    if (!canReadQueueDiagnostics) return;
    const timer = window.setInterval(() => {
      void refresh();
    }, 5000);
    return () => clearInterval(timer);
  }, [canReadQueueDiagnostics, refresh]);

  if (!canReadQueueDiagnostics) {
    return (
      <div className="border-l-2 border-book-border/70 pl-3 text-sm text-book-text-muted">
        {t("settingsQueue.adminOnly")}
      </div>
    );
  }

  return (
    <div className="custom-scrollbar flex h-auto flex-col gap-3 overflow-visible md:h-full md:overflow-y-auto md:pr-2">
      <div className="flex shrink-0 items-center justify-end gap-2 text-small text-book-text-muted">
        <span className="inline-flex items-center gap-1.5">
          <Activity size={12} className="text-book-primary" />
          {t("settingsQueue.autoRefresh")}
        </span>
        <BookIconButton
          label={t("settingsQueue.refreshNow")}
          onClick={() => void refresh()}
          variant="secondary"
          size="sm"
          disabled={loading || queueOperationInFlight}
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>

      {loadErrors.length > 0 ? (
        <div role="alert" className="space-y-1 border-l-2 border-error px-3 py-2 text-xs text-error">
          {loadErrors.map(({ label, reason }) => (
            <p key={label}>
              {t(label)}: {extractErrorMessage(reason, t("settingsQueue.loadQueueFailed"))}
            </p>
          ))}
        </div>
      ) : null}

      <QueueStatsGrid stats={stats} />

      <ActiveWorkflowsPanel
        workflows={workflows}
        loading={loading}
        cancellingTaskIds={cancellingTaskIds}
        onCancel={(taskId) => void handleCancelWorkflow(taskId)}
      />

      <TaskHistoryPanel
        tasks={tasks}
        loading={tasksLoading}
        cancellingTaskIds={cancellingTaskIds}
        onCancel={(taskId) => void handleCancel(taskId)}
      />

      <WorkflowRunsPanel
        runs={workflowRuns}
        loading={loading}
        resumingRunIds={resumingWorkflowRunIds}
        onResume={(run) => void handleResumeWorkflowRun(run)}
      />

      <QueueInfoCard />
    </div>
  );
};
