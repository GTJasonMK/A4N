import { createRoot } from "react-dom/client";
import { useState } from "react";
import { I18nProvider } from "../../src/i18n";
import { ShootingScriptTab } from "../../src/pages/novel-detail/ShootingScriptTab";
import { ConfirmDialogHost } from "../../src/components/feedback/ConfirmDialog";
import { ToastContainer } from "../../src/components/feedback/Toast";
import { writeFixtureAuth } from "../fixtureAuth";
import { ProgressTaskProvider } from "../../src/hooks/useProgressTask";
import type { ApiErrorResponse } from "../../src/api/generated/host";
import { GENERATION_START_PATHS } from "../../src/api/generated/host";
import type { GenerationTaskEvent, GenerationTaskHandle } from "../../src/api/generated/generationContracts";
import "../../src/index.css";

// Responses are captured by the real CLI regression; this fixture checks UI requests and rendering.
const captured = JSON.parse(sessionStorage.getItem("shot-sync-fixture")!);
const mode = new URLSearchParams(location.search).get("mode") ?? "manual";
const partial = mode.startsWith("partial");
const scenario = captured[partial ? "partial" : ["readonly", "save-race"].includes(mode) ? "manual" : mode];
let current = partial ? null : structuredClone(scenario.before);
let progress = partial ? structuredClone(scenario.before) : null;
const calls: Array<{ command: string; input: unknown }> = [];
const errors: string[] = [];
let generation: { handle: GenerationTaskHandle; events: GenerationTaskEvent[] } | null = null;
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
  if (url.pathname.endsWith("/generation-tasks/active")) return Response.json([]);
  if (generation && url.pathname.endsWith(`/generation-tasks/${generation.handle.task_id}/events`)) {
    return new Response(generation.events.map((event) =>
      `event: generation_task_event\nid: ${event.attempt}:${event.sequence}\ndata: ${JSON.stringify(event)}\n\n`,
    ).join(""), { headers: { "Content-Type": "text/event-stream" } });
  }
  const startPath = GENERATION_START_PATHS.get("start_chapter_shooting_script_generation");
  const starting = startPath != null && url.pathname.endsWith(startPath);
  if (!url.pathname.includes("/commands/") && !starting) {
    if (url.pathname.startsWith("/api/")) return errorResponse(`Unexpected fixture route: ${url.pathname}`, 500);
    return originalFetch(resource, options);
  }
  const command = starting ? "start_chapter_shooting_script_generation" : decodeURIComponent(url.pathname.split("/").at(-1)!);
  const payload = JSON.parse(String(options?.body ?? "{}"));
  const input = starting ? payload : payload.input;
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
    case "start_chapter_shooting_script_generation": {
      current = structuredClone(scenario.after);
      const handle: GenerationTaskHandle = { event_schema_version: 1, task_id: crypto.randomUUID(), attempt: 1, status: "reserved" };
      generation = { handle, events: [
        { schema_version: 1, task_id: handle.task_id, attempt: 1, sequence: 1, emitted_at_ms: Date.now(),
          event: { kind: "started", data: { delivery_mode: "stream", root_task_id: handle.task_id, task_kind: "llm_call" } } },
        { schema_version: 1, task_id: handle.task_id, attempt: 1, sequence: 2, emitted_at_ms: Date.now(),
          event: { kind: "completed", data: { final_text_hashes: [], result_ref: {
            kind: "chapter_shooting_script", id: String(current.record.id), revision: String(current.record.revision),
          } } } },
      ] };
      result = handle;
      break;
    }
    default: return errorResponse(`Unexpected fixture command: ${command}`, 500);
  }
  const response = Response.json(result);
  if (mode.endsWith("-race") && command.startsWith("edit_chapter_shooting_script")) {
    return new Promise<Response>((resolve) => Object.assign(window, { releaseShotSyncSave: () => resolve(response) }));
  }
  return response;
};

function Fixture() {
  const [chapterId, setChapterId] = useState(captured.chapter.id);
  Object.assign(window, { selectShotSyncChapter: setChapterId });
  return <ShootingScriptTab projectId={captured.chapter.project_id} chapter={{ ...captured.chapter, id: chapterId }} canEdit={mode !== "readonly"} />;
}

createRoot(document.getElementById("root")!).render(
  <I18nProvider initialLocale="zh-CN">
    <ProgressTaskProvider>
    <main className="h-dvh min-w-0 overflow-y-auto bg-book-bg p-3 sm:p-6">
      <Fixture />
    </main>
    <ConfirmDialogHost /><ToastContainer />
    </ProgressTaskProvider>
  </I18nProvider>,
);
