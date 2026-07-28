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
import { utf8ByteLength } from "./text";

const EVENT_SCHEMA_VERSION = 1;
const MAX_PENDING_EVENTS = 512;
const MAX_PENDING_BYTES = 1024 * 1024;

export type GenerationClientState =
  | "attached"
  | "recovering"
  | "detached"
  | "protocol_error"
  | "consistency_error";

export type GenerationRecoveryState =
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

export interface GenerationTextStreamState {
  text: string;
  nextOffset: number;
  version: number;
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
): string {
  if (issue === "expired") return "生成任务恢复快照已过期";
  if (issue === "corrupt") return "生成任务恢复快照已损坏";
  return "生成结果一致性校验失败";
}

/** 正式结果恢复的产品域；四个文本 generation 共用同一裁决。 */
export type FormalResultRecoveryDomain =
  | "chapter"
  | "blueprint"
  | "optimize_turn"
  | "inspiration_message";

export type FormalResultRecoveryKind = "ok" | "readonly" | "unrecoverable";

export interface FormalResultRecoveryInput {
  recoveryIssue: GenerationTaskRecoveryIssue | null;
  /** 正式对象存在且 revision/id 与 result_ref 一致 */
  consistent: boolean;
  hasPersisted: boolean;
  hasResultRef: boolean;
  domain: FormalResultRecoveryDomain;
}

export interface FormalResultRecoveryOutcome {
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
    ? generationTaskRecoveryIssueMessage(input.recoveryIssue)
    : "生成结果一致性校验失败";
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
    const { message, warning } = formalResultReadonlyCopy(input.domain, issueMessage);
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
    ),
    warning: null,
  };
}

function formalResultReadonlyCopy(
  domain: FormalResultRecoveryDomain,
  issueMessage: string,
): { message: string; warning: string } {
  switch (domain) {
    case "chapter":
      return {
        message: `${issueMessage}，已只读加载正式章节供核对`,
        warning: `${issueMessage}，已只读加载正式章节供核对`,
      };
    case "blueprint":
      return {
        message: `${issueMessage}，已只读加载正式蓝图供核对`,
        warning: `${issueMessage}，当前展示正式蓝图，只供核对`,
      };
    case "optimize_turn":
      return {
        message: `${issueMessage}，已只读加载正式 Turn 供核对`,
        warning: `${issueMessage}，已只读加载正式 Turn 供核对`,
      };
    case "inspiration_message":
      return {
        message: `${issueMessage}，以下内容来自正式消息，只供核对`,
        warning: `${issueMessage}，以下内容来自正式消息，只供核对`,
      };
  }
}

function formalResultUnrecoverableCopy(
  domain: FormalResultRecoveryDomain,
  issueMessage: string,
  hasResultRef: boolean,
): string {
  if (!hasResultRef) {
    if (domain === "chapter") {
      return `${issueMessage}，且没有正式结果引用，无法恢复；已恢复编辑器快照`;
    }
    return `${issueMessage}，且没有正式结果引用，无法恢复`;
  }
  switch (domain) {
    case "chapter":
      return `${issueMessage}，正式章节不可用，已恢复编辑器快照`;
    case "blueprint":
      return `${issueMessage}，正式蓝图不可用`;
    case "optimize_turn":
      return `${issueMessage}，正式 Turn 不可用`;
    case "inspiration_message":
      return `${issueMessage}，正式消息不可用，请稍后从历史记录重新加载`;
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
        : { ...next, progress: event.event.data };
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
      );
    case "failed":
      return applyTerminal(next, "failed", event.event.data.error, null, null);
    case "cancelled":
      return applyTerminal(next, "cancelled", null, null, null);
    case "interrupted":
      return applyTerminal(next, "interrupted", null, null, null);
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
): GenerationTaskViewState {
  if (state.terminalReceived) return protocolFailure(state, "重复终态事件");
  return {
    ...state,
    taskStatus: status,
    error,
    usage: usage ?? state.usage,
    resultRef: resultRef ?? state.resultRef,
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
  let current = { ...state, pendingEvents: state.pendingEvents.slice() };
  while (current.pendingEvents.length) {
    const event = current.pendingEvents[0];
    if (event.sequence < current.nextSequence) {
      current.pendingEvents.shift();
      current.pendingBytes = Math.max(0, current.pendingBytes - eventBytes(event));
      continue;
    }
    if (event.sequence > current.nextSequence) break;
    current.pendingEvents.shift();
    current.pendingBytes = Math.max(0, current.pendingBytes - eventBytes(event));
    const expectedSequence = current.nextSequence;
    current = applyContiguousEvent(current, event);
    if (current.clientState === "protocol_error") return current;
    if (
      current.recoveryState === "required"
      && !current.pendingEvents.length
      && current.nextSequence === expectedSequence
    ) {
      return current;
    }
  }
  if (!current.pendingEvents.length && current.recoveryState === "required") {
    return { ...current, clientState: "attached", recoveryState: "idle" };
  }
  return current;
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
