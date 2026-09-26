import React, { useCallback, useEffect, useRef, useState } from "react";
import { Copy,
  Database,
  FolderOpen,
  RefreshCw,
  Trash2, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import {
  cleanupOrphanProjectDataDirs,
  getStoragePaths,
  listOrphanProjectDataDirs,
  type OrphanProjectDataDirsSummary,
  type StoragePathsSummary,
} from "../../../api/storagePaths";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { formatNumber, useI18n } from "../../../i18n";

interface StoragePathsPanelProps {
  isAdmin?: boolean;
  className?: string;
}

const formatBytes = (bytes: number, locale: ReturnType<typeof useI18n>["locale"]): string => {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  const displayValue = value >= 10 || unitIndex === 0 ? value : Number(value.toFixed(1));
  return `${formatNumber(displayValue, locale)} ${units[unitIndex]}`;
};

export const StoragePathsPanel: React.FC<StoragePathsPanelProps> = ({
  isAdmin = false,
  className = "mt-5",
}) => {
  const { addToast } = useToast();
  const { t, locale } = useI18n();
  const [paths, setPaths] = useState<StoragePathsSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [orphanSummary, setOrphanSummary] =
    useState<OrphanProjectDataDirsSummary | null>(null);
  const [orphanLoading, setOrphanLoading] = useState(false);
  const [orphanError, setOrphanError] = useState<string | null>(null);
  const [orphanCleaning, setOrphanCleaning] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const refreshSeqRef = useRef(0);
  const refreshInFlightRef = useRef(false);
  const orphanSeqRef = useRef(0);
  const orphanScanningRef = useRef(false);
  const orphanCleaningRef = useRef(false);
  const copySeqRef = useRef(0);
  const copiedTimerRef = useRef<number | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
      refreshInFlightRef.current = false;
      orphanSeqRef.current += 1;
      orphanScanningRef.current = false;
      orphanCleaningRef.current = false;
      copySeqRef.current += 1;
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
      }
    };
  }, []);

  const refresh = useCallback(async () => {
    if (!isAdmin) {
      refreshInFlightRef.current = false;
      setPaths(null);
      setLoadError(null);
      setLoading(false);
      return;
    }
    if (refreshInFlightRef.current) return;
    refreshInFlightRef.current = true;
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    setLoading(true);
    setLoadError(null);
    try {
      const nextPaths = await getStoragePaths();
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      setPaths(nextPaths);
    } catch (err) {
      if (!mountedRef.current || refreshSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, t("settingsAbout.loadStoragePathsFailed"));
      setLoadError(message);
      addToast(message, "error");
    } finally {
      if (mountedRef.current && refreshSeqRef.current === requestSeq) {
        setLoading(false);
        refreshInFlightRef.current = false;
      }
    }
  }, [addToast, isAdmin]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const copyPath = async (key: string, value?: string | null) => {
    if (!value) return;
    const requestSeq = copySeqRef.current + 1;
    copySeqRef.current = requestSeq;
    try {
      await copyTextToClipboard(value);
      if (!mountedRef.current || copySeqRef.current !== requestSeq) return;
      setCopiedKey(key);
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
      }
      copiedTimerRef.current = window.setTimeout(() => {
        if (copySeqRef.current === requestSeq) setCopiedKey(null);
      }, 1500);
    } catch {
      if (!mountedRef.current || copySeqRef.current !== requestSeq) return;
      addToast(t("settingsAbout.copyPathFailed"), "error");
    }
  };

  const scanOrphanProjectDirs = useCallback(async () => {
    if (
      !isAdmin ||
      orphanScanningRef.current ||
      orphanCleaningRef.current
    ) {
      return;
    }
    orphanScanningRef.current = true;
    const requestSeq = orphanSeqRef.current + 1;
    orphanSeqRef.current = requestSeq;
    setOrphanLoading(true);
    setOrphanError(null);
    try {
      const summary = await listOrphanProjectDataDirs();
      if (!mountedRef.current || orphanSeqRef.current !== requestSeq) return;
      setOrphanSummary(summary);
    } catch (err) {
      if (!mountedRef.current || orphanSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, t("settingsAbout.scanOrphansFailed"));
      setOrphanError(message);
      addToast(message, "error");
    } finally {
      if (orphanSeqRef.current === requestSeq) {
        orphanScanningRef.current = false;
      }
      if (mountedRef.current && orphanSeqRef.current === requestSeq) {
        setOrphanLoading(false);
      }
    }
  }, [addToast, isAdmin]);

  const cleanupOrphanProjectDirs = useCallback(async () => {
    if (
      !isAdmin ||
      orphanCleaningRef.current ||
      orphanScanningRef.current
    ) {
      return;
    }
    const summary = orphanSummary;
    if (!summary || summary.total_count <= 0) return;
    orphanCleaningRef.current = true;
    setOrphanCleaning(true);
    const confirmed = await confirmDialog({
      title: t("settingsAbout.cleanupOrphans"),
      message: t("settingsAbout.cleanupOrphansMessage", { count: summary.total_count, size: formatBytes(summary.total_size_bytes, locale) }),
      confirmText: t("common.delete"),
      cancelText: t("common.cancel"),
      dialogType: "danger",
    });
    if (!confirmed) {
      orphanCleaningRef.current = false;
      if (mountedRef.current) setOrphanCleaning(false);
      return;
    }
    const requestSeq = orphanSeqRef.current + 1;
    orphanSeqRef.current = requestSeq;
    setOrphanError(null);
    try {
      const summary = await cleanupOrphanProjectDataDirs();
      if (!mountedRef.current || orphanSeqRef.current !== requestSeq) return;
      setOrphanSummary({ items: [], total_count: 0, total_size_bytes: 0 });
      addToast(
        t("settingsAbout.cleanedOrphans", { count: summary.total_count, size: formatBytes(summary.total_size_bytes, locale) }),
        "success",
      );
    } catch (err) {
      if (!mountedRef.current || orphanSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, t("settingsAbout.cleanupOrphansFailed"));
      setOrphanError(message);
      addToast(message, "error");
    } finally {
      if (orphanSeqRef.current === requestSeq) {
        orphanCleaningRef.current = false;
      }
      if (mountedRef.current && orphanSeqRef.current === requestSeq) {
        setOrphanCleaning(false);
      }
    }
  }, [addToast, isAdmin, orphanSummary]);

  const rows = paths
    ? [
        [t("settingsAbout.storageApplication"), paths.application_home],
        [t("settingsAbout.storageData"), paths.data_dir],
        [t("settingsAbout.storageConfig"), paths.config_file],
        [t("settingsAbout.storageDatabase"), paths.database_url],
        [t("settingsAbout.storageProjects"), paths.projects_dir],
        [t("settingsAbout.storageExports"), paths.exports_dir],
        [t("settingsAbout.storageBackups"), paths.backups_dir],
        [t("settingsAbout.storageLogs"), paths.log_dir],
        [t("settingsAbout.storageWebview"), paths.webview_dir],
        [t("settingsAbout.storageFrontendState"), paths.frontend_state_file],
        [t("settingsAbout.storageCache"), paths.cache_dir],
        [t("settingsAbout.storageEmbedding"), paths.embedding_models_dir],
        [t("settingsAbout.storageTemp"), paths.temp_dir],
        [t("settingsAbout.storagePromptHistory"), paths.prompts_git_dir],
        [t("settingsAbout.storageRagIndex"), paths.qdrant_url || t("settingsAbout.storageUnsetQdrant")],
        [t("settingsAbout.storageRagCollection"), paths.qdrant_collection],
      ]
    : [];

  return (
    <div className={`${className} space-y-3 text-xs`}>
      <section aria-label={t("settingsAbout.storageLocation")} className="a4n-flat-surface space-y-3 p-3">
        <div className="flex items-center gap-2">
          <FolderOpen size={14} className="shrink-0 text-book-primary" />
          <div className="min-w-0 flex-1">
            <h3 className="text-xs font-semibold text-book-text-main">{t("settingsAbout.storageLocation")}</h3>
            <p className="mt-0.5 text-small text-book-text-muted">{t("settingsAbout.storagePathHidden")}</p>
          </div>
          <BookIconButton
            label={t("settingsAbout.refreshPaths")}
            variant="ghost"
            size="sm"
            onClick={refresh}
            disabled={loading}
          >
            <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
          </BookIconButton>
        </div>

        {loadError ? <p role="alert" className="text-error">{loadError}</p> : null}
        {loading && !paths ? (
          <div className="inline-flex items-center gap-1.5 text-book-text-muted">
            <LoadingSpinner size="sm" /> {t("settingsAbout.loading")}
          </div>
        ) : paths ? (
          <details className="border-t border-book-border/35 pt-2">
            <summary className="flex cursor-pointer list-none items-center gap-2 py-1 text-book-text-sub [&::-webkit-details-marker]:hidden">
              <span>{t("settingsAbout.viewStorageLocation")}</span>
              <span className="text-small text-book-text-muted">{t("settingsAbout.storageItemCount", { count: rows.length })}</span>
            </summary>
            <div className="mt-2 divide-y divide-book-border/30 border-y border-book-border/30">
              {rows.map(([label, value]) => (
                <div
                  key={label}
                  className="grid grid-cols-[72px_minmax(0,1fr)_auto] items-center gap-2 py-2"
                >
                  <span className="text-book-text-muted">{label}</span>
                  <span className="break-all font-mono text-small text-book-text-main">
                    {storageValueForDisplay(label, value, t("settingsAbout.storageDatabase"), t)}
                  </span>
                  {label === t("settingsAbout.storageDatabase") ? (
                    <span className="h-8 w-8" aria-hidden="true" />
                  ) : (
                    <BookIconButton
                      label={t("settingsAbout.copyPath", { label })}
                      variant="ghost"
                      size="sm"
                      onClick={() => copyPath(label, value)}
                    >
                      <Copy size={11} />
                    </BookIconButton>
                  )}
                </div>
              ))}
            </div>
            {copiedKey && (
              <div className="mt-2 inline-flex items-center gap-1 text-success">
                <Database size={11} />
                {t("settingsAbout.copiedPath", { label: copiedKey })}
              </div>
            )}
          </details>
        ) : null}
      </section>

      {isAdmin && (
        <section aria-label={t("settingsAbout.storageCleanup")} className="a4n-flat-surface space-y-3 p-3">
          <div className="flex items-center gap-2">
            <Trash2 size={14} className="shrink-0 text-book-primary" />
            <div className="min-w-0 flex-1">
              <h3 className="text-xs font-semibold text-book-text-main">{t("settingsAbout.storageCleanup")}</h3>
              <p className="mt-0.5 text-small text-book-text-muted">
                {t("settingsAbout.storageCleanupHint")}
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2 border-t border-book-border/35 pt-3">
            <div className="min-w-0 flex-1">
              <div className="font-medium text-book-text-main">{t("settingsAbout.orphanProjectDirs")}</div>
              <div className="mt-0.5 text-small text-book-text-muted">
                {orphanSummary
                  ? orphanSummary.total_count > 0
                    ? t("settingsAbout.orphanSummary", { count: orphanSummary.total_count, size: formatBytes(orphanSummary.total_size_bytes, locale) })
                    : t("settingsAbout.noOrphans")
                  : orphanError ? t("settingsAbout.scanOrphansFailed") : t("settingsAbout.notScanned")}
              </div>
            </div>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={scanOrphanProjectDirs}
              disabled={orphanLoading || orphanCleaning}
            >
              {orphanLoading ? <LoadingSpinner size="sm" /> : null}
              {orphanLoading ? t("settingsAbout.scanning") : t("settingsAbout.scan")}
            </BookButton>
            {orphanCleaning ? (
              <span className="inline-flex min-h-8 items-center gap-1 px-2 text-small text-book-text-muted">
                <LoadingSpinner size="sm" />
                {t("settingsAbout.cleaning")}
              </span>
            ) : (orphanSummary?.total_count ?? 0) > 0 ? (
              <Dropdown
                items={[
                  {
                    label: t("settingsAbout.cleanupOrphans"),
                    icon: <Trash2 size={12} />,
                    danger: true,
                    onClick: () => void cleanupOrphanProjectDirs(),
                  },
                ]}
              />
            ) : null}
          </div>

          {orphanError ? <p role="alert" className="text-error">{orphanError}</p> : null}
          {orphanSummary && orphanSummary.items.length > 0 ? (
            <details className="border-t border-book-border/35 pt-2 text-caption text-book-text-muted">
              <summary className="w-fit cursor-pointer hover:text-book-text-main">
                {t("settingsAbout.diagnostic")}
              </summary>
              <div className="mt-1 space-y-1 font-mono">
                {orphanSummary.items.slice(0, 5).map((item) => (
                  <div key={item.project_id} className="break-all">
                    {item.path} · {formatBytes(item.size_bytes, locale)}
                  </div>
                ))}
              </div>
            </details>
          ) : null}
        </section>
      )}
    </div>
  );
};

function storageValueForDisplay(
  label: string,
  value: string,
  databaseLabel: string,
  t: ReturnType<typeof useI18n>["t"],
): string {
  if (label !== databaseLabel) return value;
  try {
    const url = new URL(value);
    const databaseName = url.pathname.replace(/^\//, "");
    const endpoint = `${url.hostname}${url.port ? `:${url.port}` : ""}`;
    return databaseName ? `${endpoint}/${databaseName}` : endpoint || t("settingsAdmin.configured");
  } catch {
    return value ? t("settingsAdmin.configured") : t("settingsAdmin.notConfigured");
}
}
