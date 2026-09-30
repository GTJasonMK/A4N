import type {
  CancelRequestedEvent,
  CheckpointEvent,
  GenerationArtifactRef,
  GenerationError,
  GenerationTaskEvent,
  GenerationTaskHandle,
  GenerationTaskSnapshot,
  GenerationTaskStatus,
  GenerationTaskSummary,
  ProgressEvent,
  UsageEvent,
} from "../api/generated/generationContracts";
import type { MessageTranslator } from "../i18n";
import { utf8ByteLength } from "./text";

const EVENT_SCHEMA_VERSION = 1;
const MAX_PENDING_EVENTS = 512;
const MAX_PENDING_BYTES = 1024 * 1024;

type GenerationClientState =
  | "attached"
  | "recovering"
  | "detached"
  | "protocol_error"
  | "consistency_error";

type GenerationRecoveryState =
  | "idle"
  | "required"
  | "recovering"
  | "expired"
  | "corrupt"
  | "failed";

export type GenerationTaskRecoveryIssue =
  | "consistency_error"
  | "expired"
  | "corrupt";

interface GenerationTextStreamState {
  text: string;
  nextOffset: number;
  version: number;
  lastSequence?: number;
}

export interface GenerationTaskViewState {
  taskId: string;
  attempt: number;
  taskStatus: GenerationTaskStatus;
  started: GenerationTaskSnapshot["started"] | null;
  clientState: GenerationClientState;
  nextSequence: number;
  progress: ProgressEvent | null;
  streams: Record<string, GenerationTextStreamState>;
  artifacts: GenerationArtifactRef[];
  latestCheckpoint: CheckpointEvent | null;
  usage: UsageEvent["usage"] | null;
  error: GenerationError | null;
  resultRef: GenerationTaskSummary["result_ref"];
  completionSummary: string | null;
  terminalReceived: boolean;
  recoveryState: GenerationRecoveryState;
  pendingEvents: GenerationTaskEvent[];
  pendingBytes: number;
}

export function generationTaskRecoveryIssue(
  state: Pick<GenerationTaskViewState, "clientState" | "recoveryState">,
): GenerationTaskRecoveryIssue | null {
  if (state.clientState === "consistency_error") return "consistency_error";
  return state.recoveryState === "expired" || state.recoveryState === "corrupt"
    ? state.recoveryState
    : null;
}

export function generationTaskRecoveryIssueMessage(
  issue: GenerationTaskRecoveryIssue,
  t: MessageTranslator,
): string {
  if (issue === "expired") return t("feedback.generationRecovery.snapshotExpired");
  if (issue === "corrupt") return t("feedback.generationRecovery.snapshotCorrupt");
  return t("feedback.generationRecovery.consistencyFailed");
}

/** 正式结果恢复的产品域；四个文本 generation 共用同一裁决。 */
type FormalResultRecoveryDomain =
  | "chapter"
  | "blueprint"
  | "optimize_turn"
  | "inspiration_message";

type FormalResultRecoveryKind = "ok" | "readonly" | "unrecoverable";

interface FormalResultRecoveryInput {
  t: MessageTranslator;
  recoveryIssue: GenerationTaskRecoveryIssue | null;
  /** 正式对象存在且 revision/id 与 result_ref 一致 */
  consistent: boolean;
  hasPersisted: boolean;
  hasResultRef: boolean;
  domain: FormalResultRecoveryDomain;
}

interface FormalResultRecoveryOutcome {
  kind: FormalResultRecoveryKind;
  issueMessage: string;
  /** 是否需要把 client 标为 consistency_error（已有 recoveryIssue 时不再重复标记） */
  shouldMarkConsistencyError: boolean;
  /** 主用户提示（toast / Error / 助手正文） */
  message: string;
  /** 只读横幅文案；unrecoverable/ok 时为 null */
  warning: string | null;
}

/**
 * 统一正式结果恢复裁决：verified terminal 与 consistency_error/expired/corrupt
 * 共用同一套只读展示或不可恢复失败语义，禁止各页面再写第二套状态机。
 */
