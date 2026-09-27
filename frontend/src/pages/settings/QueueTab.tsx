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
import { ActiveWorkflowsPanel } from "./queue/ActiveWorkflowsPanel";
import { QueueInfoCard } from "./queue/QueueInfoCard";
import { QueueStatsGrid } from "./queue/QueueStatsGrid";
import { TaskHistoryPanel } from "./queue/TaskHistoryPanel";
import { WorkflowRunsPanel } from "./queue/WorkflowRunsPanel";
import { useI18n } from "../../i18n";

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
  const [loading, setLoading] = useState(false);
  const [tasks, setTasks] = useState<TaskInfoSummary[]>([]);
  const [tasksLoading, setTasksLoading] = useState(false);
  const [cancellingTaskIds, setCancellingTaskIds] = useState<Set<string>>(() => new Set());
  const [workflows, setWorkflows] = useState<WorkflowStateSummary[]>([]);
  const [workflowRuns, setWorkflowRuns] = useState<WorkflowRunSummary[]>([]);
  const [loadErrors, setLoadErrors] = useState<string[]>([]);
  const [resumingWorkflowRunIds, setResumingWorkflowRunIds] = useState<Set<number>>(() => new Set());
  const [cancellingWorkflowIds, setCancellingWorkflowIds] = useState<Set<string>>(() => new Set());
  const mountedRef = useRef(true);
  const tasksRef = useRef(tasks);
  const workflowsRef = useRef(workflows);
  const cancellingTaskIdsRef = useRef(cancellingTaskIds);
  const cancellingWorkflowIdsRef = useRef(cancellingWorkflowIds);
  const resumingWorkflowRunIdsRef = useRef(resumingWorkflowRunIds);
  const refreshSeqRef = useRef(0);
  const refreshBusyRef = useRef(false);

  tasksRef.current = tasks;
  workflowsRef.current = workflows;
  cancellingTaskIdsRef.current = cancellingTaskIds;
  cancellingWorkflowIdsRef.current = cancellingWorkflowIds;
  resumingWorkflowRunIdsRef.current = resumingWorkflowRunIds;

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
      const errors: string[] = [];
      if (statsResult.status === "fulfilled") setStats(statsResult.value);
      else {
        errors.push(`${t("settingsQueue.queueLoad")}: ${extractErrorMessage(statsResult.reason, t("settingsQueue.loadQueueFailed"))}`);
      }
      if (tasksResult.status === "fulfilled") setTasks(tasksResult.value);
      else {
        errors.push(`${t("settingsQueue.recentTasks")}: ${extractErrorMessage(tasksResult.reason, t("settingsQueue.loadQueueFailed"))}`);
      }
      if (workflowsResult.status === "fulfilled") setWorkflows(workflowsResult.value);
      else {
        errors.push(`${t("settingsQueue.activeWorkflows")}: ${extractErrorMessage(workflowsResult.reason, t("settingsQueue.loadQueueFailed"))}`);
      }
      if (runsResult.status === "fulfilled") setWorkflowRuns(runsResult.value);
      else {
        errors.push(`${t("settingsQueue.workflowRuns")}: ${extractErrorMessage(runsResult.reason, t("settingsQueue.loadQueueFailed"))}`);
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
  }, [canReadQueueDiagnostics, isCurrentRefresh, t]);

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

  const markWorkflowCancelling = (taskId: string) => {
    const next = new Set(cancellingWorkflowIdsRef.current);
    next.add(taskId);
    cancellingWorkflowIdsRef.current = next;
    setCancellingWorkflowIds(next);
  };

  const clearWorkflowCancelling = (taskId: string) => {
    const next = new Set(cancellingWorkflowIdsRef.current);
    next.delete(taskId);
    cancellingWorkflowIdsRef.current = next;
    setCancellingWorkflowIds(next);
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
    if (cancellingTaskIdsRef.current.has(taskId)) return;
    if (!tasksRef.current.some((task) => task.id === taskId)) {
      addToast(t("settingsQueue.taskMissing"), "info");
      return;
    }
    markTaskCancelling(taskId);
    try {
      await cancelTask(taskId);
      if (!mountedRef.current) return;
      const refreshed = await refresh({ force: true });
      if (!refreshed) return;
      addToast(t("settingsQueue.taskCancelled"), "success");
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(extractErrorMessage(err, t("settingsQueue.cancelTaskFailed")), "error");
    } finally {
      if (mountedRef.current) clearTaskCancelling(taskId);
    }
  };

  const handleCancelWorkflow = async (taskId: string) => {
    if (cancellingWorkflowIdsRef.current.has(taskId)) return;
    if (!workflowsRef.current.some((workflow) => workflow.task_id === taskId)) {
      addToast(t("settingsQueue.workflowMissing"), "info");
      return;
    }
    markWorkflowCancelling(taskId);
    try {
      await cancelWorkflow({ task_id: taskId, workflow_run_id: null });
      if (!mountedRef.current) return;
      const refreshed = await refresh({ force: true });
      if (!refreshed) return;
      addToast(t("settingsQueue.workflowCancelled"), "success");
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(extractErrorMessage(err, t("settingsQueue.cancelWorkflowFailed")), "error");
    } finally {
      if (mountedRef.current) clearWorkflowCancelling(taskId);
    }
  };

  const handleResumeWorkflowRun = async (run: WorkflowRunSummary) => {
    if (resumingWorkflowRunIdsRef.current.has(run.id)) return;
    markWorkflowRunResuming(run.id);
    try {
      await resumeWorkflowRun(run.id);
      if (!mountedRef.current) return;
      const refreshed = await refresh({ force: true });
      if (!refreshed) return;
      addToast(t("settingsQueue.workflowResumed"), "success");
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
          disabled={loading}
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>

      {loadErrors.length > 0 ? (
        <div role="alert" className="space-y-1 border-l-2 border-error px-3 py-2 text-xs text-error">
          {loadErrors.map((error, index) => <p key={`${index}:${error}`}>{error}</p>)}
        </div>
      ) : null}

      <QueueStatsGrid stats={stats} />

      <ActiveWorkflowsPanel
        workflows={workflows}
        loading={loading}
        cancellingTaskIds={cancellingWorkflowIds}
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
