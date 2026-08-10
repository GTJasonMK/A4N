import {
  apiInvoke,
  authenticatedWebFetch,
  extractErrorMessage,
} from ".";
import { requireCurrentUserId as currentUserId } from "./authStorage";
import {
  parseGenerationTaskEvent,
  parseGenerationTaskHandle,
  parseGenerationTaskHistoryPage,
  parseGenerationTaskSnapshot,
  parseGenerationTaskSummary,
  parseGenerationSubscriptionHandle,
  parseHostErrorResponse,
  type GenerationTaskEvent,
} from "./generationEvents";
import {
  type CompletedEvent,
  type CancelGenerationTaskInput,
  type GenerationStartRequestMeta,
  type GenerationTaskHandle,
  type GenerationTaskHistoryPage,
  type GenerationTaskHistoryQuery,
  type GenerationTaskSnapshot,
  type GenerationTaskSummary,
  type UnsubscribeGenerationTaskInput,
} from "./generated/generationContracts";
import {
  createRuntimeChannel,
  isTauriRuntime,
  runtimeInvoke,
  shouldUseHttpBackend,
} from "./runtime";
import { readSseStream } from "../utils/sseParser";
import { sha256Hex } from "../utils/contentHash";
import { utf8ByteLength } from "../utils/text";
import {
  generationTaskKey,
  getGenerationTaskViewState,
  useGenerationTasksStore,
} from "../store/generationTasks";

interface GenerationTransportSession {
  controller: AbortController;
  subscriptionId?: string;
  closed: boolean;
  backendFinished: boolean;
  close: (backendFinished?: boolean) => Promise<void>;
}

const sessions = new Map<string, GenerationTransportSession>();
const pendingAttach = new Map<string, Promise<void>>();
const pendingRecovery = new Set<string>();