export function resolveFormalResultRecovery(
  input: FormalResultRecoveryInput,
): FormalResultRecoveryOutcome {
  const issueMessage = input.recoveryIssue
    ? generationTaskRecoveryIssueMessage(input.recoveryIssue, input.t)
    : input.t("feedback.generationRecovery.consistencyFailed");
  if (!input.recoveryIssue && input.consistent) {
    return {
      kind: "ok",
      issueMessage,
      shouldMarkConsistencyError: false,
      message: issueMessage,
      warning: null,
    };
  }
  const shouldMarkConsistencyError = input.recoveryIssue == null;
  if (input.hasPersisted) {
    const { message, warning } = formalResultReadonlyCopy(input.domain, issueMessage, input.t);
    return {
      kind: "readonly",
      issueMessage,
      shouldMarkConsistencyError,
      message,
      warning,
    };
  }
  return {
    kind: "unrecoverable",
    issueMessage,
    shouldMarkConsistencyError,
    message: formalResultUnrecoverableCopy(
      input.domain,
      issueMessage,
      input.hasResultRef,
      input.t,
    ),
    warning: null,
  };
}

function formalResultReadonlyCopy(
  domain: FormalResultRecoveryDomain,
  issueMessage: string,
  t: MessageTranslator,
): { message: string; warning: string } {
  switch (domain) {
    case "chapter":
      return {
        message: t("feedback.generationRecovery.readonlyChapter", { issue: issueMessage }),
        warning: t("feedback.generationRecovery.readonlyChapter", { issue: issueMessage }),
      };
    case "blueprint":
      return {
        message: t("feedback.generationRecovery.readonlyBlueprint", { issue: issueMessage }),
        warning: t("feedback.generationRecovery.readonlyBlueprintWarning", { issue: issueMessage }),
      };
    case "optimize_turn":
      return {
        message: t("feedback.generationRecovery.readonlyOptimizeTurn", { issue: issueMessage }),
        warning: t("feedback.generationRecovery.readonlyOptimizeTurn", { issue: issueMessage }),
      };
    case "inspiration_message":
      return {
        message: t("feedback.generationRecovery.readonlyInspirationMessage", { issue: issueMessage }),
        warning: t("feedback.generationRecovery.readonlyInspirationMessage", { issue: issueMessage }),
      };
  }
}

function formalResultUnrecoverableCopy(
  domain: FormalResultRecoveryDomain,
  issueMessage: string,
  hasResultRef: boolean,
  t: MessageTranslator,
): string {
  if (!hasResultRef) {
    if (domain === "chapter") {
      return t("feedback.generationRecovery.noResultRefChapter", { issue: issueMessage });
    }
    return t("feedback.generationRecovery.noResultRef", { issue: issueMessage });
  }
  switch (domain) {
    case "chapter":
      return t("feedback.generationRecovery.unavailableChapter", { issue: issueMessage });
    case "blueprint":
      return t("feedback.generationRecovery.unavailableBlueprint", { issue: issueMessage });
    case "optimize_turn":
      return t("feedback.generationRecovery.unavailableOptimizeTurn", { issue: issueMessage });
    case "inspiration_message":
      return t("feedback.generationRecovery.unavailableInspirationMessage", { issue: issueMessage });
  }
}

export type GenerationTaskReducerAction =
  | { type: "event"; event: GenerationTaskEvent }
  | { type: "snapshot"; snapshot: GenerationTaskSnapshot }
  | { type: "summary"; summary: GenerationTaskSummary }
  | { type: "client_state"; state: GenerationClientState }
  | { type: "consistency_error"; resultRef: GenerationTaskSummary["result_ref"] }
  | { type: "recovery_state"; state: GenerationRecoveryState };

export function createGenerationTaskViewState(
  handle: GenerationTaskHandle,
): GenerationTaskViewState {
  return {
    taskId: handle.task_id,
    attempt: handle.attempt,
    taskStatus: handle.status,
    started: null,
    clientState: "attached",
    nextSequence: 1,
    progress: null,
    streams: {},
    artifacts: [],
    latestCheckpoint: null,
    usage: null,
    error: null,
    resultRef: null,
    completionSummary: null,
    terminalReceived: false,
    recoveryState: "idle",
    pendingEvents: [],
    pendingBytes: 0,
  };
}

