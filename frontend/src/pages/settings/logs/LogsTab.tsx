import React, { useCallback, useEffect, useRef, useState } from "react";
import { Copy, RefreshCw } from "lucide-react";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
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
      <div className="pyqt-card p-3 text-sm text-book-text-muted">
        服务器诊断仅管理员可查看。
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full gap-3 min-h-0">
      <div className="flex items-center gap-1 border-b border-book-border">
        <button
          type="button"
          onClick={() => setActiveTab("analytics")}
          className={`px-3 py-2 text-sm border-b-2 transition-colors ${
            activeTab === "analytics"
              ? "border-info text-book-text-main font-medium"
              : "border-transparent text-book-text-muted hover:text-book-text-main"
          }`}
        >
          生成调用分析
        </button>
        <button
          type="button"
          onClick={() => setActiveTab("logs")}
          className={`px-3 py-2 text-sm border-b-2 transition-colors ${
            activeTab === "logs"
              ? "border-info text-book-text-main font-medium"
              : "border-transparent text-book-text-muted hover:text-book-text-main"
          }`}
        >
          运行日志
        </button>
      </div>

      {activeTab === "analytics" ? (
        <div className="flex-1 min-h-0 overflow-y-auto custom-scrollbar pr-1">
          <GenerationCallAnalyticsPanel />
        </div>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-2 text-xs">
            <span className="text-book-text-muted">行数</span>
            <select
              value={lines}
              onChange={(e) => setLines(Number(e.target.value))}
              className="pyqt-input py-1 px-2 w-24"
            >
              <option value={100}>100</option>
              <option value={500}>500</option>
              <option value={2000}>2000</option>
              <option value={10000}>10000</option>
            </select>
            <span className="text-book-text-muted ml-2">日期</span>
            <select
              value={daysAgo}
              onChange={(e) => setDaysAgo(Number(e.target.value))}
              className="pyqt-input py-1 px-2 w-28"
            >
              <option value={0}>今天</option>
              <option value={1}>昨天</option>
              <option value={2}>2 天前</option>
              <option value={3}>3 天前</option>
              <option value={7}>7 天前</option>
            </select>
            <BookIconButton
              label="刷新"
              variant="secondary"
              size="sm"
              onClick={refresh}
              disabled={loading}
            >
              <RefreshCw
                size={14}
                className={loading ? "animate-spin" : undefined}
              />
            </BookIconButton>
            <div className="flex-1" />
            <BookInput
              value={filterInput}
              onChange={(e) => setFilterInput(e.target.value)}
              placeholder="过滤：info / debug / a4n=trace,warn"
              className="!w-72 text-xs"
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

          <div className="text-xs text-book-text-muted flex items-center gap-2 break-all">
            <span className="font-mono">
              {logPathText(result?.file_path, logDir)}
            </span>
            {result && (
              <span className="pyqt-pill">
                {result.returned_lines}/{result.total_lines} 行
              </span>
            )}
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

          <div className="flex-1 min-h-0 pyqt-card p-3 overflow-y-auto custom-scrollbar font-mono text-[11px] leading-relaxed bg-book-bg-tertiary/40">
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
        </>
      )}
    </div>
  );
};
