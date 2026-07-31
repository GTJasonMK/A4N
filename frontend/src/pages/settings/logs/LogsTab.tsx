import React, { useCallback, useEffect, useRef, useState } from "react";
import { Copy, RefreshCw } from "lucide-react";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import { getLogDir, readRecentLogs, setLogLevel } from "../../../api/logs";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { logger } from "../../../utils/logger";
import { useAuthStore } from "../../../store/auth";
import { GenerationCallAnalyticsPanel } from "./GenerationCallAnalyticsPanel";
import { logLineClassName, logPathText, type LogReadResult } from "./logUtils";

export const LogsTab: React.FC = () => {
  const { addToast } = useToast();
  const user = useAuthStore((state) => state.user);
  const [activeTab, setActiveTab] = useState<"analytics" | "logs">("analytics");
  const [result, setResult] = useState<LogReadResult | null>(null);
  const [logDir, setLogDir] = useState<string>("");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [lines, setLines] = useState(500);
  const [daysAgo, setDaysAgo] = useState(0);
  const [filterInput, setFilterInput] = useState("info,a4n=debug");
  const [loading, setLoading] = useState(false);
  const [applyingFilter, setApplyingFilter] = useState(false);
  const [copied, setCopied] = useState(false);
  const requestKey = `${lines}:${daysAgo}`;
  const callerUserId = user?.id ?? null;
  const canReadDiagnostics = user?.role === "admin";
  const mountedRef = useRef(true);
  const activeRequestKeyRef = useRef(requestKey);
  const refreshInFlightKeyRef = useRef<string | null>(null);
  const refreshSeqRef = useRef(0);
  const applyFilterSeqRef = useRef(0);
  const applyingFilterRef = useRef(false);
  const copySeqRef = useRef(0);
  const copiedTimerRef = useRef<number | null>(null);

  activeRequestKeyRef.current = requestKey;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
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

  const refresh = useCallback(async () => {
    if (!canReadDiagnostics || callerUserId == null) {
      refreshInFlightKeyRef.current = null;
      setResult(null);
      setLogDir("");
      setLoadError(null);
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
    try {
      const res = await readRecentLogs(callerUserId, {
        lines: requestLines,
        days_ago: requestDaysAgo,
      });
      if (!isCurrentRefresh(key, requestSeq)) return;
      setResult(res);
      setLoadError(null);
      const dir = await getLogDir(callerUserId);
      if (!isCurrentRefresh(key, requestSeq)) return;
      setLogDir(dir);
    } catch (err) {
      if (!isCurrentRefresh(key, requestSeq)) return;
      const message = extractErrorMessage(err, "读取日志失败");
      setLoadError(message);
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

  useEffect(() => {
    setResult(null);
    setLoadError(null);
    void refresh();
  }, [refresh]);

  const handleApplyFilter = async () => {
    if (!canReadDiagnostics || callerUserId == null) {
      addToast("服务器诊断数据仅管理员可操作", "error");
      return;
    }
    if (applyingFilterRef.current) {
      addToast("日志级别正在更新，请等待完成", "error");
      return;
    }
    const requestFilter = filterInput.trim();
    if (!requestFilter) {
      addToast("请输入过滤表达式", "error");
      return;
    }
    const requestSeq = applyFilterSeqRef.current + 1;
    applyFilterSeqRef.current = requestSeq;
    applyingFilterRef.current = true;
    setApplyingFilter(true);
    try {
      await setLogLevel(callerUserId, { filter: requestFilter });
      if (!mountedRef.current || applyFilterSeqRef.current !== requestSeq)
        return;
      addToast(`日志级别已更新：${requestFilter}`, "success");
    } catch (err) {
      if (!mountedRef.current || applyFilterSeqRef.current !== requestSeq)
        return;
      addToast(extractErrorMessage(err, "应用级别失败"), "error");
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
      addToast("复制失败：请手动选取路径", "error");
    }
  };

  if (!canReadDiagnostics) {
    return (
      <div className="border-l-2 border-book-border/70 py-2 pl-4 text-sm text-book-text-muted">
        服务器诊断仅管理员可查看。
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full gap-3 min-h-0">
      <div className="shrink-0">
        <SegmentedControl
          ariaLabel="诊断视图"
          value={activeTab}
          onChange={setActiveTab}
          options={[
            { id: "analytics" as const, label: "生成调用分析" },
            { id: "logs" as const, label: "运行日志" },
          ]}
        />
      </div>

      {activeTab === "analytics" ? (
        <div
          id="diagnostics-panel-analytics"
          className="custom-scrollbar min-h-0 overflow-visible md:flex-1 md:overflow-y-auto md:pr-1"
          role="tabpanel"
          aria-label="生成调用分析"
        >
          <GenerationCallAnalyticsPanel />
        </div>
      ) : (
        <div
          id="diagnostics-panel-logs"
          className="flex min-h-0 flex-1 flex-col gap-3"
          role="tabpanel"
          aria-label="运行日志"
        >
          <div className="flex min-h-8 flex-wrap items-center gap-2 text-xs">
            <span className="text-book-text-muted">读取范围</span>
            <select
              id="log-lines"
              aria-label="日志行数"
              value={lines}
              onChange={(e) => setLines(Number(e.target.value))}
              className="pyqt-input w-28 px-2 py-1"
            >
              <option value={100}>最近 100 行</option>
              <option value={500}>最近 500 行</option>
              <option value={2000}>最近 2,000 行</option>
              <option value={10000}>最近 10,000 行</option>
            </select>
            <select
              id="log-days"
              aria-label="日志日期"
              value={daysAgo}
              onChange={(e) => setDaysAgo(Number(e.target.value))}
              className="pyqt-input w-24 px-2 py-1"
            >
              <option value={0}>今天</option>
              <option value={1}>昨天</option>
              <option value={2}>2 天前</option>
              <option value={3}>3 天前</option>
              <option value={7}>7 天前</option>
            </select>
            {result ? (
              <span className="text-book-text-muted sm:ml-auto">
                已显示 {result.returned_lines.toLocaleString()} / {result.total_lines.toLocaleString()} 行
              </span>
            ) : (
              <span className="flex-1" />
            )}
            <BookIconButton label="刷新日志" size="sm" onClick={refresh} disabled={loading}>
              <RefreshCw
                size={14}
                className={loading ? "animate-spin" : undefined}
              />
            </BookIconButton>
          </div>

          <details className="group border-y border-book-border/35 text-xs">
            <summary className="w-fit cursor-pointer py-2 text-book-text-muted transition-colors hover:text-book-text-main">
              日志级别
            </summary>
            <div className="flex flex-wrap items-center gap-2 pb-2">
              <input
                aria-label="日志过滤级别"
                value={filterInput}
                onChange={(e) => setFilterInput(e.target.value)}
                placeholder="过滤：info / debug / a4n=trace,warn"
                className="pyqt-input w-full text-xs sm:w-72"
              />
              <BookButton
                variant="secondary"
                size="sm"
                onClick={handleApplyFilter}
                disabled={applyingFilter}
              >
                {applyingFilter ? "应用中…" : "应用级别"}
              </BookButton>
            </div>
          </details>

          <details className="text-xs text-book-text-muted">
            <summary className="w-fit cursor-pointer transition-colors hover:text-book-text-main">
              日志位置
            </summary>
            <div className="mt-2 flex flex-wrap items-center gap-2 break-all border-l border-book-border/40 pl-2.5">
              <span className="font-mono">
                {logPathText(result?.file_path, logDir)}
              </span>
              <BookIconButton
                label="复制路径"
                variant="ghost"
                size="sm"
                onClick={handleCopyPath}
                disabled={!logDir}
              >
                <Copy size={12} />
              </BookIconButton>
              {copied && <span className="text-success">已复制</span>}
            </div>
          </details>

          <div className="a4n-flat-surface flex-1 min-h-0 p-3 overflow-y-auto custom-scrollbar font-mono text-small leading-relaxed bg-book-bg-tertiary/40">
            {!result ? (
              loading ? (
                <div className="text-book-text-muted">加载中…</div>
              ) : loadError ? (
                <div className="text-error whitespace-pre-wrap break-words">
                  {loadError}
                </div>
              ) : (
                <div className="text-book-text-muted">暂无日志数据</div>
              )
            ) : result.lines.length === 0 ? (
              <div className="text-book-text-muted italic">（暂无日志）</div>
            ) : (
              <pre className="whitespace-pre-wrap break-words">
                {result.lines.map((line, index) => (
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
