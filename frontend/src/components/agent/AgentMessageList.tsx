import { useEffect, useState } from "react";
import type { AgentMessageView, AgentRunView } from "../../api/agent";
import {
  AgentMessageRow,
  agentToolCallIds,
  agentToolResultCallId,
} from "./AgentMessageRow";
import type { Locale } from "../../i18n";

interface AgentMessageListProps {
  messages: AgentMessageView[];
  locale: Locale;
  streamedMessageIds?: ReadonlySet<string>;
  pendingMessages?: AgentMessageView[];
  activeRun?: Pick<AgentRunView, "status" | "created_at" | "started_at"> | null;
  compactAssistantMessages?: boolean;
}

export interface PendingAgentMessage {
  message: AgentMessageView;
  acceptedMessageId: string | null;
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

export function AgentMessageList({
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
}

function AgentRunStatus({
  run,
  locale,
}: {
  run: Pick<AgentRunView, "status" | "created_at" | "started_at">;
  locale: Locale;
}) {
  const [now, setNow] = useState(() => Date.now());
  const isRunning = run.status === "running";

  useEffect(() => {
    if (!isRunning) return undefined;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [isRunning]);

  const startedAt = run.started_at ? Date.parse(run.started_at) : NaN;
  const elapsed = Number.isFinite(startedAt)
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
  const labels: Record<string, string> = locale === "zh-CN" ? {
      queued: "正在排队",
      running: "Agent 正在处理任务",
      waiting: "等待你的确认",
      pausing: "正在暂停任务",
      paused: "任务已暂停，可继续",
      cancelling: "正在取消任务",
    } : {
      queued: "Queued",
      running: "Agent is working",
      waiting: "Waiting for your confirmation",
      pausing: "Pausing task",
      paused: "Task paused",
      cancelling: "Cancelling task",
    };
  const label = labels[run.status] ?? (locale === "zh-CN" ? "Agent 已完成" : "Agent complete");

  return (
    <div
      className={`agent-message-run-status ${run.status === "running" ? "agent-message-run-status--running" : ""}`}
      role="status"
      aria-live="polite"
    >
      <span className="agent-message-run-status__dot" aria-hidden="true" />
      <span>{label}</span>
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