export function reduceGenerationTask(
  state: GenerationTaskViewState,
  action: GenerationTaskReducerAction,
): GenerationTaskViewState {
  switch (action.type) {
    case "event":
      return reduceEvent(state, action.event);
    case "snapshot":
      return applySnapshot(state, action.snapshot);
    case "summary":
      return applySummary(state, action.summary);
    case "client_state":
      return { ...state, clientState: action.state };
    case "consistency_error":
      return {
        ...state,
        clientState: "consistency_error",
        recoveryState: "failed",
        resultRef: action.resultRef ?? state.resultRef,
      };
    case "recovery_state":
      return { ...state, recoveryState: action.state };
  }
}

export function markGenerationTaskDetached(
  state: GenerationTaskViewState,
): GenerationTaskViewState {
  if (state.clientState === "protocol_error" || state.clientState === "consistency_error") {
    return state;
  }
  return { ...state, clientState: "detached" };
}

function reduceEvent(
  state: GenerationTaskViewState,
  event: GenerationTaskEvent,
): GenerationTaskViewState {
  if (event.task_id !== state.taskId) {
    return protocolFailure(state, `事件 task_id=${event.task_id} 与当前任务不一致`);
  }
  if (event.attempt < state.attempt) return state;
  if (event.attempt !== state.attempt) {
    return protocolFailure(
      state,
      `事件 attempt=${event.attempt} 与当前 attempt=${state.attempt} 不一致`,
    );
  }
  if (event.schema_version !== EVENT_SCHEMA_VERSION) {
    return protocolFailure(state, `不支持 generation schema version=${event.schema_version}`);
  }
  if (event.sequence < state.nextSequence) return state;
  if (event.sequence > state.nextSequence) {
    return queueForRecovery(state, event);
  }

  const applied = applyContiguousEvent(state, event);
  if (applied.clientState === "protocol_error") return applied;
  if (applied.recoveryState === "required" && !applied.pendingEvents.length) {
    return applied;
  }
  return drainPending(applied);
}

function applyContiguousEvent(
  state: GenerationTaskViewState,
  event: GenerationTaskEvent,
): GenerationTaskViewState {
  const next = {
    ...state,
    nextSequence: state.nextSequence + 1,
    pendingEvents: state.pendingEvents.slice(),
  };
  switch (event.event.kind) {
    case "started":
      if (state.started) return protocolFailure(state, "重复 started 事件");
      return { ...next, started: event.event.data, taskStatus: "reserved" };
    case "status_changed": {
      const { status } = event.event.data;
      if (!validStatusTransition(state.taskStatus, status)) {
        return protocolFailure(
          state,
          `非法状态转移 ${state.taskStatus} -> ${status}`,
        );
      }
      return { ...next, taskStatus: status };
    }
    case "progress":
      return state.terminalReceived
        ? protocolFailure(state, "终态后收到 progress")
        : {
          ...next,
          // Model-connect/stream events intentionally omit counters. Keep the
          // last known pair so batching or a late phase update cannot erase a
          // durable task total from the view.
          progress: {
            ...event.event.data,
            current: event.event.data.current ?? state.progress?.current,
            total: event.event.data.total ?? state.progress?.total,
          },
        };
    case "text_delta":
      return applyTextDelta(next, event);
    case "usage":
      return state.terminalReceived
        ? protocolFailure(state, "终态后收到 usage")
        : { ...next, usage: event.event.data.usage };
    case "checkpoint":
      return state.terminalReceived
        ? protocolFailure(state, "终态后收到 checkpoint")
        : { ...next, latestCheckpoint: event.event.data };
    case "artifact":
      return state.terminalReceived
        ? protocolFailure(state, "终态后收到 artifact")
        : applyArtifact(next, event.event.data);
    case "cancel_requested":
      return applyCancelRequested(next, event.event.data);
    case "completed":
      return applyTerminal(
        next,
        "completed",
        null,
        event.event.data.final_usage ?? null,
        event.event.data.result_ref ?? null,
        event.event.data.summary ?? null,
      );
    case "failed":
      return applyTerminal(next, "failed", event.event.data.error, null, null, null);
    case "cancelled":
      return applyTerminal(next, "cancelled", null, null, null, null);
    case "interrupted":
      return applyTerminal(next, "interrupted", null, null, null, null);
  }
}

