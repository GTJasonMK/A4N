import React from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { I18nProvider } from "../../src/i18n";
import { useAuthStore } from "../../src/store/auth";
import { TasksSection } from "../../src/pages/settings/admin/TasksSection";
import type { TaskQueueSummary } from "../../src/api/generated/host";
import { writeCurrentAuth } from "../../src/api/authStorage";
import "../../src/index.css";

declare global {
  interface Window {
    __adminPendingTasks: TaskQueueSummary[];
    measureAdminPendingTaskList: () => Promise<{
      initialTaskCount: number;
      finalTaskCount: number;
      roundP50Ms: number[];
      roundP95Ms: number[];
      medianP50Ms: number;
      medianP95Ms: number;
      resultSha256: string;
    }>;
  }
}

const taskCount = 1000;
const ignoredPayload = { text: "等待队列中的完整任务参数。".repeat(128) };
const ignoredProgress = { phase: "queued", ratio: 0 };

function task(index: number): TaskQueueSummary {
  return {
    id: `task-${index + 1}`,
    task_type: "llm_call",
    priority: index === 0 ? "high" : ["high", "medium", "low"][index % 3],
    status: "pending",
    project_id: index % 4 === 0 ? 1 : null,
    root_project_id: null,
    source_snapshot_id: null,
    task_scope: "chapter",
    payload: ignoredPayload,
    progress: ignoredProgress,
    error_message: null,
    dag_id: null,
    created_at: new Date(Date.UTC(2026, 0, 1, 0, 0, index)).toISOString(),
    started_at: null,
    completed_at: null,
  };
}

const pendingTasks = (firstTaskPriority: string): TaskQueueSummary[] =>
  Array.from({ length: taskCount }, (_, index) => ({
    ...task(index),
    priority: index === 0 ? firstTaskPriority : ["high", "medium", "low"][index % 3],
    payload: { text: `task ${index + 1} ${"队列任务参数。".repeat(64)}` },
    progress: { phase: "queued", ratio: 0 },
  }));

const adminUser = {
    id: 1,
    username: "admin",
    email: "admin@example.test",
    role: "admin",
    is_active: true,
    page_cli_enabled: false,
    avatar_url: null,
    created_at: null,
    updated_at: null,
    last_login_at: null,
    oauth_provider: null,
    oauth_subject: null,
  };

Object.assign(window, {
  __adminPendingTasks: pendingTasks("high"),
});

const nativeSetInterval = window.setInterval.bind(window);
const nativeClearInterval = window.clearInterval.bind(window);
let refreshPendingTasks: (() => void) | null = null;
const performanceIntervalId = -7319;
window.setInterval = ((callback: TimerHandler, delay?: number, ...args: unknown[]) => {
  if (delay === 5000 && typeof callback === "function") {
    refreshPendingTasks = () => callback(...args);
    return performanceIntervalId;
  }
  return nativeSetInterval(callback, delay, ...args);
}) as typeof window.setInterval;
window.clearInterval = ((timerId: number) => {
  if (timerId === performanceIntervalId) return;
  nativeClearInterval(timerId);
}) as typeof window.clearInterval;

const root = createRoot(document.getElementById("root")!);

const nativeFetch = window.fetch.bind(window);
window.fetch = async (input, init) => {
  const url = new URL(typeof input === "string" ? input : input.url, location.href);
  if (url.pathname.endsWith("/commands/list_pending_tasks")
    || url.pathname.endsWith("/commands/list_pending_task_index")) {
    return Response.json(window.__adminPendingTasks);
  }
  return nativeFetch(input, init);
};

void (async () => {
  await writeCurrentAuth({
    user: adminUser,
    tokens: {
      session_id: "admin-pending-task-fixture",
      access_token: "synthetic-token",
      refresh_token: "synthetic-refresh",
      token_type: "Bearer",
      expires_in: 3600,
    },
  });
  useAuthStore.setState({ initialized: true, user: adminUser });
  flushSync(() => root.render(
    <I18nProvider initialLocale="zh-CN">
      <TasksSection />
    </I18nProvider>,
  ));
})();

function percentile(values: number[], fraction: number): number {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1)];
}

function updatePendingTasks(firstTaskPriority: string): Promise<number> {
  const nextTasks = pendingTasks(firstTaskPriority);
  return new Promise((resolve, reject) => {
    const rootElement = document.getElementById("root")!;
    const startedAt = performance.now();
    const taskText = () => rootElement.querySelector('[title="task-1"]')?.closest("li")?.textContent ?? "";
    const observer = new MutationObserver(() => {
      if (!taskText().includes(firstTaskPriority === "low" ? "低优先级" : "高优先级")) return;
      observer.disconnect();
      window.clearTimeout(timeout);
      resolve(performance.now() - startedAt);
    });
    const timeout = window.setTimeout(() => {
      observer.disconnect();
      reject(new Error(`First pending task did not update to ${firstTaskPriority}: ${taskText()}`));
    }, 5000);
    observer.observe(rootElement, { characterData: true, childList: true, subtree: true });
    Object.assign(window, { __adminPendingTasks: nextTasks });
    if (!refreshPendingTasks) {
      observer.disconnect();
      window.clearTimeout(timeout);
      reject(new Error("Admin pending task refresh interval was not installed"));
      return;
    }
    refreshPendingTasks();
  });
}

async function measureAdminPendingTaskList() {
  let updates = 0;
  const roundP50Ms: number[] = [];
  const roundP95Ms: number[] = [];
  for (let round = 0; round < 3; round += 1) {
    for (let warmup = 0; warmup < 2; warmup += 1) {
      updates += 1;
      await updatePendingTasks(updates % 2 === 1 ? "low" : "high");
    }
    const samples: number[] = [];
    for (let sample = 0; sample < 15; sample += 1) {
      updates += 1;
      samples.push(await updatePendingTasks(updates % 2 === 1 ? "low" : "high"));
    }
    roundP50Ms.push(percentile(samples, 0.5));
    roundP95Ms.push(percentile(samples, 0.95));
  }
  const markup = document.querySelector("#root ul")?.innerHTML ?? "";
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(markup));
  const resultSha256 = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
  return {
    initialTaskCount: taskCount,
    finalTaskCount: document.querySelectorAll("#root ul > li").length,
    finalTaskText: document.querySelector('#root [title="task-1"]')?.closest("li")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
    roundP50Ms,
    roundP95Ms,
    medianP50Ms: percentile(roundP50Ms, 0.5),
    medianP95Ms: percentile(roundP95Ms, 0.5),
    resultSha256,
  };
}

Object.assign(window, {
  measureAdminPendingTaskList,
  performanceReady: true,
});
