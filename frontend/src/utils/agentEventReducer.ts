import type {
  AgentEvent,
  AgentMessageView,
  AgentRunView,
  AgentThreadSnapshot,
} from "../api/agent";

export interface AgentEventReduction {
  snapshot: AgentThreadSnapshot;
  cursor: number;
  needsReload: boolean;
}

const MESSAGE_EVENT_KINDS = new Set([
  "message_appended",
  "message_delta",
  "message_completed",
  "message_interrupted",
]);

const RUN_EVENT_STATUSES: Record<string, AgentRunView["status"]> = {
  run_queued: "queued",
  run_waiting: "waiting",
  run_completed: "completed",
  run_failed: "failed",
  run_cancelled: "cancelled",
  run_recovery_required: "recovery_required",
};

export function snapshotCursor(snapshot: AgentThreadSnapshot): number {
  const eventCursor = snapshot.events.reduce(
    (cursor, event) => Math.max(cursor, event.sequence),
    0,
  );
  return Math.max(eventCursor, snapshot.thread.next_event_sequence - 1);
}

export function reduceAgentEvent(
  snapshot: AgentThreadSnapshot,
  event: AgentEvent,
  cursor: number,
): AgentEventReduction {
  if (event.thread_id !== snapshot.thread.id || event.sequence > cursor + 1) {
    return { snapshot, cursor, needsReload: true };
  }
  if (event.sequence <= cursor) {
    return { snapshot, cursor, needsReload: false };
  }

  // An editor request carries a live UI operation and an execution that may
  // have been created after this snapshot. Reload the authoritative snapshot
  // before the page decides whether it can answer the request.
  if (event.editor_request) {
    return { snapshot, cursor, needsReload: true };
  }

  const payload = objectPayload(event.payload);
  let next = snapshot;
  if (MESSAGE_EVENT_KINDS.has(event.kind)) {
    const message = messageProjection(payload?.message);
    if (!message || (event.message_id != null && event.message_id !== message.id)) {
      return { snapshot, cursor, needsReload: true };
    }
    const messages = snapshot.messages.some((item) => item.id === message.id)
      ? snapshot.messages.map((item) => (item.id === message.id ? message : item))
      : [...snapshot.messages, message];
    next = { ...next, messages };
  } else if (event.kind === "message_queued") {
    // The event only carries the ID; the message projection is not safe to
    // invent locally. The next snapshot also materializes a newly admitted run.
    return { snapshot, cursor, needsReload: true };
  } else if (event.kind in RUN_EVENT_STATUSES || event.kind === "run_controlled") {
    if (!event.run_id) return { snapshot, cursor, needsReload: true };
    const index = snapshot.runs.findIndex((run) => run.id === event.run_id);
    const status = event.kind === "run_controlled"
      ? payload?.status
      : RUN_EVENT_STATUSES[event.kind];
    if (index < 0 || typeof status !== "string") {
      return { snapshot, cursor, needsReload: true };
    }
    const run = snapshot.runs[index];
    const nextRun: AgentRunView = {
      ...run,
      status: status as AgentRunView["status"],
      error: typeof payload?.error === "string" ? payload.error : run.error,
    };
    const runs = [...snapshot.runs];
    runs[index] = nextRun;
    next = { ...next, runs };
  }

  const events = [...snapshot.events, event].slice(-1_000);
  next = {
    ...next,
    events,
    thread: {
      ...next.thread,
      next_event_sequence: Math.max(next.thread.next_event_sequence, event.sequence + 1),
    },
  };
  return { snapshot: next, cursor: event.sequence, needsReload: false };
}

function objectPayload(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function messageProjection(value: unknown): AgentMessageView | null {
  const message = objectPayload(value);
  if (!message) return null;
  const requiredStrings = [
    "id",
    "thread_id",
    "role",
    "status",
    "content",
    "created_at",
  ];
  if (requiredStrings.some((key) => typeof message[key] !== "string")) return null;
  if (message.thread_id === "" || typeof message.metadata === "undefined") return null;
  if (message.updated_at !== null && typeof message.updated_at !== "string") return null;
  return message as unknown as AgentMessageView;
}