export const generationClient = {
  async getSummary(taskId: string, attempt: number): Promise<GenerationTaskSummary> {
    if (shouldUseHttpBackend()) {
      const response = await authenticatedWebFetch(
        `/generation-tasks/${encodeURIComponent(taskId)}?attempt=${attempt}`,
      );
      return parseGenerationTaskSummary(await readResponseJson(response));
    }
    return parseGenerationTaskSummary(
      await runtimeInvoke<unknown>("get_generation_task_summary", {
        userId: currentUserId(),
        taskId,
        attempt,
      }),
    );
  },

  async getSnapshot(taskId: string, attempt: number): Promise<GenerationTaskSnapshot> {
    if (shouldUseHttpBackend()) {
      const response = await authenticatedWebFetch(
        `/generation-tasks/${encodeURIComponent(taskId)}/snapshot?attempt=${attempt}`,
      );
      return parseGenerationTaskSnapshot(await readResponseJson(response));
    }
    return parseGenerationTaskSnapshot(
      await runtimeInvoke<unknown>("get_generation_task_snapshot", {
        userId: currentUserId(),
        taskId,
        attempt,
      }),
    );
  },

  async listActive(projectId?: number | null): Promise<GenerationTaskSummary[]> {
    if (shouldUseHttpBackend()) {
      const query = projectId == null ? "" : `?project_id=${encodeURIComponent(String(projectId))}`;
      const response = await authenticatedWebFetch(`/generation-tasks/active${query}`);
      const value = await readResponseJson(response);
      if (!Array.isArray(value)) throw new Error("generation active response must be an array");
      return value.map(parseGenerationTaskSummary);
    }
    const value = await runtimeInvoke<unknown[]>("list_active_generation_tasks", {
      userId: currentUserId(),
      projectId: projectId ?? null,
    });
    return value.map(parseGenerationTaskSummary);
  },

  async listHistory(query: GenerationTaskHistoryQuery): Promise<GenerationTaskHistoryPage> {
    if (shouldUseHttpBackend()) {
      const params = new URLSearchParams({ limit: String(query.limit) });
      if (query.project_id != null) params.set("project_id", String(query.project_id));
      if (query.cursor) params.set("cursor", query.cursor);
      const response = await authenticatedWebFetch(`/generation-tasks/history?${params}`);
      return parseGenerationTaskHistoryPage(await readResponseJson(response));
    }
    return parseGenerationTaskHistoryPage(
      await runtimeInvoke<unknown>("list_generation_task_history", {
        userId: currentUserId(),
        input: {
          project_id: query.project_id ?? null,
          limit: query.limit,
          cursor: query.cursor ?? null,
        },
      }),
    );
  },

  async cancel(handle: GenerationTaskHandle): Promise<GenerationTaskHandle> {
    const input: CancelGenerationTaskInput = {
      task_id: handle.task_id,
      attempt: handle.attempt,
    };
    if (shouldUseHttpBackend()) {
      const response = await authenticatedWebFetch(
        `/generation-tasks/${encodeURIComponent(handle.task_id)}/cancel`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(input),
        },
      );
      return parseGenerationTaskHandle(await readResponseJson(response));
    }
    return parseGenerationTaskHandle(
      await apiInvoke<unknown>("cancel_generation_task", {
        userId: currentUserId(),
        input,
      }),
    );
  },

  async start<T extends GenerationTaskHandle>(
    start: (meta: GenerationStartRequestMeta) => Promise<T>,
    retryOfTaskId?: string | null,
  ): Promise<T> {
    const start_request_id = crypto.randomUUID();
    return start({
      start_request_id,
      retry_of_task_id: retryOfTaskId ?? null,
    });
  },

  async waitForTerminal(
    handle: GenerationTaskHandle,
    onUpdate?: (task: ReturnType<typeof getGenerationTaskViewState>) => void,
  ): Promise<NonNullable<ReturnType<typeof getGenerationTaskViewState>>> {
    const key = generationTaskKey(handle.task_id, handle.attempt);
    const attach = generationClient.attach(handle);
    try {
      return await new Promise((resolve, reject) => {
        let unsubscribe = () => {};
        let settled = false;
        const finish = (
          task: NonNullable<ReturnType<typeof getGenerationTaskViewState>>,
          error?: Error,
        ) => {
          if (settled) return;
          settled = true;
          unsubscribe();
          if (error) reject(error);
          else resolve(task);
        };
        const inspect = () => {
          const task = getGenerationTaskViewState(handle.task_id, handle.attempt);
          onUpdate?.(task);
          if (!task) return;
          if (task.terminalReceived) {
            if (task.taskStatus === "completed") {
              finish(task);
              return;
            }
            finish(
              task,
              new Error(
                task.taskStatus === "cancelled" || task.taskStatus === "interrupted"
                  ? "生成已停止"
                  : task.error?.message?.trim() || "生成失败",
              ),
            );
            return;
          }
          if (
            ["protocol_error", "consistency_error"].includes(task.clientState)
            || ["failed", "expired", "corrupt"].includes(task.recoveryState)
          ) {
            finish(
              task,
              new Error(
                task.clientState === "consistency_error"
                  ? "生成结果一致性校验失败，请从正式结果重新加载"
                  : "生成状态连接失败，请在任务队列查看最终结果",
              ),
            );
          }
        };
        unsubscribe = useGenerationTasksStore.subscribe((state, previous) => {
          if (state.tasks[key] !== previous.tasks[key]) inspect();
        });
        void attach.catch((error) => {
          const task = getGenerationTaskViewState(handle.task_id, handle.attempt);
          const attachError = error instanceof Error ? error : new Error(String(error));
          if (task) {
            finish(task, attachError);
          } else if (!settled) {
            settled = true;
            unsubscribe();
            reject(attachError);
          }
        });
        inspect();
      });
    } finally {
      await generationClient.detach(handle);
    }
  },

  async attach(handle: GenerationTaskHandle): Promise<void> {
    const key = generationTaskKey(handle.task_id, handle.attempt);
    useGenerationTasksStore.getState().retain(handle);
    if (sessions.has(key)) return;
    const existing = pendingAttach.get(key);
    if (existing) return rollbackOnAttachFailure(existing, key);
    const task = attachTransport(handle, undefined).finally(() => pendingAttach.delete(key));
    pendingAttach.set(key, task);
    return rollbackOnAttachFailure(task, key);
  },

  async detach(handle: GenerationTaskHandle): Promise<void> {
    const key = generationTaskKey(handle.task_id, handle.attempt);
    const references = useGenerationTasksStore.getState().release(key);
    if (references > 0) return;
    await closeTransport(key);
  },

  async recover(
    handle: GenerationTaskHandle,
    consistencyResultRef: GenerationTaskSummary["result_ref"] = null,
  ): Promise<void> {
    const key = generationTaskKey(handle.task_id, handle.attempt);
    if (pendingRecovery.has(key)) return;
    pendingRecovery.add(key);
    try {
      useGenerationTasksStore.getState().dispatch(key, {
        type: "recovery_state",
        state: "recovering",
      });
      const snapshot = await this.getSnapshot(handle.task_id, handle.attempt);
      if (!await snapshotTerminalIsConsistent(snapshot)) {
        const message = "generation completed hash 与 snapshot 正文不一致";
        const terminal = snapshot.terminal_event;
        markConsistencyFailure(
          handle,
          message,
          terminal?.event.kind === "completed"
            ? terminal.event.data.result_ref ?? consistencyResultRef
            : consistencyResultRef,
        );
        throw new Error(message);
      }
      useGenerationTasksStore.getState().dispatch(key, { type: "snapshot", snapshot });
      const state = getGenerationTaskViewState(handle.task_id, handle.attempt);
      if (state) {
        await closeTransport(key);
        if (!state.terminalReceived) {
          await attachTransport(handle, snapshot.covers_through_sequence);
        }
      }
    } catch (error) {
      if (getGenerationTaskViewState(handle.task_id, handle.attempt)?.clientState === "consistency_error") {
        await closeTransport(key);
        throw error;
      }
      let summary: GenerationTaskSummary;
      try {
        summary = await this.getSummary(handle.task_id, handle.attempt);
      } catch (summaryError) {
        await closeTransport(key);
        useGenerationTasksStore.getState().dispatch(key, {
          type: "recovery_state",
          state: "failed",
        });
        throw new Error(
          `${extractErrorMessage(error, "generation snapshot 恢复失败")}；${extractErrorMessage(summaryError, "summary 查询失败")}`,
        );
      }
      useGenerationTasksStore.getState().dispatch(key, { type: "summary", summary });
      await closeTransport(key);
      if (summary.snapshot_state === "available") {
        useGenerationTasksStore.getState().dispatch(key, {
          type: "recovery_state",
          state: "failed",
        });
        throw error;
      }
    } finally {
      pendingRecovery.delete(key);
    }
  },
};

