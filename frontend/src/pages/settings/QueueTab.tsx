import React, { useCallback, useEffect, useRef, useState } from "react";
import { Activity, RefreshCw } from "lucide-react";
import { extractErrorMessage } from "../../api";
import {
  cancelTask,
  getQueueStats,
  getTaskHistory,
  type QueueStats,
  type TaskInfo,
} from "../../api/task";
import {
  cancelWorkflow,
  getWorkflowRunDetail,
  listActiveWorkflows,
  listWorkflowRuns,
  resumeWorkflowRun,
  type WorkflowRun,
  type WorkflowRunDetail,
  type WorkflowState,
} from "../../api/streaming";
import { useToast } from "../../components/feedback/Toast";
import { BookIconButton } from "../../components/ui/BookButton";
import { useAuthStore } from "../../store/auth";
import { ActiveWorkflowsPanel } from "./queue/ActiveWorkflowsPanel";
import { QueueInfoCard } from "./queue/QueueInfoCard";
import { QueueStatsGrid } from "./queue/QueueStatsGrid";
import { TaskHistoryPanel } from "./queue/TaskHistoryPanel";
import { WorkflowRunsPanel } from "./queue/WorkflowRunsPanel";

/**
 * 任务队列 Tab
 *
 * 页面负责轮询和取消命令；指标、任务历史、活跃工作流和手动提交器拆到
 * settings/queue 下，避免队列状态展示与高级提交表单混在一个文件。
 */
