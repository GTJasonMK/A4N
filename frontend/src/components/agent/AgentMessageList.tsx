import { memo, useEffect, useState } from "react";
import type { AgentMessageView, AgentRunView } from "../../api/agent";
import {
  AgentMessageRow,
  agentToolCallIds,
  agentToolResultCallId,
} from "./AgentMessageRow";
import type { Locale } from "../../i18n";
import { parseApiTimestampMs } from "../../utils/timestamp";

interface AgentMessageListProps {
  messages: AgentMessageView[];
  locale: Locale;
  streamedMessageIds?: ReadonlySet<string>;
  pendingMessages?: AgentMessageView[];
  activeRun?: Pick<AgentRunView, "status" | "created_at" | "started_at" | "error"> | null;
  compactAssistantMessages?: boolean;
}

export interface PendingAgentMessage {
  message: AgentMessageView;
  acceptedMessageId: string | null;
}

const EMPTY_PENDING_MESSAGES: AgentMessageView[] = [];

export type PendingAgentMessagesByOwner = Record<string, PendingAgentMessage[]>;

export function updatePendingAgentMessages(
  current: PendingAgentMessagesByOwner,
  ownerId: string,
  update: (messages: PendingAgentMessage[]) => PendingAgentMessage[],
): PendingAgentMessagesByOwner {
  const messages = current[ownerId] ?? [];
  const nextMessages = update(messages);
  if (nextMessages === messages) return current;
  const next = { ...current };
  if (nextMessages.length === 0) delete next[ownerId];
  else next[ownerId] = nextMessages;
  return next;
}

export function appendPendingAgentMessage(
  current: PendingAgentMessagesByOwner,
  ownerId: string,
  message: PendingAgentMessage,
): PendingAgentMessagesByOwner {
  return updatePendingAgentMessages(current, ownerId, (messages) => [...messages, message]);
}

export function acceptPendingAgentMessage(
  current: PendingAgentMessagesByOwner,
  ownerId: string,
  localMessageId: string,
  acceptedMessageId: string,
): PendingAgentMessagesByOwner {
  return updatePendingAgentMessages(current, ownerId, (messages) => {
    let found = false;
    const next = messages.map((item) => {
      if (item.message.id !== localMessageId) return item;
      found = true;
      return { ...item, acceptedMessageId };
    });
    return found ? next : messages;
  });
}

export function removePendingAgentMessage(
  current: PendingAgentMessagesByOwner,
  ownerId: string,
  localMessageId: string,
): PendingAgentMessagesByOwner {
  return updatePendingAgentMessages(current, ownerId, (messages) => {
    const next = messages.filter((item) => item.message.id !== localMessageId);
    return next.length === messages.length ? messages : next;
  });
}

export function clearPendingAgentMessages(
  current: PendingAgentMessagesByOwner,
  ownerId: string,
): PendingAgentMessagesByOwner {
  return updatePendingAgentMessages(current, ownerId, () => []);
}

export function reconcilePendingAgentMessages(
  current: PendingAgentMessagesByOwner,
  ownerId: string,
  messages: ReadonlyArray<Pick<AgentMessageView, "id">>,
): PendingAgentMessagesByOwner {
  const visibleMessageIds = new Set(messages.map((message) => message.id));
  return updatePendingAgentMessages(current, ownerId, (pending) => {
    const next = pending.filter((item) =>
      !item.acceptedMessageId || !visibleMessageIds.has(item.acceptedMessageId),
    );
    return next.length === pending.length ? pending : next;
  });
}

export function createPendingAgentMessage(
  threadId: string,
  content: string,
  clientMessageId: string,
): AgentMessageView {
  const createdAt = new Date().toISOString();
  return {
    content,
    created_at: createdAt,
    id: `pending:${clientMessageId}`,
    metadata: { pending: true },
    role: "user",
    run_id: null,
    source_id: null,
    source_kind: "user",
    status: "complete",
    thread_id: threadId,
    turn_id: null,
    updated_at: createdAt,
  };
}

interface AgentMessageGroup {
  message: AgentMessageView;
  toolResults: AgentMessageView[];
}