function applyTextDelta(
  state: GenerationTaskViewState,
  event: GenerationTaskEvent,
): GenerationTaskViewState {
  if (state.terminalReceived) return protocolFailure(state, "终态后收到 text_delta");
  const payload = event.event.kind === "text_delta" ? event.event.data : null;
  if (!payload) return state;
  const stream = state.streams[payload.stream_id] ?? {
    text: "",
    nextOffset: 0,
    version: 0,
  };
  if (payload.offset !== stream.nextOffset) {
    return {
      ...state,
      nextSequence: state.nextSequence - 1,
      pendingEvents: [],
      pendingBytes: 0,
      clientState: "recovering",
      recoveryState: "required",
    };
  }
  return {
    ...state,
    streams: {
      ...state.streams,
      [payload.stream_id]: {
        text: stream.text + payload.delta,
        nextOffset: stream.nextOffset + utf8ByteLength(payload.delta),
        version: stream.version + 1,
        lastSequence: event.sequence,
      },
    },
  };
}

function applyArtifact(
  state: GenerationTaskViewState,
  payload: Extract<GenerationTaskEvent["event"], { kind: "artifact" }>["data"],
): GenerationTaskViewState {
  const artifacts = state.artifacts.filter(
    (artifact) => artifact.artifact_id !== payload.artifact.artifact_id,
  );
  if (payload.operation !== "removed") artifacts.push(payload.artifact);
  return { ...state, artifacts };
}

function applyCancelRequested(
  state: GenerationTaskViewState,
  _payload: CancelRequestedEvent,
): GenerationTaskViewState {
  if (state.terminalReceived || !["reserved", "running"].includes(state.taskStatus)) {
    return protocolFailure(state, `状态 ${state.taskStatus} 不允许 cancel_requested`);
  }
  return { ...state, taskStatus: "cancelling" };
}

function applyTerminal(
  state: GenerationTaskViewState,
  status: Extract<GenerationTaskStatus, "completed" | "failed" | "cancelled" | "interrupted">,
  error: GenerationError | null,
  usage: UsageEvent["usage"] | null,
  resultRef: GenerationTaskSummary["result_ref"],
  completionSummary: string | null,
): GenerationTaskViewState {
  if (state.terminalReceived) return protocolFailure(state, "重复终态事件");
  return {
    ...state,
    taskStatus: status,
    error,
    usage: usage ?? state.usage,
    resultRef: resultRef ?? state.resultRef,
    completionSummary,
    terminalReceived: true,
    clientState: "detached",
  };
}

function applySnapshot(
  state: GenerationTaskViewState,
  snapshot: GenerationTaskSnapshot,
): GenerationTaskViewState {
  if (snapshot.task_id !== state.taskId || snapshot.attempt !== state.attempt) {
    return protocolFailure(state, "snapshot 任务身份不一致");
  }
  if (snapshot.schema_version !== EVENT_SCHEMA_VERSION) {
    return protocolFailure(state, `不支持 snapshot schema version=${snapshot.schema_version}`);
  }
  const streams = Object.fromEntries(
    snapshot.streams.map((stream) => [stream.stream_id, {
      text: stream.text,
      nextOffset: stream.next_offset,
      version: 0,
      lastSequence: 0,
    }]),
  );
  const terminalReceived = Boolean(snapshot.terminal_event);
  const next: GenerationTaskViewState = {
    ...state,
    taskStatus: snapshot.status,
    started: snapshot.started,
    clientState: terminalReceived ? "detached" : "attached",
    nextSequence: snapshot.covers_through_sequence + 1,
    progress: snapshot.progress ?? null,
    streams,
    artifacts: snapshot.artifacts.slice(),
    latestCheckpoint: snapshot.latest_checkpoint ?? null,
    usage: snapshot.usage
      ?? (snapshot.terminal_event?.event.kind === "completed"
        ? snapshot.terminal_event.event.data.final_usage ?? null
        : null),
    error: snapshot.terminal_event?.event.kind === "failed"
      ? snapshot.terminal_event.event.data.error
      : null,
    resultRef: snapshot.terminal_event?.event.kind === "completed"
      ? snapshot.terminal_event.event.data.result_ref ?? null
      : state.resultRef,
    completionSummary: snapshot.terminal_event?.event.kind === "completed"
      ? snapshot.terminal_event.event.data.summary ?? null
      : state.completionSummary,
    terminalReceived,
    recoveryState: "idle",
    pendingEvents: state.pendingEvents.slice(),
    pendingBytes: state.pendingBytes,
  };
  return drainPending(next);
}