export const QueueTab: React.FC = () => {
  const { addToast } = useToast();
  const user = useAuthStore((state) => state.user);
  const canReadQueueDiagnostics = user?.role === "admin";
  const [stats, setStats] = useState<QueueStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [tasks, setTasks] = useState<TaskInfo[]>([]);
  const [tasksLoading, setTasksLoading] = useState(false);
  const [cancellingTaskIds, setCancellingTaskIds] = useState<Set<string>>(() => new Set());
  const [workflows, setWorkflows] = useState<WorkflowState[]>([]);
  const [workflowRuns, setWorkflowRuns] = useState<WorkflowRun[]>([]);
  const [workflowRunDetails, setWorkflowRunDetails] = useState<Map<number, WorkflowRunDetail>>(
    () => new Map(),
  );
  const [expandedWorkflowRunIds, setExpandedWorkflowRunIds] = useState<Set<number>>(
    () => new Set(),
  );
  const [loadingWorkflowRunDetailIds, setLoadingWorkflowRunDetailIds] = useState<Set<number>>(
    () => new Set(),
  );
  const [resumingWorkflowRunIds, setResumingWorkflowRunIds] = useState<Set<number>>(() => new Set());
  const [cancellingWorkflowIds, setCancellingWorkflowIds] = useState<Set<string>>(() => new Set());
  const mountedRef = useRef(true);
  const tasksRef = useRef(tasks);
  const workflowsRef = useRef(workflows);
  const cancellingTaskIdsRef = useRef(cancellingTaskIds);
  const cancellingWorkflowIdsRef = useRef(cancellingWorkflowIds);
  const resumingWorkflowRunIdsRef = useRef(resumingWorkflowRunIds);
  const workflowRunDetailsRef = useRef(workflowRunDetails);
  const expandedWorkflowRunIdsRef = useRef(expandedWorkflowRunIds);
  const loadingWorkflowRunDetailIdsRef = useRef(loadingWorkflowRunDetailIds);
  const refreshSeqRef = useRef(0);
  const refreshBusyRef = useRef(false);

  tasksRef.current = tasks;
  workflowsRef.current = workflows;
  cancellingTaskIdsRef.current = cancellingTaskIds;
  cancellingWorkflowIdsRef.current = cancellingWorkflowIds;
  resumingWorkflowRunIdsRef.current = resumingWorkflowRunIds;
  workflowRunDetailsRef.current = workflowRunDetails;
  expandedWorkflowRunIdsRef.current = expandedWorkflowRunIds;
  loadingWorkflowRunDetailIdsRef.current = loadingWorkflowRunDetailIds;

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

  const refresh = useCallback(async (options?: { force?: boolean }) => {
    if (!canReadQueueDiagnostics) {
      refreshBusyRef.current = false;
      setStats(null);
      setTasks([]);
      setWorkflows([]);
      setWorkflowRuns([]);
      setLoading(false);
      setTasksLoading(false);
      return;
    }
    if (refreshBusyRef.current && !options?.force) return;
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    refreshBusyRef.current = true;
    setLoading(true);
    setTasksLoading(true);
    try {
      const [queueStats, taskHistory, activeWorkflows, persistedWorkflowRuns] = await Promise.all([
        getQueueStats(),
        getTaskHistory(null, 0, 30),
        listActiveWorkflows(),
        listWorkflowRuns({ limit: 40 }),
      ]);
      if (!isCurrentRefresh(requestSeq)) return;
      setStats(queueStats);
      setTasks(taskHistory);
      setWorkflows(activeWorkflows);
      setWorkflowRuns(persistedWorkflowRuns);
    } catch (err) {
      if (!isCurrentRefresh(requestSeq)) return;
      addToast(extractErrorMessage(err, "加载任务队列失败"), "error");
    } finally {
      if (refreshSeqRef.current === requestSeq) {
        refreshBusyRef.current = false;
      }
      if (isCurrentRefresh(requestSeq)) {
        setLoading(false);
        setTasksLoading(false);
      }
    }
  }, [addToast, canReadQueueDiagnostics, isCurrentRefresh]);

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

  const setWorkflowRunDetailLoading = (runId: number, loading: boolean) => {
    const next = new Set(loadingWorkflowRunDetailIdsRef.current);
    if (loading) next.add(runId);
    else next.delete(runId);
    loadingWorkflowRunDetailIdsRef.current = next;
    setLoadingWorkflowRunDetailIds(next);
  };

  const handleToggleWorkflowRunDetail = async (run: WorkflowRun) => {
    const expanded = new Set(expandedWorkflowRunIdsRef.current);
    if (expanded.has(run.id)) {
      expanded.delete(run.id);
      expandedWorkflowRunIdsRef.current = expanded;
      setExpandedWorkflowRunIds(expanded);
      return;
    }
    expanded.add(run.id);
    expandedWorkflowRunIdsRef.current = expanded;
    setExpandedWorkflowRunIds(expanded);
    if (workflowRunDetailsRef.current.has(run.id) || loadingWorkflowRunDetailIdsRef.current.has(run.id)) {
      return;
    }
    setWorkflowRunDetailLoading(run.id, true);
    try {
      const detail = await getWorkflowRunDetail(run.id);
      if (!mountedRef.current) return;
      const next = new Map(workflowRunDetailsRef.current);
      next.set(run.id, detail);
      workflowRunDetailsRef.current = next;
      setWorkflowRunDetails(next);
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(extractErrorMessage(err, "加载工作流详情失败"), "error");
    } finally {
      if (mountedRef.current) setWorkflowRunDetailLoading(run.id, false);
    }
  };

  const handleCancel = async (taskId: string) => {
    if (cancellingTaskIdsRef.current.has(taskId)) return;
    if (!tasksRef.current.some((task) => task.id === taskId)) {
      addToast("任务已不在最近任务列表中", "info");
      return;
    }
    markTaskCancelling(taskId);
    try {
      await cancelTask(taskId);
      if (!mountedRef.current) return;
      addToast("任务已取消", "success");
      await refresh({ force: true });
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(extractErrorMessage(err, "取消任务失败"), "error");
    } finally {
      if (mountedRef.current) clearTaskCancelling(taskId);
    }
  };

  const handleCancelWorkflow = async (taskId: string) => {
    if (cancellingWorkflowIdsRef.current.has(taskId)) return;
    if (!workflowsRef.current.some((workflow) => workflow.task_id === taskId)) {
      addToast("工作流已不在活跃列表中", "info");
      return;
    }
    markWorkflowCancelling(taskId);
    try {
      await cancelWorkflow(taskId);
      if (!mountedRef.current) return;
      addToast("工作流已取消", "success");
      await refresh({ force: true });
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(extractErrorMessage(err, "取消工作流失败"), "error");
    } finally {
      if (mountedRef.current) clearWorkflowCancelling(taskId);
    }
  };

  const handleResumeWorkflowRun = async (run: WorkflowRun) => {
    if (resumingWorkflowRunIdsRef.current.has(run.id)) return;
    markWorkflowRunResuming(run.id);
    try {
      await resumeWorkflowRun(run.id);
      if (!mountedRef.current) return;
      addToast("已恢复工作流", "success");
      await refresh({ force: true });
    } catch (err) {
      if (!mountedRef.current) return;
      addToast(extractErrorMessage(err, "恢复工作流失败"), "error");
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
        请求队列诊断仅管理员可查看。
      </div>
    );
  }

  return (
    <div className="custom-scrollbar flex h-auto flex-col gap-3 overflow-visible md:h-full md:overflow-y-auto md:pr-2">
      <div className="flex shrink-0 items-center justify-end gap-2 text-[11px] text-book-text-muted">
        <span className="inline-flex items-center gap-1.5">
          <Activity size={12} className="text-book-primary" />
          每 5 秒自动刷新
        </span>
        <BookIconButton
          label="立即刷新"
          onClick={() => void refresh()}
          variant="secondary"
          size="sm"
          disabled={loading}
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>

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
        expandedRunIds={expandedWorkflowRunIds}
        loadingDetailRunIds={loadingWorkflowRunDetailIds}
        details={workflowRunDetails}
        onResume={(run) => void handleResumeWorkflowRun(run)}
        onToggleDetail={(run) => void handleToggleWorkflowRunDetail(run)}
      />

      <QueueInfoCard />
    </div>
  );
};
