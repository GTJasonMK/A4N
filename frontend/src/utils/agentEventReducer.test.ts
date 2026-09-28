import { describe, expect, it } from "vitest";
import type {
  AgentEvent,
  AgentMessageView,
  AgentRunView,
  AgentThreadSnapshot,
} from "../api/agent";
import {
  reduceAgentEvent,
  loadAgentSnapshotWithEvents,
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
  attempt: 1,
  budget: {
    max_children: null,
    max_cost_micros: null,
    max_delegation_depth: null,
    max_duration_seconds: null,
    max_model_calls: null,
    max_parallel_tools: null,
    max_steps: null,
    max_tokens: null,
    max_tool_calls: null,
  },
  budget_limiting_run_id: null,
  context_capacity_tokens: null,
  context_input_tokens: null,
  cost_micros: null,
  created_at: "2026-09-12T00:00:00Z",
  error: null,
  id: "run-1",
  lease_expires_at: null,
  lease_owner: null,
  model_calls_used: 0,
  started_at: null,
  status: "running",
  steps_used: 0,
  thread_id: "thread-1",
  tokens_reserved: 0,
  tokens_used: 0,
  tool_calls_used: 0,
  turn_id: "turn-1",
  updated_at: "2026-09-12T00:00:00Z",
  usage_complete: false,
  waiting_reason: null,
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
  it.each(["approval_requested", "approval_resolved", "execution_completed", "child_completed", "run_waiting", "run_completed"])("reloads authoritative state for %s", (kind) => {
    const result = reduceAgentEvent(snapshot(), { ...event(1, null), kind }, 0);
    expect(result.needsReload).toBe(true);
    expect(result.cursor).toBe(0);
  });

  it("replays streaming deltas received while loading a snapshot", async () => {
    const pending: AgentEvent[] = [];
    const current = snapshot();
    current.thread.next_event_sequence = 1;
    const next = await loadAgentSnapshotWithEvents(async () => {
      pending.push(event(1, { message: message("arrived during reload") }));
      return current;
    }, pending, () => true);
    expect(next.messages[0].content).toBe("arrived during reload");
    expect(snapshotCursor(next)).toBe(1);
    expect(pending).toEqual([]);
  });

  it("reloads lifecycle changes arriving after the snapshot watermark", async () => {
    const pending = [{ ...event(1, null), kind: "approval_requested" }];
    let reads = 0;
    const next = await loadAgentSnapshotWithEvents(async () => {
      const current = snapshot();
      current.thread.next_event_sequence = ++reads;
      return current;
    }, pending, () => true);
    expect(reads).toBe(2);
    expect(snapshotCursor(next)).toBe(1);
    expect(pending).toEqual([]);
  });

  it("reloads budget ownership and usage when a known run starts waiting", async () => {
    const before = snapshot();
    before.thread.next_event_sequence = 1;
    before.runs = [run()];
    const committed = snapshot();
    committed.runs = [{
      ...run(),
      status: "waiting",
      waiting_reason: "budget",
      budget_limiting_run_id: "parent-run",
      tokens_used: 800,
      model_calls_used: 1,
    }];
    const pending = [{
      ...event(1, { waiting_reason: "budget" }),
      kind: "run_waiting",
      message_id: null,
    }];
    let reads = 0;
    const next = await loadAgentSnapshotWithEvents(
      async () => ++reads === 1 ? before : committed,
      pending,
      () => true,
    );
    expect(next.runs).toEqual(committed.runs);
    expect(reads).toBe(2);
    expect(pending).toEqual([]);
  });

  it("keeps buffered events when an older snapshot request is superseded", async () => {
    const pending = [event(1, { message: message("arrived during both reads") })];
    let current = 1;
    let finishOld!: (value: AgentThreadSnapshot) => void;
    let finishNew!: (value: AgentThreadSnapshot) => void;
    const old = loadAgentSnapshotWithEvents(() => new Promise((resolve) => { finishOld = resolve; }), pending, () => current === 1);
    current = 2;
    const next = loadAgentSnapshotWithEvents(() => new Promise((resolve) => { finishNew = resolve; }), pending, () => current === 2);
    const beforeEvent = snapshot();
    beforeEvent.thread.next_event_sequence = 1;
    finishOld(beforeEvent);
    await old;
    expect(pending).toHaveLength(1);
    finishNew(beforeEvent);
    expect((await next).messages[0].content).toBe("arrived during both reads");
    expect(pending).toEqual([]);
  });

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
            protocol_version: 2,
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

  it("reloads approval and waiting events even for known runs", () => {
    const current = snapshot();
    current.runs = [run()];
    const approval = reduceAgentEvent(
      current,
      {
        ...event(1, { approval_id: "approval-1" }),
        kind: "approval_requested",
        message_id: null,
      },
      0,
    );
    expect(approval.needsReload).toBe(true);

    const waiting = reduceAgentEvent(
      current,
      {
        ...event(1, { waiting_reason: "budget" }),
        kind: "run_waiting",
        message_id: null,
      },
      0,
    );
    expect(waiting.needsReload).toBe(true);
    expect(waiting.snapshot).toBe(current);
    expect(waiting.cursor).toBe(0);
  });

  it("reloads waiting events with an unknown reason", () => {
    const current = snapshot();
    current.runs = [run()];
    const result = reduceAgentEvent(
      current,
      {
        ...event(1, { waiting_reason: "future_reason" }),
        kind: "run_waiting",
        message_id: null,
      },
      0,
    );
    expect(result.needsReload).toBe(true);
    expect(result.snapshot).toBe(current);
  });

  it("reloads run control events with an unknown status", () => {
    const current = snapshot();
    current.runs = [run()];
    const result = reduceAgentEvent(
      current,
      {
        ...event(1, { status: "future_status" }),
        kind: "run_controlled",
        message_id: null,
      },
      0,
    );
    expect(result.needsReload).toBe(true);
    expect(result.snapshot).toBe(current);
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