async function attachTransport(
  handle: GenerationTaskHandle,
  afterSequence: number | undefined,
): Promise<void> {
  const key = generationTaskKey(handle.task_id, handle.attempt);
  if (sessions.has(key)) return;
  const controller = new AbortController();
  const session: GenerationTransportSession = {
    controller,
    closed: false,
    backendFinished: false,
    close: async (backendFinished = false) => {
      session.closed = true;
      session.backendFinished ||= backendFinished;
      controller.abort();
    },
  };
  sessions.set(key, session);
  try {
    if (shouldUseHttpBackend()) {
      await runWebTransport(handle, afterSequence, session);
      if (sessions.get(key) === session) sessions.delete(key);
    } else {
      await runTauriTransport(handle, afterSequence, session);
    }
  } catch (error) {
    if (isSnapshotRequired(error)) {
      await closeTransport(key);
      await generationClient.recover(handle);
      return;
    }
    if (!controller.signal.aborted) {
      await closeTransport(key);
      useGenerationTasksStore.getState().dispatch(key, {
        type: "recovery_state",
        state: "failed",
      });
      throw error;
    }
  } finally {
    if (
      sessions.get(key) === session
      && (shouldUseHttpBackend() || session.closed || !session.subscriptionId)
    ) {
      sessions.delete(key);
    }
  }
}

