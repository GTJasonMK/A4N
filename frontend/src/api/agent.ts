import { apiInvoke, authenticatedWebFetch } from ".";
import { buildRuntimeConfigSnapshot } from "./runtimeConfigSnapshot";
import { readSseStream } from "../utils/sseParser";
import { runtimeInvoke } from "./runtime";
import { shouldUseHttpBackend } from "../utils/runtimeMode";
import type {
  AgentApprovalView,
  RespondAgentEditorInput,
  AgentEditorReply,
  AgentDelegationView,
  AgentEvent,
  AgentEventsInput,
  AgentMemoryInput,
  AgentMemoryView,
  AgentMessageAccepted,
  AgentMessageView,
  AgentRunInput,
  AgentRunView,
  AgentSkillProfileView,
  AgentSkillSettingsView,
  AgentThreadSnapshot,
  AgentThreadView,
  AgentToolDescriptor,
  AgentToolSearchResult,
  ArchiveAgentThreadInput,
  ControlAgentRunInput,
  ControlChildAgentInput,
  CreateAgentThreadInput,
  DescribeAgentToolInput,
  EnsureMainAgentThreadInput,
  ForkAgentThreadInput,
  ListAgentMessagesInput,
  ListAgentThreadsInput,
  RetractAgentMemoryInput,
  ResolveAgentApprovalInput,
  SearchAgentToolsInput,
  SendAgentMessageInput,
  SendAgentMessageToChildInput,
  SpawnAgentInput,
  UpdateAgentThreadInput,
  UpdateAgentSkillConfigurationInput,
} from "./generated/host";

const AGENT_EVENT_POLL_INTERVAL_MS = 100;
const CLEAR_CONTEXT_WAIT_TIMEOUT_MS = 30_000;
const CLEAR_CONTEXT_POLL_INTERVAL_MS = 200;

export const ACTIVE_AGENT_RUN_STATUSES = new Set<AgentRunView["status"]>([
  "queued",
  "running",
  "waiting",
  "pausing",
  "paused",
  "cancelling",
]);

export type {
  AgentEditorSource,
  AgentEditorState,
  AgentEditorRequest,
  AgentEditorResponse,
  AgentApprovalView,
  AgentBudget,
  AgentControlAction,
  AgentDeliveryMode,
  AgentEvent,
  AgentExecutionView,
  AgentExecutionMode,
  AgentMemoryInput,
  AgentMemoryView,
  AgentMessageAccepted,
  AgentMessageView,
  AgentResourceRef,
  AgentRunInput,
  AgentRunView,
  AgentSkillContent,
  AgentSkillDescriptor,
  AgentSkillProfileView,
  AgentSkillSearchResult,
  AgentSkillSettingsView,
  AgentDelegationView,
  AgentScope,
  AgentThreadKind,
  AgentThreadSnapshot,
  AgentThreadView,
  AgentToolDescriptor,
  AgentToolSearchResult,
  ControlAgentRunInput,
  ControlChildAgentInput,
  CreateAgentThreadInput,
  DescribeAgentToolInput,
  EnsureMainAgentThreadInput,
  ForkAgentThreadInput,
  ListAgentMessagesInput,
  ListAgentThreadsInput,
  RetractAgentMemoryInput,
  ResolveAgentApprovalInput,
  SearchAgentToolsInput,
  SendAgentMessageInput,
  SendAgentMessageToChildInput,
  SpawnAgentInput,
  UpdateAgentThreadInput,
  UpdateAgentSkillConfigurationInput,
} from "./generated/host";

/** A thread remains usable until it is explicitly archived. */
export function isAgentThreadOperational(
  thread: { status: string },
): boolean {
  return thread.status !== "archived";
}

export function activeAgentRun(
  snapshot: AgentThreadSnapshot | null,
): AgentRunView | null {
  return snapshot
    ? [...snapshot.runs]
        .reverse()
        .find((run) => ACTIVE_AGENT_RUN_STATUSES.has(run.status)) ?? null
    : null;
}

