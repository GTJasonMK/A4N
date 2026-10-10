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

function Harness() {
  const [, setRevision] = useState(0);
  const pendingMessages = useMemo<AgentMessageView[]>(() => [], []);
  const streamedMessageIds = useMemo(() => new Set<string>(), []);
  bump = () => setRevision((revision) => revision + 1);
  return (
    <AgentMessageList
      messages={messages}
      locale={"zh-CN" as Locale}
      streamedMessageIds={streamedMessageIds}
      pendingMessages={pendingMessages}
    />
  );
}

const root = createRoot(document.getElementById("root")!);
flushSync(() => root.render(<Harness />));

async function measureAgentMessages() {
  const samples: number[] = [];
  const warmupUntil = performance.now() + 250;
  for (let sample = 0; samples.length < 9; sample += 1) {
    const started = performance.now();
    for (let update = 0; update < 80; update += 1) {
      flushSync(() => bump?.());
    }
    const elapsed = performance.now() - started;
    if (sample >= 3 && started >= warmupUntil) samples.push(elapsed);
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
    samplesMs: samples,
    medianMs: sorted[4],
    renderedRows: rows.length,
    htmlSha256,
  };
}

Object.assign(window, { measureAgentMessages, performanceReady: true });