function applySummary(
  state: GenerationTaskViewState,
  summary: GenerationTaskSummary,
): GenerationTaskViewState {
  if (summary.handle.task_id !== state.taskId || summary.handle.attempt !== state.attempt) {
    return protocolFailure(state, "summary 任务身份不一致");
  }
  if (summary.snapshot_state === "available") {
    return { ...state, resultRef: summary.result_ref };
  }
  return {
    ...state,
    resultRef: summary.result_ref,
    clientState: "detached",
    recoveryState: summary.snapshot_state,
  };
}

function queueForRecovery(
  state: GenerationTaskViewState,
  event: GenerationTaskEvent,
): GenerationTaskViewState {
  const pendingEvents = [...state.pendingEvents, event];
  const pendingBytes = state.pendingBytes + eventBytes(event);
  if (pendingEvents.length > MAX_PENDING_EVENTS || pendingBytes > MAX_PENDING_BYTES) {
    return {
      ...state,
      pendingEvents: [],
      pendingBytes: 0,
      clientState: "recovering",
      recoveryState: "required",
    };
  }
  return {
    ...state,
    pendingEvents,
    pendingBytes,
    clientState: "recovering",
    recoveryState: "required",
  };
}

function drainPending(state: GenerationTaskViewState): GenerationTaskViewState {
  const pendingEvents = state.pendingEvents;
  let pendingIndex = 0;
  let current: GenerationTaskViewState = { ...state, pendingEvents: [] };
  const withRemaining = (): GenerationTaskViewState => ({
    ...current,
    pendingEvents: pendingEvents.slice(pendingIndex),
  });
  while (pendingIndex < pendingEvents.length) {
    const event = pendingEvents[pendingIndex];
    if (event.sequence < current.nextSequence) {
      pendingIndex += 1;
      current.pendingBytes = Math.max(0, current.pendingBytes - eventBytes(event));
      continue;
    }
    if (event.sequence > current.nextSequence) break;
    pendingIndex += 1;
    current.pendingBytes = Math.max(0, current.pendingBytes - eventBytes(event));
    const expectedSequence = current.nextSequence;
    current = applyContiguousEvent(current, event);
    if (current.clientState === "protocol_error") return withRemaining();
    if (
      current.recoveryState === "required"
      && pendingIndex >= pendingEvents.length
      && current.nextSequence === expectedSequence
    ) {
      return withRemaining();
    }
  }
  if (pendingIndex >= pendingEvents.length && current.recoveryState === "required") {
    return {
      ...withRemaining(),
      clientState: "attached",
      recoveryState: "idle",
    };
  }
  return withRemaining();
}

function validStatusTransition(
  current: GenerationTaskStatus,
  next: GenerationTaskStatus,
): boolean {
  return (current === "reserved" && next === "running")
    || (current === "running" && next === "finalizing");
}

function protocolFailure(
  state: GenerationTaskViewState,
  message: string,
): GenerationTaskViewState {
  return {
    ...state,
    clientState: "protocol_error",
    recoveryState: "failed",
    error: protocolError("generation_event_invalid", message, "protocol"),
  };
}

function protocolError(
  code: string,
  message: string,
  stageCode: string,
): GenerationError {
  return {
    code,
    message,
    retryable: false,
    source_error: null,
    stage_code: stageCode,
    trace_id: null,
  };
}

function eventBytes(event: GenerationTaskEvent): number {
  return utf8ByteLength(JSON.stringify(event));
}
