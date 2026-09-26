import { createRoot } from "react-dom/client";
import { I18nProvider } from "../../src/i18n";
import { ShootingScriptTab } from "../../src/pages/novel-detail/ShootingScriptTab";
import { ConfirmDialogHost } from "../../src/components/feedback/ConfirmDialog";
import { ToastContainer } from "../../src/components/feedback/Toast";
import { writeFixtureAuth } from "../fixtureAuth";
import { ProgressTaskProvider } from "../../src/hooks/useProgressTask";
import type { ApiErrorResponse } from "../../src/api/generated/host";
import type { GenerationTaskEvent, GenerationTaskHandle } from "../../src/api/generated/generationContracts";
import "../../src/index.css";

// Responses are captured by the real CLI regression; this fixture checks UI requests and rendering.
const captured = JSON.parse(sessionStorage.getItem("shot-sync-fixture")!);
const mode = new URLSearchParams(location.search).get("mode") ?? "manual";
const partial = mode.startsWith("partial");
const scenario = captured[partial ? "partial" : mode === "readonly" ? "manual" : mode];
let current = partial ? null : structuredClone(scenario.before);
let progress = partial ? structuredClone(scenario.before) : null;
const task: GenerationTaskHandle = { task_id: crypto.randomUUID(), attempt: 1, event_schema_version: 1, status: "reserved" };
const calls: Array<{ command: string; input: unknown }> = [];
const errors: string[] = [];
window.addEventListener("error", (event) => errors.push(event.error?.stack ?? event.message));
window.addEventListener("unhandledrejection", (event) => errors.push(String(event.reason)));
Object.assign(window, { shotSyncCalls: calls, shotSyncErrors: errors });
await writeFixtureAuth("shot-sync-browser-fixture");
const originalFetch = window.fetch.bind(window);
const errorResponse = (message: string, status: number) => new Response(JSON.stringify({
  code: "invalid_state", message, retryable: false, trace_id: null, details: {},
} satisfies ApiErrorResponse), { status, headers: { "content-type": "application/json" } });
window.fetch = async (resource, options) => {
  const url = new URL(typeof resource === "string" ? resource : resource instanceof URL ? resource.href : resource.url, location.href);
  if (url.pathname === "/api/generation-tasks/active") return Response.json([]);
  if (url.pathname === "/api/generation-tasks/shooting-script") {
    const input = JSON.parse(String(options?.body ?? "{}"));
    calls.push({ command: "start_chapter_shooting_script_generation", input });
    current = structuredClone(scenario.after);
    progress = null;
    return Response.json(task);
  }
  if (url.pathname === `/api/generation-tasks/${task.task_id}/events`) {
    const envelope = { schema_version: 1, task_id: task.task_id, attempt: task.attempt, emitted_at_ms: Date.now() };
    const events: GenerationTaskEvent[] = [
      { ...envelope, sequence: 1, event: { kind: "started", data: {
        task_kind: "chapter_shooting_script", delivery_mode: "stream", root_task_id: task.task_id,
        project_id: captured.chapter.project_id, target: { kind: "chapter_shooting_script", id: `chapter:${captured.chapter.id}` },
      } } },
      { ...envelope, sequence: 2, event: { kind: "completed", data: {
        final_text_hashes: [], result_ref: { kind: "chapter_shooting_script", id: String(current.record.id), revision: String(current.record.revision) },
      } } },
    ];
    return new Response(events.map((event) => `id: ${event.attempt}:${event.sequence}\nevent: generation_task_event\ndata: ${JSON.stringify(event)}\n\n`).join(""),
      { headers: { "content-type": "text/event-stream" } });
  }
  if (url.pathname.startsWith("/api/generation-tasks/")) return errorResponse(`Unexpected task request: ${url.pathname}`, 500);
  if (!url.pathname.includes("/commands/")) return originalFetch(resource, options);
  const command = decodeURIComponent(url.pathname.split("/").at(-1)!);
  const { input } = JSON.parse(String(options?.body ?? "{}"));
  calls.push({ command, input });
  let result: unknown;
  switch (command) {
    case "get_chapter_visual_plan":
      result = { source: { chapter_number: 1, title: captured.chapter.title, content: captured.chapter.content, content_sha256: scenario.plan.record.source_content_sha256 }, starter_plan: scenario.plan.plan, draft: null, confirmed: scenario.plan };
      break;
    case "get_chapter_shooting_script": result = current; break;
    case "get_chapter_shooting_script_unit_progress": result = progress; break;
    case "edit_chapter_shooting_script_progress":
      if (mode === "partial-failure") return errorResponse("进度保存冲突，请重新加载", 409);
      if (JSON.stringify(input.expected_script) !== JSON.stringify(progress.script)) {
        return errorResponse("进度版本不一致", 409);
      }
      progress.script = structuredClone(input.script);
      result = { progress, invalidated_project_ids: [] };
      break;
    case "edit_chapter_shooting_script":
      current = structuredClone(scenario.after);
      result = { shooting_script: current, invalidated_project_ids: [] };
      break;
    default: return errorResponse(`Unexpected fixture command: ${command}`, 500);
  }
  return new Response(JSON.stringify(result), { status: 200, headers: { "content-type": "application/json" } });
};

createRoot(document.getElementById("root")!).render(
  <I18nProvider initialLocale="zh-CN">
    <ProgressTaskProvider>
    <main className="h-dvh min-w-0 overflow-y-auto bg-book-bg p-3 sm:p-6">
      <ShootingScriptTab projectId={captured.chapter.project_id} chapter={captured.chapter} canEdit={mode !== "readonly"} />
    </main>
    <ConfirmDialogHost /><ToastContainer />
    </ProgressTaskProvider>
  </I18nProvider>,
);
