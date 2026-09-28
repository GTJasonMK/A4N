import React from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { WritingDesk } from "../../src/pages/WritingDesk";
import { ChaptersTab } from "../../src/pages/novel-detail/ChaptersTab";
import { StatusBarProvider } from "../../src/components/ui/StatusBar";
import { ProgressTaskProvider } from "../../src/hooks/useProgressTask";
import { ConfirmDialogHost } from "../../src/components/feedback/ConfirmDialog";
import { PromptDialogHost } from "../../src/components/feedback/PromptDialog";
import { ToastContainer, useToast } from "../../src/components/feedback/Toast";
import { I18nProvider } from "../../src/i18n";
import { writeCurrentTokens } from "../../src/api/authStorage";
import { writeWritingDraft, getWritingDraftKey } from "../../src/utils/writingDraft";
import "../../src/index.css";

const root = createRoot(document.getElementById("root")!);
let version = 0;
function mount(chapter = 1, view = "desk") {
  const key = ++version;
  root.render(<I18nProvider initialLocale="zh-CN" key={key}>
    <MemoryRouter initialEntries={[`/write/1?chapter=${chapter}`]}>
      <StatusBarProvider>
        <ProgressTaskProvider>
        {view === "desk" ? <Routes><Route path="/write/:id" element={<WritingDesk />} /></Routes>
          : <ChaptersTab projectId={1} canEditChapters={false} />}
        <ConfirmDialogHost /><PromptDialogHost /><ToastContainer />
        </ProgressTaskProvider>
      </StatusBarProvider>
    </MemoryRouter>
  </I18nProvider>);
}

void (async () => {
  await writeCurrentTokens({ session_id: "chapter-load-fixture", access_token: "fixture",
    refresh_token: "fixture-refresh", token_type: "Bearer", expires_in: 3600 });
  localStorage.setItem("a4n:current_user_id", "1");
  Object.assign(window, {
    chapterLoad: {
      mount,
      draft: (chapter: number, body: string) => writeWritingDraft(getWritingDraftKey("1", chapter), body),
      toasts: () => useToast.getState().toasts,
    },
  });
})();