export async function clearAgentThreadContext(
  threadId: string,
  timeoutMessage: string,
): Promise<void> {
  const deadline = Date.now() + CLEAR_CONTEXT_WAIT_TIMEOUT_MS;
  const cancellationRequested = new Set<string>();

  const waitForRunsToStop = async (): Promise<void> => {
    let snapshot = await getAgentThread(threadId);
    while (Date.now() < deadline) {
      const run = activeAgentRun(snapshot);
      if (!run) return;
      if (!cancellationRequested.has(run.id)) {
        try {
          await controlAgentRun({
            thread_id: threadId,
            run_id: run.id,
            action: "cancel",
          });
        } catch (error) {
          const next = await getAgentThread(threadId);
          if (!activeAgentRun(next)) return;
          throw error;
        }
        cancellationRequested.add(run.id);
      }
      await new Promise((resolve) =>
        setTimeout(resolve, CLEAR_CONTEXT_POLL_INTERVAL_MS),
      );
      snapshot = await getAgentThread(threadId);
    }
    throw new Error(timeoutMessage);
  };

  await waitForRunsToStop();
  let archiveError: unknown = null;
  while (Date.now() < deadline) {
    try {
      await archiveAgentThread({ thread_id: threadId });
      archiveError = null;
      break;
    } catch (error) {
      archiveError = error;
      const snapshot = await getAgentThread(threadId);
      if (activeAgentRun(snapshot)) {
        await waitForRunsToStop();
      } else {
        await new Promise((resolve) =>
          setTimeout(resolve, CLEAR_CONTEXT_POLL_INTERVAL_MS),
        );
      }
    }
  }
  if (archiveError) throw archiveError;

  const snapshot = await getAgentThread(threadId);
  if (activeAgentRun(snapshot) || snapshot.thread.status !== "archived") {
    throw new Error(timeoutMessage);
  }
}

export function respondAgentEditor(input: RespondAgentEditorInput): Promise<AgentEditorReply> {
  return apiInvoke("respond_agent_editor", { input });
}

export class AgentRequestError extends Error {
  constructor(
    readonly status: number,
    message: string,
    readonly code?: string,
  ) {
    super(message);
    this.name = "AgentRequestError";
  }
}

export function listAgentThreads(
  input: ListAgentThreadsInput = {},
): Promise<AgentThreadView[]> {
  return agentOperation(
    "list_agent_threads",
    input,
    agentThreadsPath(input),
  );
}

export function listAgentSkills(): Promise<AgentSkillSettingsView> {
  return apiInvoke("list_agent_skills");
}

export function updateAgentSkillConfiguration(
  input: UpdateAgentSkillConfigurationInput,
): Promise<AgentSkillProfileView> {
  return apiInvoke("update_agent_skill_configuration", { input });
}

export function ensureMainAgentThread(
  input: EnsureMainAgentThreadInput = {},
): Promise<AgentThreadView> {
  return agentOperation(
    "ensure_main_agent_thread",
    input,
    "/agent-threads/default",
    jsonRequest("POST", input),
  );
}

export function createAgentThread(
  input: CreateAgentThreadInput,
): Promise<AgentThreadView> {
  return agentOperation(
    "create_agent_thread",
    input,
    "/agent-threads",
    jsonRequest("POST", input),
  );
}

/** Create a child thread from an existing thread snapshot. */
export function forkAgentThread(
  input: ForkAgentThreadInput,
): Promise<AgentThreadView> {
  return agentOperation(
    "fork_agent_thread",
    input,
    `/agent-threads/${pathId(input.thread_id, "thread_id")}/fork`,
    jsonRequest("POST", input),
  );
}

export function updateAgentThread(
  input: UpdateAgentThreadInput,
): Promise<AgentThreadView> {
  return agentOperation(
    "update_agent_thread",
    input,
    `/agent-threads/${pathId(input.thread_id, "thread_id")}`,
    jsonRequest("PATCH", input),
  );
}

export function archiveAgentThread(
  input: ArchiveAgentThreadInput,
): Promise<AgentThreadView> {
  return agentOperation(
    "archive_agent_thread",
    input,
    `/agent-threads/${pathId(input.thread_id, "thread_id")}/archive`,
    jsonRequest("POST", input),
  );
}

