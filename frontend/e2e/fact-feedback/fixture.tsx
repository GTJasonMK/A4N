import React from "react";
import { createRoot } from "react-dom/client";
import { FactFeedbackPanel } from "../../src/pages/novel-detail/FactFeedbackPanel";
import { I18nProvider } from "../../src/i18n";
import { ProgressTaskProvider } from "../../src/hooks/useProgressTask";
import { ToastContainer } from "../../src/components/feedback/Toast";
import { initializePortableStorage } from "../../src/utils/portableStorage";
import type { ProjectFactFeedbackSummary } from "../../src/api/factFeedback";
import "../../src/index.css";

const summary: ProjectFactFeedbackSummary = {
  project_id: 1, interval_chapters: 5, pending_chapters: [11, 12], consolidated_chapters: [6, 7, 8, 9, 10],
  legacy_chapters: [1, 2], last_consolidated_at: "2026-09-25T12:00:00Z", workflow_run_id: null,
  workflow_status: null, workflow_kind: null, can_resume: false, error: null,
  changes: [
    { id: 1, material_kind: "world_setting", target_id: 1, field_path: "/technology_or_magic/costs", before: "尚未确认施法代价", after: "每次施法都会永久失去一段记忆", reason: "第十二章首次明确了术法的代价，补充能力体系。", evidence: [{ chapter_number: 12, quote: "灯火重新亮起时，她已记不起母亲的声音。老师说，这是每一次施法都要付出的代价。" }], status: "applied", created_at: "2026-09-25T12:00:00Z" },
    { id: 2, material_kind: "planning", target_id: null, field_path: "", before: "主角依靠不断施法找到母亲", after: "增加一次必须保留记忆、放弃施法的选择", reason: "新确认的代价会影响后续冲突；保留为规划建议。", evidence: [{ chapter_number: 12, quote: "她把法印收回袖中，第一次没有回应门外的呼救。" }], status: "proposal", created_at: "2026-09-25T12:00:00Z" },
  ],
};
const calls: Array<{ command: string; args: any }> = [];
const factFeedbackFixture = {
  calls,
  summary,
  deferPreview: false,
  pendingPreviewResponses: [] as Array<(response: Response) => void>,
};
const nativeFetch = window.fetch.bind(window);
window.fetch = async (url, options) => {
  const path = String(url);
  if (!path.includes("/api/commands/")) return nativeFetch(url, options);
  const command = path.split("/").pop()!;
  const args = JSON.parse(String(options?.body ?? "{}"));
  calls.push({ command, args });
  if (command === "preview_project_fact_feedback_request" && factFeedbackFixture.deferPreview) {
    return await new Promise<Response>((resolve) => factFeedbackFixture.pendingPreviewResponses.push(resolve));
  }
  let result: unknown;
  if (command === "get_project_fact_feedback") result = { ...summary, project_id: args.projectId };
  else if (command === "update_project") {
    summary.interval_chapters = args.input.fact_feedback_interval_chapters;
    result = { id: 1, fact_feedback_interval_chapters: summary.interval_chapters };
  } else if (command === "run_project_fact_feedback") {
    await new Promise((resolve) => setTimeout(resolve, 120));
    summary.consolidated_chapters.push(...summary.pending_chapters);
    summary.pending_chapters = [];
    result = summary;
  } else if (command === "get_task_progress_events") result = { events: [], stream_text: null };
  else if (command === "preview_project_fact_feedback_request") result = { title: "正文事实归纳请求", workflow: "project_fact_consolidation", steps: [], notes: [], execution_location: "server" };
  else throw new Error(`Unexpected fixture command: ${command}`);
  return new Response(JSON.stringify(result), { status: 200, headers: { "Content-Type": "application/json" } });
};
Object.assign(window, { factFeedbackFixture });
localStorage.setItem("a4n:current_user_id", "1");
localStorage.setItem("a4n:current_access_token", "fixture-only-token");
localStorage.setItem("a4n:current_session_id", "fixture-session");
await initializePortableStorage();
createRoot(document.getElementById("root")!).render(
  <I18nProvider initialLocale="zh-CN"><ProgressTaskProvider>
    <main className="a4n-app-shell min-h-dvh bg-book-bg px-3 py-5 text-book-text-main sm:p-6">
      <div className="mx-auto max-w-4xl space-y-4"><h1 className="font-serif text-xl">《记忆之城》· 创作资料</h1>
        <FactFeedbackPanel projectId={1} canEdit />
      </div>
    </main><ToastContainer />
  </ProgressTaskProvider></I18nProvider>,
);