async function runWebTransport(
  handle: GenerationTaskHandle,
  afterSequence: number | undefined,
  session: GenerationTransportSession,
): Promise<void> {
  let cursor = afterSequence;
  let retry = 0;
  let terminalVerification: Promise<void> | undefined;
  while (!session.controller.signal.aborted) {
    try {
      const headers = new Headers({ Accept: "text/event-stream" });
      if (cursor != null) headers.set("Last-Event-ID", `${handle.attempt}:${cursor}`);
      const response = await authenticatedWebFetch(
        `/generation-tasks/${encodeURIComponent(handle.task_id)}/events?attempt=${handle.attempt}`,
        { headers, signal: session.controller.signal },
      );
      if (!response.ok) {
        const error = await readHostError(response);
        if (error.code === "generation_snapshot_required") throw error;
        throw new Error(error.message);
      }
      if (!response.body) throw new Error("generation SSE response 缺少 body");
      await readSseStream(response.body, (frame) => {
        if (frame.event !== "generation_task_event") return;
        if (terminalVerification) throw new Error("generation terminal 后收到额外事件");
        const event = parseGenerationTaskEvent(JSON.parse(frame.data));
        cursor = event.sequence;
        retry = 0;
        terminalVerification = handleEvent(handle, event) ?? terminalVerification;
      }, session.controller.signal);
      await terminalVerification;
    } catch (error) {
      if (session.controller.signal.aborted) return;
      if (!(error instanceof TypeError)) throw error;
    }
    if (session.controller.signal.aborted || isTerminal(handle)) return;
    retry += 1;
    await backoff(retry, session.controller.signal);
  }
}

async function runTauriTransport(
  handle: GenerationTaskHandle,
  afterSequence: number | undefined,
  session: GenerationTransportSession,
): Promise<void> {
  if (!isTauriRuntime()) throw new Error("当前环境不支持 Tauri generation Channel");
  const channel = createRuntimeChannel<unknown>();
  channel.onmessage = (value) => {
    try {
      void handleEvent(handle, parseGenerationTaskEvent(value))?.catch((error) => {
        handleTauriEventError(handle, error);
      });
    } catch (error) {
      handleTauriEventError(handle, error);
    }
  };
  const subscription = await runtimeInvoke<unknown>("subscribe_generation_task", {
    userId: currentUserId(),
    taskId: handle.task_id,
    attempt: handle.attempt,
    afterSequence: afterSequence ?? null,
    onEvent: channel,
  });
  const parsed = parseGenerationSubscriptionHandle(subscription);
  session.subscriptionId = parsed.subscription_id;
  let unsubscribe: (() => Promise<void>) | undefined = async () => {
    const input: UnsubscribeGenerationTaskInput = {
      subscription_id: parsed.subscription_id,
    };
    try {
      await runtimeInvoke<void>("unsubscribe_generation_task", { input });
    } catch (error) {
      // The backend already unregisters subscriptions on Channel drop/window destroy.
      console.warn("generation subscription unsubscribe failed", error);
    }
  };
  session.close = async (backendFinished = false) => {
    session.closed = true;
    session.backendFinished ||= backendFinished;
    session.controller.abort();
    const close = unsubscribe;
    unsubscribe = undefined;
    if (close && !session.backendFinished) await close();
  };
  if (session.closed) await session.close(session.backendFinished);
}

function handleEvent(
  handle: GenerationTaskHandle,
  event: GenerationTaskEvent,
): Promise<void> | undefined {
  if (isTerminalEvent(event) && !shouldUseHttpBackend()) {
    void closeTransport(generationTaskKey(handle.task_id, handle.attempt), true);
  }
  if (event.event.kind === "completed") return handleCompletedEvent(handle, event);
  applyEvent(handle, event);
}

async function handleCompletedEvent(
  handle: GenerationTaskHandle,
  event: GenerationTaskEvent,
): Promise<void> {
  const state = getGenerationTaskViewState(handle.task_id, handle.attempt);
  if (!state || await textHashesMatch(
    event.event.kind === "completed" ? event.event.data.final_text_hashes : [],
    Object.entries(state.streams).map(([streamId, stream]) => ({
      streamId,
      text: stream.text,
      nextOffset: stream.nextOffset,
    })),
  )) {
    applyEvent(handle, event);
    return;
  }

  await generationClient.recover(
    handle,
    event.event.kind === "completed" ? event.event.data.result_ref ?? null : null,
  );
}

function applyEvent(handle: GenerationTaskHandle, event: GenerationTaskEvent): void {
  const key = generationTaskKey(handle.task_id, handle.attempt);
  const next = useGenerationTasksStore.getState().dispatch(key, { type: "event", event });
  if (!next) return;
  if (next.recoveryState === "required") void generationClient.recover(handle);
  if (next.terminalReceived) void closeTransport(key);
}

function isTerminalEvent(event: GenerationTaskEvent): boolean {
  return ["completed", "failed", "cancelled", "interrupted"].includes(event.event.kind);
}