export function getAgentThread(threadId: string): Promise<AgentThreadSnapshot> {
  return agentOperation(
    "get_agent_thread",
    { thread_id: threadId },
    `/agent-threads/${pathId(threadId, "thread_id")}`,
  );
}

export function listAgentMessages(
  input: ListAgentMessagesInput,
): Promise<AgentMessageView[]> {
  return agentOperation(
    "list_agent_messages",
    input,
    withQuery(
      `/agent-threads/${pathId(input.thread_id, "thread_id")}/messages`,
      { before: input.before, limit: input.limit },
    ),
  );
}

export function getAgentRun(input: AgentRunInput): Promise<AgentRunView> {
  return agentOperation(
    "get_agent_run",
    input,
    `/agent-runs/${pathId(input.run_id, "run_id")}`,
  );
}

export function sendAgentMessage(
  input: SendAgentMessageInput,
): Promise<AgentMessageAccepted> {
  const runtimeConfigSnapshot = buildRuntimeConfigSnapshot();
  const requestInput =
    Object.prototype.hasOwnProperty.call(input, "runtime_config_snapshot") ||
    !runtimeConfigSnapshot
      ? input
      : { ...input, runtime_config_snapshot: runtimeConfigSnapshot };
  return agentOperation(
    "send_agent_message",
    requestInput,
    `/agent-threads/${pathId(input.thread_id, "thread_id")}/messages`,
    jsonRequest("POST", requestInput),
  );
}

export function controlAgentRun(input: ControlAgentRunInput): Promise<AgentRunView> {
  return agentOperation(
    "control_agent_run",
    input,
    `/agent-runs/${pathId(input.run_id, "run_id")}/control`,
    jsonRequest("POST", input),
  );
}

export function listAgentEvents(input: AgentEventsInput): Promise<AgentEvent[]> {
  return agentOperation("list_agent_events", input, agentEventsPath(input));
}

export function resolveAgentApproval(
  input: ResolveAgentApprovalInput,
): Promise<AgentApprovalView> {
  return agentOperation(
    "resolve_agent_approval",
    input,
    `/agent-approvals/${pathId(input.approval_id, "approval_id")}`,
    jsonRequest("POST", input),
  );
}

export function spawnAgent(
  input: SpawnAgentInput,
): Promise<AgentDelegationView> {
  const runtimeConfigSnapshot = buildRuntimeConfigSnapshot();
  const requestInput =
    Object.prototype.hasOwnProperty.call(input, "runtime_config_snapshot") ||
    !runtimeConfigSnapshot
      ? input
      : { ...input, runtime_config_snapshot: runtimeConfigSnapshot };
  return agentOperation(
    "spawn_agent",
    requestInput,
    `/agent-runs/${pathId(input.parent_run_id, "parent_run_id")}/children`,
    jsonRequest("POST", requestInput),
  );
}

export function sendAgentMessageToChild(
  input: SendAgentMessageToChildInput,
): Promise<AgentMessageAccepted> {
  const runtimeConfigSnapshot = buildRuntimeConfigSnapshot();
  const requestInput =
    Object.prototype.hasOwnProperty.call(input, "runtime_config_snapshot") ||
    !runtimeConfigSnapshot
      ? input
      : { ...input, runtime_config_snapshot: runtimeConfigSnapshot };
  return agentOperation(
    "send_agent_message_to_child",
    requestInput,
    `/agent-delegations/${pathId(input.delegation_id, "delegation_id")}/messages`,
    jsonRequest("POST", requestInput),
  );
}

export function controlChildAgent(
  input: ControlChildAgentInput,
): Promise<AgentRunView> {
  return agentOperation(
    "control_child_agent",
    input,
    `/agent-delegations/${pathId(input.delegation_id, "delegation_id")}/control`,
    jsonRequest("POST", input),
  );
}

export function searchAgentTools(
  input: SearchAgentToolsInput = {},
): Promise<AgentToolSearchResult> {
  return agentOperation("search_agent_tools", input, agentToolsPath(input));
}

