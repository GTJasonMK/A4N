import { authenticatedWebFetch } from ".";
import { readSseStream } from "../utils/sseParser";
import { getActiveLocale, translateMessage } from "../i18n";

export type AgentStatus = "idle" | "running" | "paused" | "completed" | "failed" | "offline";

export interface AgentView {
  agent_id: string;
  name: string;
  kind: string;
  client: string;
  status: AgentStatus;
  phase: string | null;
  summary: string | null;
  progress: number | null;
  current_task_id: string | null;
  project_id: number | null;
  chapter_id: number | null;
  started_at: string;
  updated_at: string;
  last_heartbeat_at: string;
  expires_at: string;
  metadata: unknown;
}

export interface AgentEvent {
  kind: "registered" | "updated" | "removed";
  agent_id: string;
  agent: AgentView | null;
}

export async function listAgents(): Promise<AgentView[]> {
  const response = await authenticatedWebFetch("/agents");
  if (!response.ok) {
    throw new Error(translateMessage(getActiveLocale(), "errors.agentListLoadFailed", { status: response.status }));
  }
  const value: unknown = await response.json();
  if (!Array.isArray(value)) throw new Error(translateMessage(getActiveLocale(), "errors.agentListResponseInvalid"));
  return value.map(parseAgent);
}

export async function subscribeAgentEvents(
  onEvent: (event: AgentEvent) => void,
  signal: AbortSignal,
): Promise<void> {
  const response = await authenticatedWebFetch("/agents/events", {
    headers: { Accept: "text/event-stream" },
    signal,
  });
  if (!response.ok) {
    throw new Error(translateMessage(getActiveLocale(), "errors.agentEventConnectionFailed", { status: response.status }));
  }
  if (!response.body) throw new Error(translateMessage(getActiveLocale(), "errors.agentEventStreamMissingBody"));
  await readSseStream(response.body, (event) => {
    if (event.event !== "agent_update") return;
    const value = JSON.parse(event.data) as Partial<AgentEvent>;
    if (!value.agent_id || !["registered", "updated", "removed"].includes(value.kind ?? "")) {
      throw new Error(translateMessage(getActiveLocale(), "errors.agentEventResponseInvalid"));
    }
    onEvent({
      kind: value.kind as AgentEvent["kind"],
      agent_id: value.agent_id,
      agent: value.agent == null ? null : parseAgent(value.agent),
    });
  }, signal);
}

function parseAgent(value: unknown): AgentView {
  if (!value || typeof value !== "object") throw new Error(translateMessage(getActiveLocale(), "errors.agentResponseInvalid"));
  const agent = value as Partial<AgentView>;
  if (typeof agent.agent_id !== "string" || typeof agent.name !== "string") {
    throw new Error(translateMessage(getActiveLocale(), "errors.agentIdentityMissing"));
  }
  if (typeof agent.status !== "string" || !Object.prototype.hasOwnProperty.call(STATUS_LABELS, agent.status)) {
    throw new Error(translateMessage(getActiveLocale(), "errors.agentStatusInvalid"));
  }
  return agent as AgentView;
}

const STATUS_LABELS: Record<AgentStatus, true> = {
  idle: true,
  running: true,
  paused: true,
  completed: true,
  failed: true,
  offline: true,
};