async function snapshotTerminalIsConsistent(snapshot: GenerationTaskSnapshot): Promise<boolean> {
  const terminal = snapshot.terminal_event;
  if (!terminal || terminal.event.kind !== "completed") return true;
  return textHashesMatch(
    terminal.event.data.final_text_hashes,
    snapshot.streams.map((stream) => ({
      streamId: stream.stream_id,
      text: stream.text,
      nextOffset: stream.next_offset,
      sha256: stream.sha256,
    })),
  );
}

interface VisibleTextStream {
  streamId: string;
  text: string;
  nextOffset: number;
  sha256?: string;
}

async function textHashesMatch(
  hashes: CompletedEvent["final_text_hashes"],
  streams: VisibleTextStream[],
): Promise<boolean> {
  const byId = new Map(streams.map((stream) => [stream.streamId, stream]));
  if (hashes.length !== streams.length || byId.size !== streams.length) return false;
  const hashIds = new Set(hashes.map((hash) => hash.stream_id));
  if (hashIds.size !== hashes.length) return false;
  return (await Promise.all(hashes.map(async (hash) => {
    const stream = byId.get(hash.stream_id);
    if (!stream || utf8ByteLength(stream.text) !== stream.nextOffset) return false;
    const digest = await sha256Hex(stream.text);
    return digest !== null
      && hash.utf8_bytes === stream.nextOffset
      && hash.sha256 === digest
      && (stream.sha256 === undefined || stream.sha256 === digest);
  }))).every(Boolean);
}

function markConsistencyFailure(
  handle: GenerationTaskHandle,
  message: string,
  resultRef: GenerationTaskSummary["result_ref"],
): void {
  const key = generationTaskKey(handle.task_id, handle.attempt);
  const store = useGenerationTasksStore.getState();
  store.dispatch(key, { type: "consistency_error", resultRef });
  console.error("generation consistency error", {
    taskId: handle.task_id,
    attempt: handle.attempt,
    message,
  });
}

function handleTauriEventError(handle: GenerationTaskHandle, error: unknown): void {
  const key = generationTaskKey(handle.task_id, handle.attempt);
  const store = useGenerationTasksStore.getState();
  if (getGenerationTaskViewState(handle.task_id, handle.attempt)?.clientState !== "consistency_error") {
    store.dispatch(key, { type: "client_state", state: "protocol_error" });
    store.dispatch(key, { type: "recovery_state", state: "failed" });
  }
  void closeTransport(key);
  console.error("generation Tauri Channel event invalid", error);
}

function isTerminal(handle: GenerationTaskHandle): boolean {
  return Boolean(getGenerationTaskViewState(handle.task_id, handle.attempt)?.terminalReceived);
}

async function closeTransport(key: string, backendFinished = false): Promise<void> {
  const session = sessions.get(key);
  if (!session) return;
  sessions.delete(key);
  await session.close(backendFinished);
}

async function rollbackOnAttachFailure(
  task: Promise<void>,
  key: string,
): Promise<void> {
  try {
    await task;
  } catch (error) {
    useGenerationTasksStore.getState().release(key);
    throw error;
  }
}

async function readResponseJson(response: Response): Promise<unknown> {
  if (response.ok) return response.status === 204 ? undefined : response.json();
  throw await readHostError(response);
}

async function readHostError(response: Response): Promise<ReturnType<typeof parseHostErrorResponse>> {
  const value: unknown = await response.json().catch(() => ({
    code: "generation_checkpoint_unavailable",
    message: `${response.status} ${response.statusText}`,
    retryable: response.status >= 500,
  }));
  return parseHostErrorResponse(value);
}

function isSnapshotRequired(error: unknown): boolean {
  return typeof error === "object"
    && error !== null
    && "code" in error
    && (error as { code?: unknown }).code === "generation_snapshot_required";
}

async function backoff(attempt: number, signal: AbortSignal): Promise<void> {
  const base = Math.min(15_000, 100 * 2 ** Math.min(attempt - 1, 8));
  const jitter = Math.floor(base * 0.2 * Math.random());
  await new Promise<void>((resolve) => {
    const timer = setTimeout(resolve, base + jitter);
    signal.addEventListener("abort", () => {
      clearTimeout(timer);
      resolve();
    }, { once: true });
  });
}
