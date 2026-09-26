import type {
  AgentEvent,
  AgentMessageView,
  AgentThreadSnapshot,
} from "../api/agent";

export interface AgentEventReduction {
  snapshot: AgentThreadSnapshot;
  cursor: number;
  needsReload: boolean;
}

export function streamedAgentMessageIds(events: AgentEvent[], runId?: string): string[] {
  return [...new Set(
    events
      .filter((event) =>
        event.kind === "message_delta"
        && event.message_id != null
        && (runId == null || event.run_id === runId),
      )
      .map((event) => event.message_id as string),
  )];
}

const MESSAGE_EVENT_KINDS = new Set([
  "message_appended",
  "message_delta",
  "message_completed",
  "message_interrupted",
]);

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
  // Lifecycle events omit budget, usage and lease projections.
  if (event.editor_request || !MESSAGE_EVENT_KINDS.has(event.kind)) {
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

/** Replay events received during a snapshot read before publishing that snapshot. */
export async function loadAgentSnapshotWithEvents(
  load: () => Promise<AgentThreadSnapshot>,
  pending: AgentEvent[],
  isCurrent: () => boolean,
): Promise<AgentThreadSnapshot> {
  let previousCursor = -1;
  for (;;) {
    let snapshot = await load();
    if (!isCurrent()) return snapshot;
    let cursor = snapshotCursor(snapshot);
    let needsReload = false;
    for (const event of pending) {
      if (event.thread_id !== snapshot.thread.id) continue;
      const next = reduceAgentEvent(snapshot, event, cursor);
      if (next.needsReload) {
        needsReload = true;
        break;
      }
      snapshot = next.snapshot;
      cursor = next.cursor;
    }
    if (!needsReload) {
      pending.splice(0);
      return snapshot;
    }
    if (cursor <= previousCursor) {
      throw new Error("Agent 快照未覆盖已收到的事件，无法安全更新状态");
    }
    previousCursor = cursor;
    // A lifecycle event has no complete projection. Fetch its committed state;
    // never advance past it or discard streaming deltas arriving in the meantime.
  }
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