export function describeAgentTool(
  input: DescribeAgentToolInput,
): Promise<AgentToolDescriptor> {
  return agentOperation(
    "describe_agent_tool",
    input,
    `/agent-tools/${pathId(input.name, "name")}`,
  );
}

export function listAgentMemories(
  input: AgentMemoryQuery = {},
): Promise<AgentMemoryView[]> {
  return agentOperation(
    "list_agent_memories",
    input,
    withQuery("/agent-memories", {
      scope_kind: input.scope_kind,
      scope_id: input.scope_id,
    }),
  );
}

type AgentMemoryQuery = {
  scope_kind?: AgentMemoryInput["scope_kind"];
  scope_id?: string;
};

export function createAgentMemory(
  input: AgentMemoryInput,
): Promise<AgentMemoryView> {
  return agentOperation(
    "create_agent_memory",
    input,
    "/agent-memories",
    jsonRequest("POST", input),
  );
}

export function retractAgentMemory(
  input: RetractAgentMemoryInput,
): Promise<AgentMemoryView> {
  return agentOperation(
    "retract_agent_memory",
    input,
    `/agent-memories/${pathId(input.memory_id, "memory_id")}/retract`,
    jsonRequest("POST", input),
  );
}

export async function subscribeAgentThreadEvents(
  input: AgentEventsInput,
  onEvent: (event: AgentEvent) => void,
  signal: AbortSignal,
  onConnected?: () => void,
): Promise<void> {
  if (!shouldUseHttpBackend()) {
    await pollAgentThreadEvents(input, onEvent, signal, onConnected);
    return;
  }
  let lastSequence = input.after_sequence ?? 0;
  const headers = new Headers({ Accept: "text/event-stream" });
  if (input.after_sequence != null) {
    headers.set("Last-Event-ID", String(input.after_sequence));
  }
  const response = await authenticatedWebFetch(agentEventsPath(input), {
    headers,
    signal,
  });
  if (!response.ok) throw await agentError(response);
  if (!response.body) {
    throw new AgentRequestError(response.status, "Agent SSE 响应缺少 body");
  }
  onConnected?.();
  await readSseStream(
    response.body,
    (frame) => {
      if (frame.event !== "agent_event") {
        throw new Error(`未知 Agent SSE event: ${frame.event}`);
      }
      const event = parseAgentEvent(frame.data, input.thread_id);
      if (frame.id != null && frame.id !== String(event.sequence)) {
        throw new Error("Agent SSE id 与 sequence 不一致");
      }
      if (event.sequence <= lastSequence) {
        throw new Error(
          `Agent SSE sequence 非递增: previous=${lastSequence}, current=${event.sequence}`,
        );
      }
      lastSequence = event.sequence;
      onEvent(event);
    },
    signal,
  );
}

/**
 * Desktop Agent events use the same durable cursor contract as SSE. Tauri has
 * no HTTP stream, so poll the snapshot endpoint and preserve strict ordering.
 */
async function pollAgentThreadEvents(
  input: AgentEventsInput,
  onEvent: (event: AgentEvent) => void,
  signal: AbortSignal,
  onConnected?: () => void,
): Promise<void> {
  let lastSequence = input.after_sequence ?? 0;
  let connected = false;
  while (!signal.aborted) {
    const events = await listAgentEvents({
      ...input,
      after_sequence: lastSequence,
    });
    if (!connected) {
      connected = true;
      onConnected?.();
    }
    for (const event of events) {
      validateAgentEvent(event, input.thread_id, lastSequence);
      lastSequence = event.sequence;
      onEvent(event);
    }
    if (signal.aborted) return;
    await waitForAgentPoll(signal);
  }
}

function validateAgentEvent(
  event: AgentEvent,
  expectedThreadId: string,
  previousSequence: number,
): void {
  if (
    event.thread_id !== expectedThreadId
    || !Number.isSafeInteger(event.sequence)
    || event.sequence <= previousSequence
    || typeof event.kind !== "string"
    || !event.kind
    || typeof event.actor_kind !== "string"
    || !event.actor_kind
    || typeof event.created_at !== "string"
    || !event.created_at
  ) {
    throw new Error("Agent event 合同无效或 sequence 非递增");
  }
}