export const AgentMessageList = memo(function AgentMessageList({
  messages,
  locale,
  streamedMessageIds,
  pendingMessages = [],
  activeRun = null,
  compactAssistantMessages = false,
}: AgentMessageListProps) {
  const visibleMessages = [
    ...messages,
    ...pendingMessages.filter(
      (pending) => !messages.some((message) => message.id === pending.id),
    ),
  ];
  const groups = groupAgentMessages(visibleMessages);
  const streamingMessage = groups.find(
    ({ message }) => message.role === "assistant" && message.status === "streaming",
  )?.message;
  const run = activeRun ?? (streamingMessage ? {
    status: "running" as AgentRunView["status"],
    created_at: streamingMessage.created_at,
    started_at: null,
    error: null,
  } : null);
  return (
    <div className="agent-message-list">
      {groups.map(({ message, toolResults }) => (
        <AgentMessageRow
          key={message.id}
          message={message}
          locale={locale}
          toolResults={toolResults}
          wasStreamed={streamedMessageIds?.has(message.id)}
          compactAssistantMessages={compactAssistantMessages}
        />
      ))}
      {run ? <AgentRunStatus run={run} locale={locale} /> : null}
    </div>
  );
}, areAgentMessageListPropsEqual);

function areAgentMessageListPropsEqual(
  previous: AgentMessageListProps,
  next: AgentMessageListProps,
): boolean {
  const previousPendingMessages = previous.pendingMessages ?? EMPTY_PENDING_MESSAGES;
  const nextPendingMessages = next.pendingMessages ?? EMPTY_PENDING_MESSAGES;
  if (
    previous.messages !== next.messages ||
    previous.locale !== next.locale ||
    previous.streamedMessageIds !== next.streamedMessageIds ||
    previous.activeRun !== next.activeRun ||
    previous.compactAssistantMessages !== next.compactAssistantMessages ||
    previousPendingMessages.length !== nextPendingMessages.length
  ) return false;
  return previousPendingMessages.every((message, index) => message === nextPendingMessages[index]);
}

function AgentRunStatus({
  run,
  locale,
}: {
  run: Pick<AgentRunView, "status" | "created_at" | "started_at" | "error">;
  locale: Locale;
}) {
  const [now, setNow] = useState(() => Date.now());
  const isRunning = run.status === "running";

  useEffect(() => {
    if (!isRunning) return undefined;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [isRunning]);

  const startedAt = parseApiTimestampMs(run.started_at);
  const elapsed = startedAt !== null
    ? Math.max(0, Math.floor((now - startedAt) / 1000))
    : 0;
  const hours = Math.floor(elapsed / 3_600);
  const minutes = Math.floor((elapsed % 3_600) / 60);
  const seconds = String(elapsed % 60).padStart(2, "0");
  const elapsedLabel = hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}`
    : minutes > 0
      ? `${minutes}:${seconds}`
      : `${elapsed}s`;
  const labels: Record<AgentRunView["status"], string> = locale === "zh-CN" ? {
      queued: "正在排队",
      running: "Agent 正在处理任务",
      waiting: "等待你的确认",
      pausing: "正在暂停任务",
      paused: "任务已暂停，可继续",
      cancelling: "正在取消任务",
      cancelled: "任务已取消",
      completed: "Agent 已完成",
      failed: "Agent 任务失败",
      interrupted: "Agent 任务已中断",
      recovery_required: "Agent 任务需要恢复",
    } : {
      queued: "Queued",
      running: "Agent is working",
      waiting: "Waiting for your confirmation",
      pausing: "Pausing task",
      paused: "Task paused",
      cancelling: "Cancelling task",
      cancelled: "Task cancelled",
      completed: "Agent complete",
      failed: "Agent task failed",
      interrupted: "Agent task interrupted",
      recovery_required: "Agent task requires recovery",
    };
  const label = labels[run.status];

  return (
    <div
      className={`agent-message-run-status min-w-0 max-w-full flex-wrap ${run.status === "running" ? "agent-message-run-status--running" : ""}`}
      role={run.status === "failed" || run.error ? "alert" : "status"}
      aria-live="polite"
    >
      <span className="agent-message-run-status__dot" aria-hidden="true" />
      <span>{label}</span>
      {run.error ? <span className="w-full min-w-0 whitespace-pre-wrap break-words text-error">{run.error}</span> : null}
      {isRunning && elapsed >= 15 ? (
        <time className="agent-message-run-status__time">
          {elapsedLabel}
        </time>
      ) : null}
    </div>
  );
}

export function groupAgentMessages(
  messages: AgentMessageView[],
): AgentMessageGroup[] {
  const groups: AgentMessageGroup[] = [];
  const pendingByCallId = new Map<string, AgentMessageGroup>();

  for (const message of messages) {
    if (message.role === "tool") {
      const callId = agentToolResultCallId(message);
      if (callId) {
        const group = pendingByCallId.get(callId);
        if (group) {
          group.toolResults.push(message);
          pendingByCallId.delete(callId);
          continue;
        }
      }
    }

    const group: AgentMessageGroup = { message, toolResults: [] };
    groups.push(group);
    for (const callId of agentToolCallIds(message)) {
      pendingByCallId.set(callId, group);
    }
  }

  return groups;
}
