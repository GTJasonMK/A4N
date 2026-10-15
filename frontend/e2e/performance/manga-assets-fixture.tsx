import React from "react";
import { createRoot } from "react-dom/client";
import { MangaTab } from "../../src/pages/novel-detail/MangaTab";
import { ConfirmDialogHost } from "../../src/components/feedback/ConfirmDialog";
import { PromptDialogHost } from "../../src/components/feedback/PromptDialog";
import { ToastContainer } from "../../src/components/feedback/Toast";
import { StatusBarProvider } from "../../src/components/ui/StatusBar";
import { ProgressTaskProvider } from "../../src/hooks/useProgressTask";
import { I18nProvider } from "../../src/i18n";
import { writeCurrentTokens } from "../../src/api/authStorage";
import "../../src/index.css";

const root = createRoot(document.getElementById("root")!);

void (async () => {
  await writeCurrentTokens({
    session_id: "manga-assets-fixture",
    access_token: "fixture",
    refresh_token: "fixture-refresh",
    token_type: "Bearer",
    expires_in: 3600,
  });
  localStorage.setItem("a4n:current_user_id", "1");
  root.render(
    <I18nProvider initialLocale="zh-CN">
      <StatusBarProvider>
        <ProgressTaskProvider>
          <MangaTab projectId={1} />
          <ConfirmDialogHost />
          <PromptDialogHost />
          <ToastContainer />
        </ProgressTaskProvider>
      </StatusBarProvider>
    </I18nProvider>,
  );
})();
