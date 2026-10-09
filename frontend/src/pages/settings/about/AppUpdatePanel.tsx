import React, { useCallback, useEffect, useState } from "react";
import { Download, RefreshCw, RotateCw } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { dirtyRegistry } from "../../../components/navigation/dirtyRegistry";
import { useI18n } from "../../../i18n";
import {
  checkForAppUpdate,
  clearAppUpdateRecoveryState,
  downloadPreparedAppUpdate,
  getAppUpdateRecoveryState,
  installDownloadedAppUpdate,
  prepareAppUpdateInstall,
  type AppUpdateInfo,
  type AppUpdateProgress,
  type AppUpdateRecoveryState,
} from "../../../api/appUpdate";
import { isDesktopTauriRuntime, shouldUseHttpBackend } from "../../../utils/runtimeMode";

export const AppUpdatePanel: React.FC = () => {
  const { t } = useI18n();
  const [update, setUpdate] = useState<AppUpdateInfo | null>(null);
  const [progress, setProgress] = useState<AppUpdateProgress | null>(null);
  const [recovery, setRecovery] = useState<AppUpdateRecoveryState | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const checkUpdate = useCallback(async (force = false) => {
    setChecking(true);
    setError(null);
    try {
      setUpdate(await checkForAppUpdate(force));
    } catch (checkError) {
      setError(checkError instanceof Error ? checkError.message : String(checkError));
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    if (!isDesktopTauriRuntime() || shouldUseHttpBackend()) return;
    void checkUpdate();
    void getAppUpdateRecoveryState().then(setRecovery).catch((recoveryError) => {
      setError(recoveryError instanceof Error ? recoveryError.message : String(recoveryError));
    });
  }, [checkUpdate]);

  const install = async () => {
    setError(null);
    try {
      if (!update) return;
      if (dirtyRegistry.hasDirty()) {
        if (!dirtyRegistry.canDiscardDirty()) {
          throw new Error(t("common.operationInProgress"));
        }
        const confirmed = await confirmDialog({
          title: t("common.unsavedChangesTitle"),
          message: t("common.unsavedChangesMessage", {
            name: t("settingsAbout.updateTitle"),
            action: t("settingsAbout.installUpdate"),
          }),
          confirmText: t("common.discardDraft"),
          dialogType: "warning",
        });
        if (!confirmed) return;
        if (!await dirtyRegistry.discardDirty()) {
          throw new Error(t("common.operationInProgress"));
        }
      }

      setInstalling(true);
      setProgress({ stage: "preparing", downloadedBytes: 0, totalBytes: null });
      const preflight = await prepareAppUpdateInstall(update.version);
      if (preflight.migrations.length > 0) {
        const migrations = preflight.migrations.map((migration) => {
          return t("settingsAbout.updateMigrationSummary", {
            name: migration.name,
            projects: migration.affectedProjects,
            mediaFiles: migration.mediaFilesToCopy,
          });
        }).join("; ");
        const confirmed = await confirmDialog({
          title: t("settingsAbout.updateTitle"),
          message: t("settingsAbout.updateDataMigrationConfirm", {
            migrations,
            backupPath: preflight.backupPath ?? "",
          }),
          confirmText: t("settingsAbout.installUpdate"),
          dialogType: "warning",
        });
        if (!confirmed) {
          setInstalling(false);
          setProgress(null);
          return;
        }
      }
      await downloadPreparedAppUpdate(update.version, setProgress);
      if (dirtyRegistry.hasDirty()) {
        if (!dirtyRegistry.canDiscardDirty()) {
          throw new Error(t("common.operationInProgress"));
        }
        const confirmed = await confirmDialog({
          title: t("common.unsavedChangesTitle"),
          message: t("common.unsavedChangesMessage", {
            name: t("settingsAbout.updateTitle"),
            action: t("settingsAbout.installUpdate"),
          }),
          confirmText: t("common.discardDraft"),
          dialogType: "warning",
        });
        if (!confirmed) {
          setInstalling(false);
          setProgress(null);
          return;
        }
        if (!await dirtyRegistry.discardDirty()) {
          throw new Error(t("common.operationInProgress"));
        }
      }
      setProgress((current) => current && { ...current, stage: "installing" });
      await installDownloadedAppUpdate(update.version);
    } catch (installError) {
      setError(installError instanceof Error ? installError.message : String(installError));
      setInstalling(false);
      setProgress(null);
    }
  };

  const progressPercent = progress?.totalBytes
    ? Math.min(100, Math.round((progress.downloadedBytes / progress.totalBytes) * 100))
    : null;
  const progressLabel = progress?.stage === "preparing"
    ? t("settingsAbout.preparingUpdate")
    : progress?.stage === "installing"
      ? t("settingsAbout.installingUpdate")
      : t("settingsAbout.downloadingUpdate");

  return (
    <section aria-label={t("settingsAbout.updateTitle")} aria-busy={installing} className="a4n-flat-surface overflow-hidden">
      <div className="flex flex-wrap items-start justify-between gap-3 px-4 py-4">
        <div className="min-w-0">
          <h2 className="font-serif text-base font-semibold text-book-text-main">{t("settingsAbout.updateTitle")}</h2>
          <p className="mt-1 text-small text-book-text-muted">{t("settingsAbout.updateDescription")}</p>
        </div>
        <BookButton
          variant="secondary"
          size="sm"
          disabled={checking || installing || !isDesktopTauriRuntime() || shouldUseHttpBackend()}
          onClick={() => void checkUpdate(true)}
        >
          <RefreshCw size={14} className="mr-1.5 inline" aria-hidden="true" />
          {checking ? t("settingsAbout.checkingForUpdates") : t("settingsAbout.checkForUpdates")}
        </BookButton>
      </div>
      <div className="border-t border-book-border/35 px-4 py-3 text-small text-book-text-sub">
        {!isDesktopTauriRuntime() || shouldUseHttpBackend() ? (
          <p>{t("settingsAbout.desktopOnlyUpdate")}</p>
        ) : recovery ? (
          <div role="alert" className="space-y-2 text-book-warning">
            <p>
              {t("settingsAbout.updateRecoveryFailure", {
                error: recovery.failureReason ?? t("settingsAbout.installingUpdate"),
              })}
            </p>
            {recovery.backupPath ? (
              <p className="break-all text-book-text-muted">
                {t("settingsAbout.updateBackupRetained", { backupPath: recovery.backupPath })}
              </p>
            ) : null}
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => {
                void clearAppUpdateRecoveryState().then(() => setRecovery(null)).catch((clearError) => {
                  setError(clearError instanceof Error ? clearError.message : String(clearError));
                });
              }}
            >
              {t("settingsAbout.dismissUpdateFailure")}
            </BookButton>
          </div>
        ) : update ? (
          <div className="space-y-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span>{t("settingsAbout.updateAvailable", { version: update.version })}</span>
              <BookButton variant="primary" size="sm" disabled={installing} onClick={() => void install()}>
                <Download size={14} className="mr-1.5 inline" aria-hidden="true" />
                {installing ? t("settingsAbout.installingUpdate") : t("settingsAbout.installUpdate")}
              </BookButton>
            </div>
            {update.notes ? <p className="whitespace-pre-wrap text-book-text-muted">{update.notes}</p> : null}
            {installing ? (
              <div className="space-y-1" aria-live="polite">
                <div className="flex items-center gap-2">
                  <RotateCw size={13} className="animate-spin" aria-hidden="true" />
                  <span>{progress?.stage === "downloading" && progressPercent !== null ? `${progressPercent}%` : progressLabel}</span>
                </div>
                <div className="h-1.5 overflow-hidden rounded-full bg-book-bg-tertiary">
                  <div className="h-full bg-book-primary transition-[width]" style={{ width: `${progressPercent ?? 8}%` }} />
                </div>
              </div>
            ) : null}
          </div>
        ) : checking ? (
          <p>{t("settingsAbout.checkingForUpdates")}</p>
        ) : (
          <p>{t("settingsAbout.upToDate")}</p>
        )}
        {error ? <p className="mt-2 text-book-danger" role="alert">{t("settingsAbout.appUpdateFailed", { error })}</p> : null}
      </div>
    </section>
  );
};
