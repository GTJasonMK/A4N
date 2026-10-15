import React, { useCallback, useEffect, useRef, useState } from "react";
import { Download, RefreshCw, RotateCw } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { dirtyRegistry, useDirty } from "../../../components/navigation/dirtyRegistry";
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
  const addToast = useToast((state) => state.addToast);
  const [update, setUpdate] = useState<AppUpdateInfo | null>(null);
  const [progress, setProgress] = useState<AppUpdateProgress | null>(null);
  const [recovery, setRecovery] = useState<AppUpdateRecoveryState | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [recoveryLoading, setRecoveryLoading] = useState(false);
  const [clearingRecovery, setClearingRecovery] = useState(false);
  const [checkError, setCheckError] = useState<string | null>(null);
  const [recoveryError, setRecoveryError] = useState<string | null>(null);
  const [recoveryClearError, setRecoveryClearError] = useState<string | null>(null);
  const [installError, setInstallError] = useState<{ version: string; message: string } | null>(null);
  const clearingRecoveryRef = useRef(false);
  const mountedRef = useRef(false);

  useDirty(
    "settings-app-update-recovery-clear",
    clearingRecovery,
    undefined,
    () => !clearingRecoveryRef.current,
  );

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  const checkUpdate = useCallback(async (force = false) => {
    setChecking(true);
    setCheckError(null);
    setUpdate(null);
    setInstallError(null);
    try {
      const nextUpdate = await checkForAppUpdate(force);
      setUpdate(nextUpdate);
      setInstallError((current) => current?.version === nextUpdate?.version ? current : null);
    } catch (checkError) {
      setCheckError(checkError instanceof Error ? checkError.message : String(checkError));
    } finally {
      setChecking(false);
    }
  }, []);

  const loadRecoveryState = useCallback(async () => {
    setRecoveryLoading(true);
    setRecoveryError(null);
    setRecovery(null);
    setRecoveryClearError(null);
    try {
      const nextRecovery = await getAppUpdateRecoveryState();
      if (mountedRef.current) setRecovery(nextRecovery);
    } catch (recoveryReadError) {
      if (mountedRef.current) {
        setRecoveryError(recoveryReadError instanceof Error ? recoveryReadError.message : String(recoveryReadError));
      }
    } finally {
      if (mountedRef.current) setRecoveryLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!isDesktopTauriRuntime() || shouldUseHttpBackend()) return;
    void checkUpdate();
    void loadRecoveryState();
  }, [checkUpdate, loadRecoveryState]);

  const install = async () => {
    const targetVersion = update?.version;
    if (!targetVersion) return;
    setInstallError(null);
    try {
      if (dirtyRegistry.hasDirty() && !dirtyRegistry.canDiscardDirty()) {
        throw new Error(t("common.operationInProgress"));
      }

      setInstalling(true);
      setProgress({ stage: "preparing", downloadedBytes: 0, totalBytes: null });
      const preflight = await prepareAppUpdateInstall(targetVersion);
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
      await downloadPreparedAppUpdate(targetVersion, setProgress);
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
      await installDownloadedAppUpdate(targetVersion);
    } catch (installError) {
      const message = installError instanceof Error ? installError.message : String(installError);
      if (mountedRef.current) {
        setInstallError({ version: targetVersion, message });
        setInstalling(false);
        setProgress(null);
      } else {
        addToast(message, "error");
      }
    }
  };

  const dismissRecoveryFailure = async () => {
    if (clearingRecoveryRef.current) return;
    clearingRecoveryRef.current = true;
    setClearingRecovery(true);
    setRecoveryClearError(null);
    try {
      await clearAppUpdateRecoveryState();
      setRecovery(null);
      setRecoveryError(null);
      setRecoveryClearError(null);
    } catch (clearError) {
      setRecoveryClearError(clearError instanceof Error ? clearError.message : String(clearError));
    } finally {
      clearingRecoveryRef.current = false;
      setClearingRecovery(false);
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
            {recoveryClearError ? (
              <p className="text-book-danger">
                {t("settingsAbout.updateRecoveryClearFailed", { error: recoveryClearError })}
              </p>
            ) : null}
            {recovery.backupPath ? (
              <p className="break-all text-book-text-muted">
                {t("settingsAbout.updateBackupRetained", { backupPath: recovery.backupPath })}
              </p>
            ) : null}
            <BookButton
              variant="secondary"
              size="sm"
              disabled={clearingRecovery || recoveryLoading || recoveryError != null}
              aria-busy={clearingRecovery}
              onClick={() => void dismissRecoveryFailure()}
            >
              {clearingRecovery ? (
                <span className="inline-flex items-center gap-1.5">
                  <RotateCw size={13} className="animate-spin" />
                  {t("settingsAbout.dismissingUpdateFailure")}
                </span>
              ) : t("settingsAbout.dismissUpdateFailure")}
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
        ) : !checkError ? (
          <p>{t("settingsAbout.upToDate")}</p>
        ) : null}
        {checkError ? <p role="alert">{t("settingsAbout.appUpdateFailed", { error: checkError })}</p> : null}
        {recoveryError ? (
          <div role="alert" className="mt-2 flex flex-wrap items-center gap-2 text-book-danger">
            <span className="min-w-0 flex-1">{t("settingsAbout.updateRecoveryReadFailed", { error: recoveryError })}</span>
            <BookButton variant="secondary" size="sm" onClick={() => void loadRecoveryState()} disabled={recoveryLoading}>
              <RefreshCw size={13} aria-hidden="true" /> {t("common.retry")}
            </BookButton>
          </div>
        ) : recoveryLoading ? (
          <p role="status" className="mt-2">
            <LoadingSpinner size="sm" text={t("common.loading")} />
          </p>
        ) : null}
        {installError && update?.version === installError.version ? (
          <p className="mt-2 text-book-danger" role="alert">
            {t("settingsAbout.appUpdateFailed", { error: installError.message })}
          </p>
        ) : null}
      </div>
    </section>
  );
};
