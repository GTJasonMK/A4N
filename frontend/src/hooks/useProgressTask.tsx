import React, {
  createContext,
  useCallback,
  useContext,
  useRef,
  useState,
} from "react";
import { extractErrorMessage } from "../api";
import {
  listenAppEvent,
  type UnlistenFn,
} from "../api/runtime";
import { cancelWorkflow } from "../api/streaming";
import { ProgressDialog } from "../components/feedback/ProgressDialog";
import type {
  ProgressDialogStatus,
  ProgressTimelineItem,
} from "../components/feedback/ProgressDialog";
import type { ProgressMeta } from "../api/progress";
import { isCancelledOperation } from "../utils/cancellation";

interface ProgressTaskState {
  open: boolean;
  title: string;
  status: ProgressDialogStatus;
  current: number;
  total: number;
  message: string;
  meta: ProgressMeta | null;
  streamText: string | null;
  streamVersion: number;
  error: string | null;
  connectionWarning: string | null;
  events: ProgressTimelineItem[];
  cancelLabel?: string;
  runningHint?: React.ReactNode;
  formatProgress?: (current: number, total: number, percent: number) => React.ReactNode;
}

interface ProgressTaskOptions {
  title: string;
  startMessage: string;
  successMessage: string;
  errorMessage: string;
  total?: number;
  taskId?: string | null;
  showStream?: boolean;
  onTimelineEvent?: (event: ProgressTimelineItem) => void;
  onChunk?: (content: string, index: number) => void;
  cancelLabel?: string;
  runningHint?: React.ReactNode;
  formatProgress?: (current: number, total: number, percent: number) => React.ReactNode;
  cancelTask?: (taskId: string) => Promise<void>;
  eventSource?: "auto" | "tauri" | "server";
}

