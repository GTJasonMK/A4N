import React, { useMemo, useState } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { AgentMessageList } from "../../src/components/agent/AgentMessageList";
import type { AgentMessageView } from "../../src/api/agent";
import type { Locale } from "../../src/i18n";
import "../../src/index.css";

const messageCount = 1000;
const messages: AgentMessageView[] = Array.from({ length: messageCount }, (_, index) => ({
  content: index % 2 === 0 ? `用户消息 ${index}` : `助手消息 ${index}`,
  created_at: "2026-01-01T00:00:00.000Z",
  id: `message-${index}`,
  metadata: {},
  role: index % 2 === 0 ? "user" : "assistant",
  run_id: null,
  source_id: null,
  source_kind: "agent",
  status: "complete",
  thread_id: "thread-performance",
  turn_id: null,
  updated_at: "2026-01-01T00:00:00.000Z",
}));

let bump: (() => void) | null = null;
let reset: (() => void) | null = null;

function Harness() {
  const [currentMessages, setCurrentMessages] = useState(messages);
  const pendingMessages = useMemo<AgentMessageView[]>(() => [], []);
  const streamedMessageIds = useMemo(() => new Set(messages.map(({ id }) => id)), []);
  bump = () => setCurrentMessages((current) => {
    const next = current.slice();
    const lastIndex = next.length - 1;
    const last = next[lastIndex];
    next[lastIndex] = { ...last, content: `${last.content}x` };
    return next;
  });
  reset = () => setCurrentMessages(messages);
  return (
    <AgentMessageList
      messages={currentMessages}
      locale={"zh-CN" as Locale}
      streamedMessageIds={streamedMessageIds}
      pendingMessages={pendingMessages}
    />
  );
}

const root = createRoot(document.getElementById("root")!);
flushSync(() => root.render(<Harness />));

function verifyToolResultUpdate() {
  const container = document.createElement("div");
  const checkRoot = createRoot(container);
  const call: AgentMessageView = {
    ...messages[1],
    id: "tool-call-check",
    metadata: {
      tool_calls: [{ id: "call-check", name: "list_projects", arguments: {} }],
    },
  };
  const result: AgentMessageView = {
    ...messages[2],
    id: "tool-result-check",
    role: "tool",
    content: JSON.stringify({ projects: [{ id: 1 }] }),
    metadata: {
      tool_call_id: "call-check",
      result: { projects: [{ id: 1 }] },
    },
  };

  try {
    flushSync(() => checkRoot.render(<AgentMessageList messages={[call]} locale="zh-CN" />));
    if (!container.textContent?.includes("等待结果")) {
      throw new Error("the tool call did not render its waiting state");
    }
    flushSync(() => checkRoot.render(<AgentMessageList messages={[call, result]} locale="zh-CN" />));
    if (!container.textContent?.includes("返回 projects 1 项")) {
      throw new Error("the paired tool result did not update the existing row");
    }
  } finally {
    checkRoot.unmount();
  }
  return true;
}

async function measureAgentMessages() {
  const samples: number[] = [];
  const warmupUntil = performance.now() + 250;
  const expectedLastMessage = `${messages[messageCount - 1].content}${"x".repeat(80)}`;
  const updateBatch = () => {
    const started = performance.now();
    for (let update = 0; update < 80; update += 1) {
      flushSync(() => bump?.());
    }
    return performance.now() - started;
  };
  const verifyLastMessage = () => {
    const rows = document.querySelectorAll("[data-agent-message-role]");
    if (!rows.item(messageCount - 1)?.textContent?.includes(expectedLastMessage)) {
      throw new Error("the active message did not render its latest content");
    }
  };
  for (let warmups = 0; warmups < 3 || performance.now() < warmupUntil; warmups += 1) {
    updateBatch();
    verifyLastMessage();
    flushSync(() => reset?.());
  }
  for (let sample = 0; sample < 9; sample += 1) {
    samples.push(updateBatch());
    verifyLastMessage();
    flushSync(() => reset?.());
  }
  const rows = document.querySelectorAll("[data-agent-message-role]");
  if (rows.length !== messageCount) throw new Error(`message rows changed: ${rows.length}`);
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(document.getElementById("root")!.innerHTML),
  );
  const htmlSha256 = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
  const sorted = [...samples].sort((a, b) => a - b);
  return {
    messageCount,
    updates: 80,
    toolResultUpdateVerified: verifyToolResultUpdate(),
    samplesMs: samples,
    medianMs: sorted[4],
    renderedRows: rows.length,
    htmlSha256,
  };
}

Object.assign(window, { measureAgentMessages, performanceReady: true });
