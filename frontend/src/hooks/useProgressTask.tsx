import React, {
  createContext,
  useCallback,
  useContext,
  useRef,
  useState,
} from "react";
import { extractErrorMessage } from "../api";
import { cancelWorkflow } from "../api/streaming";
import { getTaskProgressEvents } from "../api/task";
import { ProgressDialog } from "../components/feedback/ProgressDialog";
import type {
  ProgressDialogStatus,
  ProgressTimelineItem,
} from "../components/feedback/ProgressDialog";
import type { ProgressMeta } from "../api/progress";
import { isCancelledOperation } from "../utils/cancellation";
import { streamLineFromBuffer } from "../utils/progressDisplay";

interface ProgressTaskState {
  open: boolean;
  title: string;
  status: ProgressDialogStatus;
  current: number;
  total: number;
  message: string;
  meta: ProgressMeta | null;
  streamText: string | null;
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
  /**
   * 仅作取消身份；不得再用于订阅旧 workflow 事件。
   * 新 generation 路径请用 runGenerationProgressTaskThen。
   */
  cancelTaskId?: string | null;
  showStream?: boolean;
  cancelLabel?: string;
  runningHint?: React.ReactNode;
  formatProgress?: (current: number, total: number, percent: number) => React.ReactNode;
  cancelTask?: (taskId: string) => Promise<void>;
  /** 自定义取消函数仍对应 Host task 时显式开启；默认 Host 任务会自动读取后端进度。 */
  pollTaskProgress?: boolean;
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
const HOST_PROGRESS_POLL_MS = 500;

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

  const timer = globalThis.setInterval(() => void pollOnce(), HOST_PROGRESS_POLL_MS);
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
): Promise<T> {
  const seenEventIds = new Set<string>();
  return runTaskWithPolling(
    task,
    () => getTaskProgressEvents(taskId, 200),
    (events) => {
      for (const event of events) {
        if (seenEventIds.has(event.id)) continue;
        seenEventIds.add(event.id);
        setProgress(event.message, event.current, event.total, event.meta ?? null);
      }
    },
  );
}

const createProgressTaskState = (): ProgressTaskState => ({
  open: false,
  title: "生成进度",
  status: "idle",
  current: 0,
  total: 1,
  message: "",
  meta: null,
  streamText: null,
  error: null,
  events: [],
  cancelLabel: undefined,
  runningHint: undefined,
  formatProgress: undefined,
});

const validTotal = (value?: number): number =>
  Number.isInteger(value) && value != null && value > 0 ? value : 1;

/**
 * 仅分配 Host 取消/后端 progress 注册身份，不订阅任何旧事件通道。
 * 不得用于前端自建规范 generation task id。
 */
export const allocateHostTaskId = (scope: string, projectId: number): string => {
  const random =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
  return `${scope}-${projectId}-${random}`;
};

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
  const [progressTask, setProgressTask] = useState<ProgressTaskState>(createProgressTaskState);
  const [activeTaskId, setActiveTaskId] = useState<string | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const cancelTaskRef = useRef<((taskId: string) => Promise<void>) | null>(null);
  const cancellingRef = useRef(false);

  const closeProgressTask = useCallback(() => {
    setProgressTask(createProgressTaskState());
  }, []);

  const runProgressTask = useCallback<RunProgressTask>(async (options, task) => {
    const showStream = options.showStream === true;
    const eventScope = `progress-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
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
      streamText: showStream ? "" : null,
      events: [createEvent("running", 0, taskTotal, options.startMessage, null)],
    });

    const setStreamText = (text: string) => {
      if (!showStream) return;
      const line = streamLineFromBuffer(text);
      setProgressTask((prev) => ({
        ...prev,
        streamText: line,
      }));
    };

    const control: ProgressTaskControl = {
      setProgress: (message, current, nextTotal, meta) => {
        if (cancellingRef.current) return;
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
      const shouldPollTaskProgress =
        cancellableTaskId != null &&
        (options.pollTaskProgress ?? options.cancelTask == null);
      const result = shouldPollTaskProgress
        ? await runTaskWithProgressEvents(cancellableTaskId, taskPromise, control.setProgress)
        : await taskPromise;
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
      cancelTaskRef.current = null;
      cancellingRef.current = false;
      setActiveTaskId(null);
      setCancelling(false);
    }
  }, []);

  const cancelProgressTask = useCallback(async () => {
    if (!activeTaskId || cancellingRef.current) return;
    cancellingRef.current = true;
    setCancelling(true);
    setProgressTask((prev) => ({
      ...prev,
      message: "已发送停止任务请求，正在等待后端确认…",
      meta: null,
      error: null,
    }));
    try {
      const cancelTask = cancelTaskRef.current ?? cancelWorkflow;
      await cancelTask(activeTaskId);
    } catch (err) {
      const errorMessage = extractErrorMessage(err, "停止任务失败");
      setProgressTask((prev) => ({
        ...prev,
        message: errorMessage,
        meta: null,
        error: errorMessage,
      }));
      cancellingRef.current = false;
      setCancelling(false);
    }
  }, [activeTaskId]);

  return {
    progressTask,
    closeProgressTask,
    runProgressTask,
    cancelProgressTask,
    activeTaskId,
    cancelling,
  };
}

export const ProgressTaskProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const controller = useProgressTaskController();

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