function waitForAgentPoll(signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    let timer: ReturnType<typeof setTimeout>;
    const onAbort = () => {
      clearTimeout(timer);
      signal.removeEventListener("abort", onAbort);
      resolve();
    };
    timer = setTimeout(() => {
      signal.removeEventListener("abort", onAbort);
      resolve();
    }, AGENT_EVENT_POLL_INTERVAL_MS);
    signal.addEventListener("abort", onAbort, { once: true });
  });
}

async function agentOperation<T>(
  nativeCommand: string,
  args: unknown,
  path: string,
  init?: RequestInit,
): Promise<T> {
  if (!shouldUseHttpBackend()) {
    const nativeArgs =
      args && typeof args === "object" && !Array.isArray(args)
        ? (args as Record<string, unknown>)
        : {};
    return runtimeInvoke<T>("invoke_agent_command", {
      command: nativeCommand,
      args: nativeArgs,
    });
  }
  return agentRequest<T>(path, init);
}

async function agentRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await authenticatedWebFetch(path, init);
  if (!response.ok) throw await agentError(response);
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

function jsonRequest(method: "POST" | "PATCH", body: unknown): RequestInit {
  return {
    method,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  };
}

function agentThreadsPath(input: ListAgentThreadsInput): string {
  return withQuery("/agent-threads", {
    capability: input.capability,
    include_archived: input.include_archived,
    limit: input.limit,
  });
}

function agentEventsPath(input: AgentEventsInput): string {
  return withQuery(`/agent-threads/${pathId(input.thread_id, "thread_id")}/events`, {
    after_sequence: input.after_sequence,
    limit: input.limit,
  });
}

function agentToolsPath(input: SearchAgentToolsInput): string {
  return withQuery("/agent-tools", {
    query: input.query,
    domain: input.domain,
    effect: input.effect,
    capability: input.capability,
    limit: input.limit,
  });
}

function withQuery(
  path: string,
  values: Record<string, string | number | boolean | null | undefined>,
): string {
  const query = new URLSearchParams();
  for (const [key, value] of Object.entries(values)) {
    if (value != null) query.set(key, String(value));
  }
  const suffix = query.toString();
  return suffix ? `${path}?${suffix}` : path;
}

function pathId(value: string | null | undefined, field: string): string {
  const normalized = value?.trim();
  if (!normalized) throw new Error(`${field} 不能为空`);
  return encodeURIComponent(normalized);
}

function parseAgentEvent(data: string, expectedThreadId: string): AgentEvent {
  const value: unknown = JSON.parse(data);
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("Agent SSE data 不是对象");
  }
  const event = value as Partial<AgentEvent>;
  if (
    event.thread_id !== expectedThreadId
    || !Number.isSafeInteger(event.sequence)
    || (event.sequence ?? 0) <= 0
    || typeof event.kind !== "string"
    || !event.kind
    || typeof event.actor_kind !== "string"
    || !event.actor_kind
    || typeof event.created_at !== "string"
    || !event.created_at
  ) {
    throw new Error("Agent SSE event 合同无效");
  }
  return event as AgentEvent;
}

async function agentError(response: Response): Promise<AgentRequestError> {
  let value: unknown;
  try {
    value = await response.json();
  } catch {
    throw invalidAgentErrorContract(response.status);
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw invalidAgentErrorContract(response.status);
  }
  const body = value as Record<string, unknown>;
  if (
    Object.keys(body).length !== 5
    || typeof body.code !== "string"
    || !body.code.trim()
    || typeof body.message !== "string"
    || !body.message.trim()
    || typeof body.retryable !== "boolean"
    || (body.trace_id !== null
      && (typeof body.trace_id !== "string" || !body.trace_id.trim()))
    || !body.details
    || typeof body.details !== "object"
    || Array.isArray(body.details)
  ) {
    throw invalidAgentErrorContract(response.status);
  }
  return new AgentRequestError(response.status, body.message, body.code);
}

function invalidAgentErrorContract(status: number): Error {
  return new Error(`Agent 返回的 HTTP ${status} 错误体不符合 A4N API 合同`);
}