interface ProgressTaskControl {
  setProgress: (message: string, current?: number, total?: number) => void;
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

type WorkflowEvent =
  | {
      type: "Progress";
      data: {
        current: number;
        total: number;
        message: string;
        percent?: number | null;
        meta?: ProgressMeta | null;
      };
    }
  | { type: "Chunk"; data: { content: string; index: number } }
  | { type: "Complete"; data: { summary?: string | null } }
  | { type: "Error"; data: { code?: string; message: string } };

const MAX_PROGRESS_EVENTS = 160;
const MAX_STREAM_BUFFER_CHARS = 800;
const MAX_STREAM_LINE_CHARS = 180;
const STREAM_FLUSH_INTERVAL_MS = 120;

const createProgressTaskState = (): ProgressTaskState => ({
  open: false,
  title: "生成进度",
  status: "idle",
  current: 0,
  total: 1,
  message: "",
  meta: null,
  streamText: null,
  streamVersion: 0,
  error: null,
  connectionWarning: null,
  events: [],
  cancelLabel: undefined,
  runningHint: undefined,
  formatProgress: undefined,
});

const validTotal = (value?: number): number =>
  Number.isInteger(value) && value != null && value > 0 ? value : 1;

export const createProgressTaskId = (scope: string, projectId: number): string =>
  `${scope}-${projectId}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

const terminalEventError = (
  status: ProgressDialogStatus | null,
  message: string | null,
): Error | null => {
  if (status === "cancelled") return new Error(message || "任务已停止");
  if (status === "error") return new Error(message || "任务失败");
  return null;
};

const appendProgressEvent = <T extends { events: ProgressTimelineItem[] }>(
  state: T,
  event: ProgressTimelineItem,
): T => ({
  ...state,
  events: [...state.events, event].slice(-MAX_PROGRESS_EVENTS),
});

const trimChars = (text: string, maxChars: number): string => {
  const chars = Array.from(text);
  if (chars.length <= maxChars) return text;
  return `…${chars.slice(chars.length - maxChars).join("")}`;
};

const streamLineFromBuffer = (buffer: string): string => {
  const compact = buffer.replace(/\s+/g, " ").trim();
  return compact ? trimChars(compact, MAX_STREAM_LINE_CHARS) : "";
};

const ProgressTaskContext = createContext<ProgressTaskApi | null>(null);

function useProgressTaskController(): ProgressTaskController {
  const [progressTask, setProgressTask] = useState<ProgressTaskState>(
    createProgressTaskState,
  );
  const [activeTaskId, setActiveTaskId] = useState<string | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const activeTaskIdRef = useRef<string | null>(null);
  const cancelTaskRef = useRef<((taskId: string) => Promise<void>) | null>(null);
  const cancellingRef = useRef(false);

  const closeProgressTask = useCallback(() => {
    setProgressTask((prev) => ({ ...prev, open: false }));
  }, []);

  const runProgressTask: RunProgressTask = useCallback(async (options, task) => {
    if (activeTaskIdRef.current) {
      throw new Error("已有任务正在运行，请等待当前任务完成后再开始新任务");
    }
    const total = validTotal(options.total);
    let eventSequence = 0;
    const eventScope = options.taskId?.trim() || `local-${Date.now()}`;
    const createEvent = (
      status: ProgressDialogStatus,
      current: number,
      nextTotal: number,
      message: string,
      meta?: ProgressMeta | null,
    ): ProgressTimelineItem => {
      const boundedTotal = validTotal(nextTotal);
      return {
        id: `${eventScope}-${Date.now()}-${eventSequence++}`,
        taskId: eventScope,
        status,
        current: Math.max(0, Math.min(current, boundedTotal)),
        total: boundedTotal,
        message,
        meta: meta ?? null,
        timestamp: Date.now(),
      };
    };
    const emitTimelineEvent = (event: ProgressTimelineItem) => {
      options.onTimelineEvent?.(event);
      return event;
    };
    const initialEvent = createEvent("running", 0, total, options.startMessage);
    emitTimelineEvent(initialEvent);
    let taskCurrent = initialEvent.current;
    let taskTotal = initialEvent.total;
    setProgressTask({
      open: true,
      title: options.title,
      status: "running",
      current: 0,
      total,
      message: options.startMessage,
      meta: initialEvent.meta ?? null,
      streamText: null,
      streamVersion: 0,
      error: null,
      connectionWarning: null,
      events: [initialEvent],
      cancelLabel: options.cancelLabel,
      runningHint: options.runningHint,
      formatProgress: options.formatProgress,
    });

    let terminalEventStatus: ProgressDialogStatus | null = null;
    let terminalEventMessage: string | null = null;
    let streamBuffer = "";
    let lastStreamLine = "";
    let streamVersion = 0;
    let streamFlushTimer: ReturnType<typeof window.setTimeout> | null = null;
    const showStream = options.showStream ?? true;

    const flushStreamBuffer = () => {
      streamFlushTimer = null;
      const nextLine = streamLineFromBuffer(streamBuffer);
      if (!nextLine || nextLine === lastStreamLine) return;
      lastStreamLine = nextLine;
      streamVersion += 1;
      setProgressTask((prev) => ({
        ...prev,
        streamText: nextLine,
        streamVersion,
      }));
    };

    const scheduleStreamFlush = () => {
      if (streamFlushTimer != null) return;
      streamFlushTimer = window.setTimeout(flushStreamBuffer, STREAM_FLUSH_INTERVAL_MS);
    };

    const appendStreamChunk = (content: string) => {
      if (!content) return;
      streamBuffer = trimChars(`${streamBuffer}${content}`, MAX_STREAM_BUFFER_CHARS);
      scheduleStreamFlush();
    };

    const control: ProgressTaskControl = {
      setProgress: (message, current, nextTotal) => {
        if (terminalEventStatus) return;
        const timelineEvent = emitTimelineEvent(
          createEvent(
            "running",
            current ?? taskCurrent,
            nextTotal == null ? taskTotal : validTotal(nextTotal),
            message,
            null,
          ),
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
    };

    let unlisten: UnlistenFn | null = null;
    const cancellableTaskId = options.taskId?.trim() || null;
    activeTaskIdRef.current = cancellableTaskId || eventScope;
    cancelTaskRef.current = options.cancelTask ?? null;
    cancellingRef.current = false;
    setActiveTaskId(cancellableTaskId);
    setCancelling(false);
    if (options.taskId) {
      unlisten = await listenAppEvent<WorkflowEvent>(`workflow:${options.taskId}`, (event) => {
        const payload = event.payload;
        if (payload.type === "Progress") {
          if (terminalEventStatus) return;
          const nextTotal = validTotal(payload.data.total);
          const nextCurrent =
            nextTotal === taskTotal
              ? Math.max(payload.data.current, taskCurrent)
              : payload.data.current;
          const timelineEvent = emitTimelineEvent(
            createEvent(
              "running",
              nextCurrent,
              nextTotal,
              payload.data.message,
              payload.data.meta ?? null,
            ),
          );
          taskCurrent = timelineEvent.current;
          taskTotal = timelineEvent.total;
          setProgressTask((prev) =>
            appendProgressEvent(
              {
                ...prev,
                status: "running",
                current: timelineEvent.current,
                total: timelineEvent.total,
                message: payload.data.message,
                meta: timelineEvent.meta ?? null,
                error: null,
                connectionWarning: null,
              },
              timelineEvent,
            ),
          );
          return;
        }
        if (payload.type === "Chunk") {
          options.onChunk?.(payload.data.content, payload.data.index);
          if (showStream) appendStreamChunk(payload.data.content);
          return;
        }
        if (payload.type === "Complete") {
          if (terminalEventStatus) return;
          terminalEventStatus = "success";
          terminalEventMessage = payload.data.summary ?? options.successMessage;
          const timelineEvent = emitTimelineEvent(
            createEvent(
              "success",
              taskTotal,
              taskTotal,
              terminalEventMessage ?? options.successMessage,
              null,
            ),
          );
          taskCurrent = timelineEvent.current;
          taskTotal = timelineEvent.total;
          setProgressTask((prev) =>
            appendProgressEvent(
              {
                ...prev,
                status: "success",
                current: timelineEvent.current,
                total: timelineEvent.total,
                message: terminalEventMessage ?? options.successMessage,
                meta: timelineEvent.meta ?? null,
                error: null,
              },
              timelineEvent,
            ),
          );
          return;
        }
        if (payload.type === "Error") {
          if (terminalEventStatus) return;
          const cancelled =
            payload.data.code === "cancelled" || isCancelledOperation(payload.data.message);
          terminalEventStatus = cancelled ? "cancelled" : "error";
          terminalEventMessage = payload.data.message;
          const timelineEvent = emitTimelineEvent(
            createEvent(
              terminalEventStatus ?? "error",
              taskCurrent,
              taskTotal,
              payload.data.message,
              null,
            ),
          );
          taskCurrent = timelineEvent.current;
          taskTotal = timelineEvent.total;
          setProgressTask((prev) =>
            appendProgressEvent(
              {
                ...prev,
                status: terminalEventStatus ?? "error",
                current: timelineEvent.current,
                total: timelineEvent.total,
                message: cancelled ? payload.data.message : options.errorMessage,
                meta: timelineEvent.meta ?? null,
                error: cancelled ? null : terminalEventMessage,
              },
              timelineEvent,
            ),
          );
        }
      }, {
        onError: () => {
          if (terminalEventStatus) return;
          setProgressTask((prev) => ({
            ...prev,
            connectionWarning: "进度连接已断开，任务可能仍在后台运行。可在任务队列中查看最新状态。",
          }));
        },
        source: options.eventSource ?? "auto",
      });
    }

    try {
      const result = await task(control);
      if (terminalEventStatus === "success") return result;
      const terminalError = terminalEventError(terminalEventStatus, terminalEventMessage);
      if (terminalError) throw terminalError;
      const timelineEvent = emitTimelineEvent(
        createEvent("success", taskTotal, taskTotal, options.successMessage),
      );
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
      if (terminalEventStatus) throw err;
      const errorMessage = extractErrorMessage(err, options.errorMessage);
      const cancelled = isCancelledOperation(errorMessage);
      const timelineEvent = emitTimelineEvent(
        createEvent(cancelled ? "cancelled" : "error", taskCurrent, taskTotal, errorMessage),
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
      if (streamFlushTimer != null) {
        window.clearTimeout(streamFlushTimer);
        flushStreamBuffer();
      }
      unlisten?.();
      activeTaskIdRef.current = null;
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
        streamVersion={controller.progressTask.streamVersion}
        error={controller.progressTask.error}
        connectionWarning={controller.progressTask.connectionWarning}
        events={controller.progressTask.events}
        onClose={controller.closeProgressTask}
        onCancel={controller.activeTaskId ? controller.cancelProgressTask : undefined}
        cancelLabel={controller.progressTask.cancelLabel}
        cancelling={controller.cancelling}
        runningHint={controller.progressTask.runningHint}
        formatProgress={controller.progressTask.formatProgress}
      />
    </ProgressTaskContext.Provider>
  );
};

export function useProgressTask() {
  return useContext(ProgressTaskContext) ?? useProgressTaskController();
}
