import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { getUsageTrends, type DailyUsagePoint } from "../../../api/admin";
import { BookIconButton } from "../../../components/ui/BookButton";
import { useAdminSectionLoader } from "./useAdminSectionLoader";

export const TrendsSection: React.FC<{ callerUserId: number }> = ({ callerUserId }) => {
  const [trends, setTrends] = useState<DailyUsagePoint[]>([]);
  const [days, setDays] = useState(30);
  const [loadedKey, setLoadedKey] = useState<string | null>(null);
  const activeKey = `${callerUserId}:${days}`;
  const activeKeyRef = useRef(activeKey);
  const mountedRef = useRef(true);
  const loadSeqRef = useRef(0);

  activeKeyRef.current = activeKey;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const isCurrentLoad = useCallback((requestKey: string, seq: number) => {
    return mountedRef.current && activeKeyRef.current === requestKey && loadSeqRef.current === seq;
  }, []);

  const loadTrends = useCallback(async () => {
    const requestCallerUserId = callerUserId;
    const requestDays = days;
    const requestKey = `${requestCallerUserId}:${requestDays}`;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    try {
      const nextTrends = await getUsageTrends(requestCallerUserId, requestDays);
      if (!isCurrentLoad(requestKey, requestSeq)) return;
      setTrends(nextTrends);
      setLoadedKey(requestKey);
    } catch (err) {
      if (!isCurrentLoad(requestKey, requestSeq)) return;
      throw err;
    }
  }, [callerUserId, days, isCurrentLoad]);
  const { loading, load } = useAdminSectionLoader(loadTrends, "加载失败");

  const displayTrends = loadedKey === activeKey ? trends : [];
  const displayLoading = loading || loadedKey !== activeKey;
  const maxCalls = displayTrends.reduce((max, trend) => Math.max(max, trend.total_calls), 0);
  const callScaleMax = maxCalls > 0 ? maxCalls : 1;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-2">
        <h4 className="text-sm font-serif text-book-text-main">最近 {days} 天用量</h4>
        <div className="flex items-center gap-2 text-xs">
          <label className="text-book-text-muted">天数</label>
          <select
            value={days}
            onChange={(e) => setDays(Number(e.target.value))}
            className="pyqt-input !py-1 !px-2 w-20"
          >
            <option value={7}>7</option>
            <option value={14}>14</option>
            <option value={30}>30</option>
            <option value={90}>90</option>
          </select>
          <BookIconButton label="刷新" onClick={load} variant="secondary" size="sm" disabled={loading}>
            <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
          </BookIconButton>
        </div>
      </div>
      {displayLoading ? (
        <div className="flex items-center justify-center gap-2 py-4 text-xs text-book-text-muted pyqt-card">
          <Loader2 size={14} className="animate-spin" />
          加载用量趋势…
        </div>
      ) : displayTrends.length === 0 ? (
        <div className="text-xs text-book-text-muted py-4 text-center pyqt-card">
          无数据
        </div>
      ) : (
        <div className="pyqt-card p-3">
          <ul className="space-y-1">
            {displayTrends.map((t) => (
              <li key={t.date} className="flex items-center gap-2 text-xs">
                <span className="font-mono text-[10px] text-book-text-muted shrink-0 w-20">
                  {t.date}
                </span>
                <div className="flex-1 h-3 bg-book-bg-tertiary rounded overflow-hidden">
                  <div
                    className="h-full bg-book-primary"
                    style={{ width: `${(t.total_calls / callScaleMax) * 100}%` }}
                  />
                </div>
                <span className="text-book-text-sub shrink-0 w-12 text-right">
                  {t.total_calls}
                </span>
                <span className="text-book-text-muted shrink-0 w-16 text-right">
                  {t.total_tokens.toLocaleString()} tok
                </span>
                <span className="text-success shrink-0 w-8 text-right">
                  ✓{t.success_count}
                </span>
                <span className="text-error shrink-0 w-8 text-right">
                  ✗{t.fail_count}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};
