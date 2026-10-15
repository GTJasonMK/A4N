import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { Copy, RefreshCw } from "lucide-react";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { useToast } from "../../../components/feedback/Toast";
import { useDirty } from "../../../components/navigation/dirtyRegistry";
import { extractErrorMessage } from "../../../api";
import { getLogDir, getLogLevel, readRecentLogs, setLogLevel } from "../../../api/logs";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { logger } from "../../../utils/logger";
import { useAuthStore } from "../../../store/auth";
import { GenerationCallAnalyticsPanel } from "./GenerationCallAnalyticsPanel";
import { ModelUsageCostPanel } from "../../novel-detail/ModelUsageCostPanel";
import { logLineClassName, logPathText, type LogReadResult } from "./logUtils";
import { formatNumber, getActiveLocale, translateMessage, useI18n } from "../../../i18n";

export const LogsTab: React.FC = () => {
  const { addToast } = useToast();
  const { t, locale } = useI18n();
  const user = useAuthStore((state) => state.user);
  const [activeTab, setActiveTab] = useState<"analytics" | "logs">("analytics");
  const [result, setResult] = useState<LogReadResult | null>(null);
  const [resultKey, setResultKey] = useState<string | null>(null);
  const [logDir, setLogDir] = useState<string>("");
  const [loadError, setLoadError] = useState<{ reason: unknown } | null>(null);
  const [loadErrorKey, setLoadErrorKey] = useState<string | null>(null);
  const [logDirError, setLogDirError] = useState<{ reason: unknown } | null>(null);
  const [logDirErrorKey, setLogDirErrorKey] = useState<string | null>(null);
  const [lines, setLines] = useState(500);
  const [daysAgo, setDaysAgo] = useState(0);
  const [filterInput, setFilterInput] = useState("");
  const [appliedFilter, setAppliedFilter] = useState<string | null>(null);
  const [filterLoading, setFilterLoading] = useState(true);
  const [filterLoadError, setFilterLoadError] = useState<{ reason: unknown } | null>(null);
  const [loading, setLoading] = useState(true);
  const [applyingFilter, setApplyingFilter] = useState(false);
  const [copied, setCopied] = useState(false);
  const requestKey = `${lines}:${daysAgo}`;
  const callerUserId = user?.id ?? null;
  const canReadDiagnostics = user?.role === "admin";
  const mountedRef = useRef(true);
  const activeRequestKeyRef = useRef(requestKey);
  const refreshInFlightKeyRef = useRef<string | null>(null);
  const refreshSeqRef = useRef(0);
  const filterSeqRef = useRef(0);
  const applyFilterSeqRef = useRef(0);
  const applyingFilterRef = useRef(false);
  const copySeqRef = useRef(0);
  const copiedTimerRef = useRef<number | null>(null);

  activeRequestKeyRef.current = requestKey;
  const filterDraftDirty = appliedFilter == null
    ? filterInput.trim().length > 0
    : filterInput !== appliedFilter;
  useDirty("settings-log-level-draft", filterDraftDirty, () => {
    setFilterInput(appliedFilter ?? "");
  }, () => !applyingFilter);
  useDirty("settings-log-level-operation", applyingFilter, undefined, () => !applyingFilter);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
      filterSeqRef.current += 1;
      refreshInFlightKeyRef.current = null;
      applyFilterSeqRef.current += 1;
      applyingFilterRef.current = false;
      copySeqRef.current += 1;
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
      }
    };
  }, []);

  const isCurrentRefresh = useCallback((key: string, seq: number) => {
    return (
      mountedRef.current &&
      activeRequestKeyRef.current === key &&
      refreshSeqRef.current === seq
    );
  }, []);

  const loadCurrentFilter = useCallback(async () => {
    if (!canReadDiagnostics || callerUserId == null) {
      setFilterInput("");
      setAppliedFilter(null);
      setFilterLoadError(null);
      setFilterLoading(false);
      return;
    }
    const requestSeq = filterSeqRef.current + 1;
    filterSeqRef.current = requestSeq;
    setFilterLoading(true);
    setFilterLoadError(null);
    try {
      const currentFilter = await getLogLevel();
      if (!mountedRef.current || filterSeqRef.current !== requestSeq) return;
      setFilterInput(currentFilter);
      setAppliedFilter(currentFilter);
    } catch (error) {
      if (!mountedRef.current || filterSeqRef.current !== requestSeq) return;
      setFilterLoadError({ reason: error });
    } finally {
      if (mountedRef.current && filterSeqRef.current === requestSeq) setFilterLoading(false);
    }
  }, [callerUserId, canReadDiagnostics]);

  useEffect(() => {
    void loadCurrentFilter();
  }, [loadCurrentFilter]);

  const refresh = useCallback(async () => {
    if (!canReadDiagnostics || callerUserId == null) {
      refreshInFlightKeyRef.current = null;
      setResult(null);
      setResultKey(null);
      setLogDir("");
      setLoadError(null);
      setLoadErrorKey(null);
      setLogDirError(null);
      setLogDirErrorKey(null);
      setLoading(false);
      return;
    }
    const key = requestKey;
    if (refreshInFlightKeyRef.current === key) return;
    refreshInFlightKeyRef.current = key;
    const requestLines = lines;
    const requestDaysAgo = daysAgo;
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    setLoading(true);
    setLoadError(null);
    setLoadErrorKey(null);
    setLogDir("");
    setLogDirError(null);
    setLogDirErrorKey(null);
    try {
      const res = await readRecentLogs({
        lines: requestLines,
        days_ago: requestDaysAgo,
      });
      if (!isCurrentRefresh(key, requestSeq)) return;
      setResult(res);
      setResultKey(key);
      setLoadError(null);
      try {
        const dir = await getLogDir();
        if (!isCurrentRefresh(key, requestSeq)) return;
        setLogDir(dir);
      } catch (err) {
        if (!isCurrentRefresh(key, requestSeq)) return;
        setLogDirError({ reason: err });
        setLogDirErrorKey(key);
      }
    } catch (err) {
      if (!isCurrentRefresh(key, requestSeq)) return;
      const message = extractErrorMessage(err, translateMessage(getActiveLocale(), "settingsLogs.readFailed"));
      setLoadError({ reason: err });
      setLoadErrorKey(key);
      addToast(message, "error");
      logger.scoped("LogsTab").error("refresh failed", { err });
    } finally {
      if (
        refreshInFlightKeyRef.current === key &&
        refreshSeqRef.current === requestSeq
      ) {
        refreshInFlightKeyRef.current = null;
      }
      if (isCurrentRefresh(key, requestSeq)) setLoading(false);
    }
  }, [
    addToast,
    callerUserId,
    canReadDiagnostics,
    daysAgo,
    isCurrentRefresh,
    lines,
    requestKey,
  ]);

  const visibleResult = resultKey === requestKey ? result : null;
  const currentLoadError = loadErrorKey === requestKey && loadError
    ? extractErrorMessage(loadError.reason, t("settingsLogs.readFailed"))
    : null;
  const currentLogDirError = logDirErrorKey === requestKey && logDirError
    ? extractErrorMessage(logDirError.reason, t("settingsLogs.pathUnavailable"))
    : null;
  const displayLoading = loading || (resultKey !== requestKey && currentLoadError == null);

  useEffect(() => {
    setResult(null);
    setLoadError(null);
    setLoadErrorKey(null);
    setLogDirError(null);
    setLogDirErrorKey(null);
    void refresh();
  }, [refresh]);

  const handleApplyFilter = async () => {
    if (!canReadDiagnostics || callerUserId == null) {
      addToast(t("settingsLogs.adminOnlyAction"), "error");
      return;
    }
    if (filterLoading) return;
    if (applyingFilterRef.current) {
      addToast(t("settingsLogs.levelApplying"), "error");
      return;
    }
    const requestFilter = filterInput.trim();
    if (!requestFilter) {
      addToast(t("settingsLogs.filterRequired"), "error");
      return;
    }
    const requestSeq = applyFilterSeqRef.current + 1;
    applyFilterSeqRef.current = requestSeq;
    applyingFilterRef.current = true;
    setApplyingFilter(true);
    try {
      await setLogLevel({ filter: requestFilter });
      if (!mountedRef.current || applyFilterSeqRef.current !== requestSeq)
        return;
      setAppliedFilter(requestFilter);
      setFilterLoadError(null);
      addToast(t("settingsLogs.levelUpdated", { filter: requestFilter }), "success");
    } catch (err) {
      if (!mountedRef.current || applyFilterSeqRef.current !== requestSeq)
        return;
      addToast(extractErrorMessage(err, t("settingsLogs.applyFailed")), "error");
    } finally {
      if (mountedRef.current && applyFilterSeqRef.current === requestSeq) {
        applyingFilterRef.current = false;
        setApplyingFilter(false);
      }
    }
  };

  const handleCopyPath = async () => {
    const requestLogDir = logDir;
    if (!requestLogDir) return;
    const requestSeq = copySeqRef.current + 1;
    copySeqRef.current = requestSeq;
    try {
      await copyTextToClipboard(requestLogDir);
      if (!mountedRef.current || copySeqRef.current !== requestSeq) return;
      setCopied(true);
      if (copiedTimerRef.current != null) {
        window.clearTimeout(copiedTimerRef.current);
      }
      copiedTimerRef.current = window.setTimeout(() => {
        if (copySeqRef.current === requestSeq) setCopied(false);
      }, 1500);
    } catch {
      if (!mountedRef.current || copySeqRef.current !== requestSeq) return;
      addToast(t("settingsLogs.copyFailed"), "error");
    }
  };

  if (!canReadDiagnostics) {
    return (
      <div className="border-l-2 border-book-border/70 py-2 pl-4 text-sm text-book-text-muted">
        {t("settingsLogs.adminOnly")}
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full gap-3 min-h-0">
      <div className="shrink-0">
        <SegmentedControl
          ariaLabel={t("settingsLogs.diagnosticView")}
          value={activeTab}
          onChange={setActiveTab}
          options={[
            { id: "analytics" as const, label: t("settingsLogs.analyticsTab") },
            { id: "logs" as const, label: t("settingsLogs.logsTab") },
          ]}
        />
      </div>

      {activeTab === "analytics" ? (
        <div
          id="diagnostics-panel-analytics"
          className="custom-scrollbar min-h-0 overflow-visible md:flex-1 md:overflow-y-auto md:pr-1"
          role="tabpanel"
          aria-label={t("settingsLogs.analyticsTab")}
        >
          <GenerationCallAnalyticsPanel />
          <div className="mt-4 border-t border-book-border/40 pt-4">
            <ModelUsageCostPanel projectId={null} />
          </div>
        </div>
      ) : (
        <div
          id="diagnostics-panel-logs"
          className="flex min-h-0 flex-1 flex-col gap-3"
          role="tabpanel"
          aria-label={t("settingsLogs.logsTab")}
        >
          <div className="flex min-h-8 flex-wrap items-center gap-2 text-xs">
            <span className="basis-full text-book-text-muted sm:basis-auto">{t("settingsLogs.readRange")}</span>
            <ThemedSelect
              id="log-lines"
              aria-label={t("settingsLogs.logLines")}
              value={lines}
              onChange={(selectedValue) => setLines(Number(selectedValue))}
              className="pyqt-input w-44 px-2 py-1"
            >
              <option value={100}>{t("settingsLogs.recentLines", { count: "100" })}</option>
              <option value={500}>{t("settingsLogs.recentLines", { count: "500" })}</option>
              <option value={2000}>{t("settingsLogs.recentLines", { count: "2,000" })}</option>
              <option value={10000}>{t("settingsLogs.recentLines", { count: "10,000" })}</option>
            </ThemedSelect>
            <ThemedSelect
              id="log-days"
              aria-label={t("settingsLogs.logDate")}
              value={daysAgo}
              onChange={(selectedValue) => setDaysAgo(Number(selectedValue))}
              className="pyqt-input w-40 px-2 py-1"
            >
              <option value={0}>{t("settingsLogs.today")}</option>
              <option value={1}>{t("settingsLogs.yesterday")}</option>
              <option value={2}>{t("settingsLogs.daysAgo", { count: 2 })}</option>
              <option value={3}>{t("settingsLogs.daysAgo", { count: 3 })}</option>
              <option value={7}>{t("settingsLogs.daysAgo", { count: 7 })}</option>
            </ThemedSelect>
            {visibleResult ? (
              <span className="text-book-text-muted sm:ml-auto">
                {t("settingsLogs.lines", { returned: formatNumber(visibleResult.returned_lines, locale), total: formatNumber(visibleResult.total_lines, locale) })}
              </span>
            ) : (
              <span className="flex-1" />
            )}
            <BookIconButton label={t("settingsLogs.refresh")} size="sm" onClick={refresh} disabled={loading}>
              <RefreshCw
                size={14}
                className={loading ? "animate-spin" : undefined}
              />
            </BookIconButton>
          </div>

          {currentLoadError && visibleResult ? (
            <p role="alert" className="text-xs text-error">{currentLoadError}</p>
          ) : null}

          <details className="group border-y border-book-border/35 text-xs">
            <summary className="w-fit cursor-pointer py-2 text-book-text-muted transition-colors hover:text-book-text-main">
              {t("settingsLogs.level")}
            </summary>
            <div className="flex flex-wrap items-center gap-2 pb-2">
              {filterLoading ? (
                <span role="status" className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
                  <LoadingSpinner size="sm" /> {t("settingsLogs.loadingLevel")}
                </span>
              ) : null}
              {filterLoadError ? (
                <div role="alert" className="flex w-full flex-wrap items-center gap-2 text-small text-error">
                  <span className="min-w-0 flex-1">
                    {extractErrorMessage(filterLoadError.reason, t("settingsLogs.filterLoadFailed"))}
                  </span>
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={() => void loadCurrentFilter()}
                    disabled={filterLoading || filterDraftDirty}
                  >
                    {t("common.retry")}
                  </BookButton>
                </div>
              ) : null}
              <input
                aria-label={t("settingsLogs.filterLevel")}
                value={filterInput}
                onChange={(e) => setFilterInput(e.target.value)}
                placeholder={t("settingsLogs.filterPlaceholder")}
                className="pyqt-input w-full text-xs sm:w-72"
                disabled={filterLoading || applyingFilter}
              />
              <BookButton
                variant="secondary"
                size="sm"
                onClick={handleApplyFilter}
                disabled={filterLoading || applyingFilter}
              >
                {filterLoading
                  ? t("settingsLogs.loadingLevel")
                  : applyingFilter
                    ? t("settingsLogs.applying")
                    : t("settingsLogs.apply")}
              </BookButton>
            </div>
          </details>

          <details className="text-xs text-book-text-muted">
            <summary className="w-fit cursor-pointer transition-colors hover:text-book-text-main">
              {t("settingsLogs.location")}
            </summary>
            {currentLogDirError ? (
              <p role="alert" className="mt-2 text-error">
                {currentLogDirError}
              </p>
            ) : null}
            <div className="mt-2 flex flex-wrap items-center gap-2 break-all border-l border-book-border/40 pl-2.5">
              <span className="font-mono">
                {logPathText(visibleResult?.file_path, logDir, t("settingsLogs.pathUnavailable"))}
              </span>
              <BookIconButton
                label={t("settingsLogs.copyPath")}
                variant="ghost"
                size="sm"
                onClick={handleCopyPath}
                disabled={!logDir}
              >
                <Copy size={12} />
              </BookIconButton>
              {copied && <span className="text-success">{t("settingsLogs.copied")}</span>}
            </div>
          </details>

          <div className="a4n-flat-surface flex-1 min-h-0 p-3 overflow-y-auto custom-scrollbar font-mono text-small leading-relaxed bg-book-bg-tertiary/40">
            {!visibleResult ? (
              displayLoading ? (
                <div className="text-book-text-muted">{t("settingsLogs.loading")}</div>
              ) : currentLoadError ? (
                <div className="text-error whitespace-pre-wrap break-words">
                  {currentLoadError}
                </div>
              ) : (
                <div className="text-book-text-muted">{t("settingsLogs.noData")}</div>
              )
            ) : visibleResult.lines.length === 0 ? (
              <div className="text-book-text-muted italic">{t("settingsLogs.empty")}</div>
            ) : (
              <pre className="whitespace-pre-wrap break-words">
                {visibleResult.lines.map((line, index) => (
                  <div key={index} className={logLineClassName(line)}>
                    {line}
                  </div>
                ))}
              </pre>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
