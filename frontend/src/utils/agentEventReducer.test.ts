import { describe, expect, it } from "vitest";
import type {
  AgentEvent,
  AgentMessageView,
  AgentRunView,
  AgentThreadSnapshot,
} from "../api/agent";
import {
  reduceAgentEvent,
  snapshotCursor,
  streamedAgentMessageIds,
} from "./agentEventReducer";

const message = (content: string, status: string = "streaming"): AgentMessageView => ({
  id: "message-1",
  thread_id: "thread-1",
  turn_id: "turn-1",
  run_id: "run-1",
  role: "assistant",
  status: status as AgentMessageView["status"],
  content,
  source_kind: "agent",
  source_id: null,
  metadata: null,
  created_at: "2026-09-12T00:00:00Z",
  updated_at: "2026-09-12T00:00:01Z",
});

const snapshot = (): AgentThreadSnapshot => ({
  protocol_version: 1,
  thread: {
    id: "thread-1",
    kind: "main",
    status: "active",
    title: null,
    capability: null,
    bound_resource: null,
    scope: {} as AgentThreadSnapshot["thread"]["scope"],
    owner_user_id: 1,
    default_model_config_id: null,
    revision: 1,
    next_turn_sequence: 2,
    next_event_sequence: 2,
    context_summary: null,
    protocol_version: 1,
    created_at: "2026-09-12T00:00:00Z",
    updated_at: "2026-09-12T00:00:00Z",
  },
  approvals: [],
  delegations: [],
  events: [],
  executions: [],
  inbox: [],
  memories: [],
  messages: [message("old")],
  runs: [],
  turns: [],
});

const run = (): AgentRunView => ({
  id: "run-1",
  thread_id: "thread-1",
  turn_id: "turn-1",
  attempt: 1,
  status: "running",
  waiting_reason: null,
  lease_owner: null,
  lease_expires_at: null,
  budget: {} as AgentRunView["budget"],
  steps_used: 0,
  model_calls_used: 0,
  tool_calls_used: 0,
  tokens_used: 0,
  context_input_tokens: null,
  context_capacity_tokens: null,
  cost_micros: null,
  updated_at: "2026-09-12T00:00:00Z",
});

const event = (sequence: number, payload: unknown): AgentEvent => ({
  protocol_version: 1,
  thread_id: "thread-1",
  sequence,
  kind: "message_delta",
  actor_kind: "agent",
  actor_id: "run-1",
  turn_id: "turn-1",
  run_id: "run-1",
  execution_id: null,
  delegation_id: null,
  message_id: "message-1",
  payload,
  editor_request: null,
  created_at: "2026-09-12T00:00:02Z",
});

describe("agent event reducer", () => {
  it("identifies streamed messages and can scope them to a run", () => {
    const otherRun = { ...event(2, null), run_id: "run-2", message_id: "message-2" };
    const completed = { ...event(3, null), kind: "message_completed" };
    expect(streamedAgentMessageIds([event(1, null), otherRun, completed])).toEqual([
      "message-1",
      "message-2",
    ]);
    expect(streamedAgentMessageIds([event(1, null), otherRun], "run-2")).toEqual([
      "message-2",
    ]);
  });

  it("uses the durable next sequence as the initial cursor", () => {
    const current = snapshot();
    current.events = [{ ...event(4, null), kind: "run_waiting", message_id: null }];
    current.thread.next_event_sequence = 8;
    expect(snapshotCursor(current)).toBe(7);
  });

  it("merges a full streaming message projection without reloading", () => {
    const result = reduceAgentEvent(
      snapshot(),
      event(1, { message: message("new text") }),
      0,
    );
    expect(result.needsReload).toBe(false);
    expect(result.cursor).toBe(1);
    expect(result.snapshot.messages[0].content).toBe("new text");
    expect(result.snapshot.events).toHaveLength(1);
  });

  it("reloads before handling an editor request event", () => {
    const result = reduceAgentEvent(
      snapshot(),
      {
        ...event(1, { waiting_reason: "external" }),
        kind: "run_waiting",
        message_id: null,
        editor_request: {
          execution_id: "execution-1",
          source: {
            view_instance_id: "view-1",
            project_id: 1,
            chapter_id: 1,
            chapter_number: null,
            editor_revision: "revision-1",
            scene_id: null,
            shot_id: null,
            keyframe_id: null,
            entity_id: null,
          },
          operation: { kind: "inspect" },
        },
      },
      0,
    );
    expect(result.needsReload).toBe(true);
    expect(result.snapshot.events).toHaveLength(0);
  });

  it("reloads after a new approval request", () => {
    const result = reduceAgentEvent(
      snapshot(),
      {
        ...event(1, { approval_id: "approval-1" }),
        kind: "approval_requested",
        message_id: null,
      },
      0,
    );

    expect(result.needsReload).toBe(true);
    expect(result.cursor).toBe(0);
    expect(result.snapshot.approvals).toHaveLength(0);
  });

  it("preserves the waiting reason from run_waiting events", () => {
    const current = snapshot();
    current.runs = [run()];

    const result = reduceAgentEvent(
      current,
      {
        ...event(1, { waiting_reason: "budget" }),
        kind: "run_waiting",
        message_id: null,
      },
      0,
    );

    expect(result.needsReload).toBe(false);
    expect(result.snapshot.runs[0].status).toBe("waiting");
    expect(result.snapshot.runs[0].waiting_reason).toBe("budget");
  });

  it("deduplicates events and detects sequence gaps or malformed projections", () => {
    const current = snapshot();
    const duplicate = reduceAgentEvent(current, event(1, { message: message("new") }), 1);
    expect(duplicate.needsReload).toBe(false);
    expect(duplicate.snapshot).toBe(current);

    const gap = reduceAgentEvent(current, event(3, { message: message("new") }), 1);
    expect(gap.needsReload).toBe(true);

    const malformed = reduceAgentEvent(current, event(1, { message: { id: "message-1" } }), 0);
    expect(malformed.needsReload).toBe(true);
  });
});
