import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { ConfirmDialogHost } from "../../src/components/feedback/ConfirmDialog";
import { ToastContainer } from "../../src/components/feedback/Toast";
import { dirtyRegistry, useDirty } from "../../src/components/navigation/dirtyRegistry";
import { I18nProvider } from "../../src/i18n";
import { AppUpdatePanel } from "../../src/pages/settings/about/AppUpdatePanel";

const params = new URLSearchParams(window.location.search);
const calls: string[] = [];
const clearResolvers: Array<() => void> = [];
const downloadResolvers: Array<(error?: string) => void> = [];
const recoveryReadResolvers: Array<() => void> = [];
let recoveryPresent = params.get("recovery") === "failed";
let recoveryReadCount = 0;
let recoveryClearCount = 0;
let updateCheckCount = 0;
const audit = { calls, clearResolvers, downloadResolvers, recoveryReadResolvers };

Object.assign(window, {
  __TAURI_INTERNALS__: {
    transformCallback: () => 1,
    unregisterCallback: () => undefined,
    invoke: async (command: string) => {
      calls.push(command);
      if (command === "check_app_update") {
        if (params.get("check") === "error") throw new Error("update feed unavailable");
        if (params.get("check") === "version-change") {
          updateCheckCount += 1;
          return { version: updateCheckCount === 1 ? "2.0.0" : "2.1.0", notes: null, pubDate: null, target: "linux-x86_64" };
        }
        if (params.get("check") === "available") {
          return { version: "2.0.0", notes: null, pubDate: null, target: "linux-x86_64" };
        }
        return null;
      }
      if (command === "get_update_recovery_state") {
        recoveryReadCount += 1;
        if (params.get("recovery") === "error" || (params.get("recovery") === "retry" && recoveryReadCount === 1)) {
          throw new Error("recovery file unavailable");
        }
        if (params.get("recovery") === "retry" && recoveryReadCount === 2) {
          return new Promise<null>((resolve) => recoveryReadResolvers.push(() => resolve(null)));
        }
        return recoveryPresent ? {
          state: "failed",
          backupPath: "/work/data/update-backup.sql",
          targetVersion: "2.0.0",
          previousVersion: "1.9.0",
          pendingMigrations: [],
          failureReason: "installer failed after download",
        } : null;
      }
      if (command === "clear_update_recovery_state") {
        recoveryClearCount += 1;
        if (params.get("clear") === "error-once" && recoveryClearCount === 1) {
          throw new Error("recovery state file is read-only");
        }
        if (params.get("clear") === "deferred") {
          return new Promise<void>((resolve, reject) => {
            clearResolvers.push(() => {
              if (!recoveryPresent) {
                reject(new Error("no update failure record"));
                return;
              }
              recoveryPresent = false;
              resolve();
            });
          });
        }
        if (!recoveryPresent) throw new Error("no update failure record");
        recoveryPresent = false;
        return;
      }
      if (command === "prepare_app_update_install") {
        if (params.get("recovery") === "error") throw new Error("existing update recovery state is unreadable");
        if (params.get("prepare") === "error") throw new Error("active generation task prevents update");
        if (params.get("prepare") === "migration") {
          return {
            backupPath: "/work/data/update-backup.sql",
            migrations: [{ name: "media layout migration", affectedProjects: 1, mediaFilesToCopy: 2 }],
          };
        }
        return { backupPath: null, migrations: [] };
      }
      if (command === "download_app_update") {
        if (params.get("download") === "error") throw new Error("update package unavailable");
        if (params.get("download") === "deferred") {
          return new Promise<void>((resolve, reject) => {
            downloadResolvers.push((error) => {
              if (error) reject(new Error(error));
              else resolve();
            });
          });
        }
        return;
      }
      throw new Error(`Unexpected updater command: ${command}`);
    },
  },
  __appUpdateAudit: audit,
});

function DirtyDraftProbe() {
  const [draft, setDraft] = useState("");
  useDirty("app-update-audit-draft", draft.trim().length > 0, () => setDraft(""));
  return (
    <label>
      <span>未保存的设置草稿</span>
      <input aria-label="应用更新测试草稿" value={draft} onChange={(event) => setDraft(event.target.value)} />
    </label>
  );
}

function AppUpdateAuditHarness() {
  const [panelOpen, setPanelOpen] = useState(true);
  const [navigationBlocked, setNavigationBlocked] = useState(false);
  const leave = () => {
    if (dirtyRegistry.hasDirty()) {
      setNavigationBlocked(true);
      return;
    }
    setPanelOpen(false);
  };
  return (
    <>
      <DirtyDraftProbe />
      <button type="button" aria-label="离开应用更新页" onClick={leave}>
        离开应用更新页
      </button>
      {navigationBlocked ? <p role="alert">操作仍在进行</p> : null}
      {panelOpen ? <AppUpdatePanel /> : <p>已离开应用更新页</p>}
      <ConfirmDialogHost />
      <ToastContainer />
    </>
  );
}

createRoot(document.getElementById("app")!).render(
  <I18nProvider initialLocale="zh-CN">
    <AppUpdateAuditHarness />
  </I18nProvider>,
);
