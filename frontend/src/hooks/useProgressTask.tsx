import React, {
  createContext,
  useCallback,
  useContext,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { extractErrorMessage } from "../api";
import { getTaskProgressEvents } from "../api/task";
import { ProgressDialog } from "../components/feedback/ProgressDialog";
import type {
  ProgressDialogStatus,
  ProgressTimelineItem,
} from "../components/feedback/ProgressDialog";
import type { ProgressMeta } from "../api/progress";
import { isCancelledOperation } from "../utils/cancellation";
import { useI18n } from "../i18n";

interface ProgressTaskState {
  open: boolean;
  title: string;
  status: ProgressDialogStatus;
  current: number;
  total: number;
  message: string;
  meta: ProgressMeta | null;
  streamText: string | null;
  streaming: boolean;
  error: string | null;
  events: ProgressTimelineItem[];
  cancelLabel?: string;
  runningHint?: React.ReactNode;
  formatProgress?: (current: number, total: number, percent: number) => React.ReactNode;
}

export interface ProgressTaskOptions {
  title: string;
  startMessage: string;
  successMessage: string;
  errorMessage: string;
  total?: number;
  /** Host workflow 或 generation task 的取消身份。 */
  cancelTaskId?: string | null;
  cancelLabel?: string;
  runningHint?: React.ReactNode;
  formatProgress?: (current: number, total: number, percent: number) => React.ReactNode;
  /** generation 取消返回 void；Host workflow 返回是否仍可取消。 */
  cancelTask?: (taskId: string) => Promise<boolean | void>;
  /** Host workflow 使用轮询；generation task 使用事件订阅。 */
  pollTaskProgress?: boolean;
  /** 是否显示固定速率的模型流预览。 */
  streaming?: boolean;
}

export interface ProgressTaskControl {
  setProgress: (
    message: string,
    current?: number,
    total?: number,
    meta?: ProgressMeta | null,
  ) => void;
  setStreamText: (text: string) => void;
}

export type RunProgressTask = <T>(
  options: ProgressTaskOptions,
  task: (control: ProgressTaskControl) => Promise<T>,
) => Promise<T>;

interface ProgressTaskApi {
  runProgressTask: RunProgressTask;
}

interface ProgressTaskController extends ProgressTaskApi {
  progressTask: ProgressTaskState;
  closeProgressTask: () => void;
  cancelProgressTask: () => Promise<void>;
  activeTaskId: string | null;
  cancelling: boolean;
}

const MAX_PROGRESS_EVENTS = 160;
const WORKFLOW_PROGRESS_POLL_MS = 500;

export async function runTaskWithPolling<T, TUpdate>(
  task: Promise<T>,
  poll: () => Promise<TUpdate>,
  applyUpdate: (update: TUpdate) => void,
): Promise<T> {
  let finished = false;
  let polling = false;

  const pollOnce = async () => {
    if (finished || polling) return;
    polling = true;
    try {
      const update = await poll();
      if (!finished) applyUpdate(update);
    } catch {
      // apiInvoke 已记录错误；辅助进度查询不能覆盖真实业务结果。
    } finally {
      polling = false;
    }
  };

  const timer = globalThis.setInterval(() => void pollOnce(), WORKFLOW_PROGRESS_POLL_MS);
  try {
    return await task;
  } finally {
    finished = true;
    globalThis.clearInterval(timer);
  }
}

export async function runTaskWithProgressEvents<T>(
  taskId: string,
  task: Promise<T>,
  setProgress: ProgressTaskControl["setProgress"],
  setStreamText?: ProgressTaskControl["setStreamText"],
): Promise<T> {
  const seenEventIds = new Set<string>();
  return runTaskWithPolling(
    task,
    () => getTaskProgressEvents(taskId, 200),
    (response) => {
      for (const event of response.events) {
        if (seenEventIds.has(event.id)) continue;
        seenEventIds.add(event.id);
        setProgress(event.message, event.current, event.total, event.meta ?? null);
      }
      // 活体信号是快照而非事件：不去重，每次轮询取最新尾巴覆盖。
      if (response.stream_text) setStreamText?.(response.stream_text);
    },
  );
}

const createProgressTaskState = (): ProgressTaskState => ({
  open: false,
  title: "",
  status: "idle",
  current: 0,
  total: 1,
  message: "",
  meta: null,
  streamText: null,
  streaming: false,
  error: null,
  events: [],
  cancelLabel: undefined,
  runningHint: undefined,
  formatProgress: undefined,
});

const validTotal = (value?: number): number =>
  Number.isInteger(value) && value != null && value > 0 ? value : 1;

export const createWorkflowTaskId = (scope: string, projectId: number): string =>
  `${scope}-${projectId}-${globalThis.crypto.randomUUID()}`;

const appendProgressEvent = <T extends { events: ProgressTimelineItem[] }>(
  state: T,
  event: ProgressTimelineItem,
): T => ({
  ...state,
  events: [...state.events, event].slice(-MAX_PROGRESS_EVENTS),
});

const ProgressTaskContext = createContext<ProgressTaskApi | null>(null);

export function useProgressTask(): ProgressTaskApi {
  const ctx = useContext(ProgressTaskContext);
  if (!ctx) {
    throw new Error("useProgressTask 必须在 ProgressTaskProvider 内使用");
  }
  return ctx;
}

function useProgressTaskController(): ProgressTaskController {
  const { t } = useI18n();
  const [progressTask, setProgressTask] = useState<ProgressTaskState>(createProgressTaskState);
  const [activeTaskId, setActiveTaskId] = useState<string | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const cancelTaskRef = useRef<((taskId: string) => Promise<boolean | void>) | null>(null);
  const cancellingRef = useRef(false);
  const activeRunRef = useRef<string | null>(null);

  const closeProgressTask = useCallback(() => {
    activeRunRef.current = null;
    cancelTaskRef.current = null;
    cancellingRef.current = false;
    setActiveTaskId(null);
    setCancelling(false);
    setProgressTask(createProgressTaskState());
  }, []);

  const runProgressTask = useCallback<RunProgressTask>(async (options, task) => {
    const eventScope = `progress-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    activeRunRef.current = eventScope;
    const isCurrentRun = () => activeRunRef.current === eventScope;
    let taskCurrent = 0;
    let taskTotal = validTotal(options.total);

    const createEvent = (
      status: ProgressDialogStatus,
      current: number,
      total: number,
      message: string,
      meta: ProgressMeta | null = null,
    ): ProgressTimelineItem => ({
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      taskId: eventScope,
      status,
      current,
      total,
      percent: total > 0 ? Math.min(100, Math.round((current / total) * 100)) : 0,
      message,
      meta,
      timestamp: Date.now(),
    });

    setProgressTask({
      ...createProgressTaskState(),
      open: true,
      title: options.title,
      status: "running",
      current: 0,
      total: taskTotal,
      message: options.startMessage,
      cancelLabel: options.cancelLabel,
      runningHint: options.runningHint,
      formatProgress: options.formatProgress,
      streaming: options.streaming === true,
      // 保留累计流文本，由 ProgressDialog 负责逐字显示和尾部裁剪。
      streamText: null,
      events: [createEvent("running", 0, taskTotal, options.startMessage, null)],
    });

    const setStreamText = (text: string) => {
      if (!isCurrentRun() || !text.trim()) return;
      setProgressTask((prev) => prev.streamText === text
        ? prev
        : { ...prev, streamText: text });
    };

    const control: ProgressTaskControl = {
      setProgress: (message, current, nextTotal, meta) => {
        if (!isCurrentRun() || cancellingRef.current) return;
        const timelineEvent = createEvent(
          "running",
          current ?? taskCurrent,
          nextTotal == null ? taskTotal : validTotal(nextTotal),
          message,
          meta ?? null,
        );
        taskCurrent = timelineEvent.current;
        taskTotal = timelineEvent.total;
        setProgressTask((prev) =>
          appendProgressEvent(
            {
              ...prev,
              message,
              meta: timelineEvent.meta ?? null,
              current: timelineEvent.current,
              total: timelineEvent.total,
            },
            timelineEvent,
          ),
        );
      },
      setStreamText,
    };

    const cancellableTaskId = options.cancelTaskId?.trim() || null;
    cancelTaskRef.current = options.cancelTask ?? null;
    cancellingRef.current = false;
    setActiveTaskId(cancellableTaskId);
    setCancelling(false);

    try {
      const taskPromise = task(control);
      // Host workflow 轮询持久化进度；generation task 由事件订阅供给进度。
      const shouldPollTaskProgress =
        cancellableTaskId != null && options.pollTaskProgress === true;
      const result = shouldPollTaskProgress
        ? await runTaskWithProgressEvents(
            cancellableTaskId,
            taskPromise,
            control.setProgress,
            control.setStreamText,
          )
        : await taskPromise;
      if (!isCurrentRun()) return result;
      const timelineEvent = createEvent("success", taskTotal, taskTotal, options.successMessage);
      taskCurrent = timelineEvent.current;
      taskTotal = timelineEvent.total;
      setProgressTask((prev) =>
        appendProgressEvent(
          {
            ...prev,
            status: "success",
            current: timelineEvent.current,
            total: timelineEvent.total,
            message: options.successMessage,
            meta: timelineEvent.meta ?? null,
            error: null,
          },
          timelineEvent,
        ),
      );
      return result;
    } catch (err) {
      if (!isCurrentRun()) throw err;
      const errorMessage = extractErrorMessage(err, options.errorMessage);
      const cancelled = isCancelledOperation(errorMessage);
      const timelineEvent = createEvent(
        cancelled ? "cancelled" : "error",
        taskCurrent,
        taskTotal,
        errorMessage,
      );
      taskCurrent = timelineEvent.current;
      taskTotal = timelineEvent.total;
      setProgressTask((prev) =>
        appendProgressEvent(
          {
            ...prev,
            status: cancelled ? "cancelled" : "error",
            current: timelineEvent.current,
            total: timelineEvent.total,
            message: cancelled ? errorMessage : options.errorMessage,
            meta: timelineEvent.meta ?? null,
            error: cancelled ? null : errorMessage,
          },
          timelineEvent,
        ),
      );
      throw err;
    } finally {
      if (isCurrentRun()) {
        activeRunRef.current = null;
        cancelTaskRef.current = null;
        cancellingRef.current = false;
        setActiveTaskId(null);
        setCancelling(false);
      }
    }
  }, []);

  const cancelProgressTask = useCallback(async () => {
    if (!activeTaskId || cancellingRef.current) return;
    const requestedRun = activeRunRef.current;
    cancellingRef.current = true;
    setCancelling(true);
    setProgressTask((prev) => ({
      ...prev,
      message: t("workflow.cancelRequested"),
      meta: null,
      error: null,
    }));
    try {
      const cancelTask = cancelTaskRef.current;
      if (!cancelTask) {
        throw new Error(t("workflow.cancelTaskMissing"));
      }
      const cancellationRequested = await cancelTask(activeTaskId);
      if (cancellationRequested === false) {
        if (activeRunRef.current !== requestedRun) return;
        setProgressTask((prev) => ({
          ...prev,
          message: t("workflow.alreadyFinished"),
          meta: null,
          error: null,
        }));
        setActiveTaskId(null);
        cancellingRef.current = false;
        setCancelling(false);
      }
    } catch (err) {
      if (activeRunRef.current !== requestedRun) return;
      const errorMessage = extractErrorMessage(err, t("workflow.stopFailed"));
      setProgressTask((prev) => ({
        ...prev,
        message: errorMessage,
        meta: null,
        error: errorMessage,
      }));
      cancellingRef.current = false;
      setCancelling(false);
    }
  }, [activeTaskId, t]);

  return {
    progressTask,
    closeProgressTask,
    runProgressTask,
    cancelProgressTask,
    activeTaskId,
    cancelling,
  };
}

export const ProgressTaskProvider: React.FC<{
  children: React.ReactNode;
  scopeKey?: string;
}> = ({ children, scopeKey }) => {
  const controller = useProgressTaskController();

  useLayoutEffect(() => {
    if (scopeKey != null) controller.closeProgressTask();
  }, [controller.closeProgressTask, scopeKey]);

  return (
    <ProgressTaskContext.Provider value={{ runProgressTask: controller.runProgressTask }}>
      {children}
      <ProgressDialog
        open={controller.progressTask.open}
        title={controller.progressTask.title}
        status={controller.progressTask.status}
        current={controller.progressTask.current}
        total={controller.progressTask.total}
        message={controller.progressTask.message}
        meta={controller.progressTask.meta}
        streamText={controller.progressTask.streamText}
        streaming={controller.progressTask.streaming}
        error={controller.progressTask.error}
        events={controller.progressTask.events}
        cancelLabel={controller.progressTask.cancelLabel}
        runningHint={controller.progressTask.runningHint}
        formatProgress={controller.progressTask.formatProgress}
        onCancel={
          controller.activeTaskId ? () => void controller.cancelProgressTask() : undefined
        }
        cancelling={controller.cancelling}
        onClose={controller.closeProgressTask}
      />
    </ProgressTaskContext.Provider>
  );
};
