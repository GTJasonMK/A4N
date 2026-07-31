import React, { useCallback, useState } from "react";
import { RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { getUsageTrends, type DailyUsagePoint } from "../../../api/admin";
import { BookIconButton } from "../../../components/ui/BookButton";
import { useAdminSectionLoader } from "./useAdminSectionLoader";

export const TrendsSection: React.FC<{ callerUserId: number }> = ({ callerUserId }) => {
  const [trends, setTrends] = useState<DailyUsagePoint[]>([]);
  const [days, setDays] = useState(30);
  const [loadedKey, setLoadedKey] = useState<string | null>(null);
  const activeKey = `${callerUserId}:${days}`;
  const loadTrends = useCallback(
    () => getUsageTrends(callerUserId, days),
    [callerUserId, days],
  );
  const applyTrends = useCallback(
    (nextTrends: DailyUsagePoint[]) => {
      setTrends(nextTrends);
      setLoadedKey(activeKey);
    },
    [activeKey],
  );
  const { loading, load } = useAdminSectionLoader(
    activeKey,
    loadTrends,
    applyTrends,
    "加载失败",
  );

  const displayTrends = loadedKey === activeKey ? trends : [];
  const displayLoading = loading || loadedKey !== activeKey;
  const maxCalls = displayTrends.reduce((max, trend) => Math.max(max, trend.total_calls), 0);
  const callScaleMax = maxCalls > 0 ? maxCalls : 1;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex min-h-8 items-center gap-2 text-xs">
        <label className="inline-flex items-center gap-2 text-book-text-muted">
          统计范围
          <select
            value={days}
            onChange={(e) => setDays(Number(e.target.value))}
            className="pyqt-input w-24 !px-2 !py-1"
            aria-label="用量趋势统计范围"
          >
            <option value={7}>最近 7 天</option>
            <option value={14}>最近 14 天</option>
            <option value={30}>最近 30 天</option>
            <option value={90}>最近 90 天</option>
          </select>
        </label>
        <span className="flex-1" />
        <BookIconButton label="刷新用量趋势" onClick={load} size="sm" disabled={loading}>
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      {displayLoading ? (
        <div className="a4n-empty-state a4n-empty-state--compact inline-flex items-center justify-center gap-2 text-xs">
          <LoadingSpinner size="sm" />
          加载用量趋势…
        </div>
      ) : displayTrends.length === 0 ? (
        <div className="a4n-empty-state a4n-empty-state--compact text-xs">
          无数据
        </div>
      ) : (
        <div className="a4n-flat-surface overflow-hidden">
          <ul className="divide-y divide-book-border/30">
            {displayTrends.map((t) => (
              <li key={t.date} className="grid gap-1.5 px-3 py-2.5 text-xs sm:grid-cols-[82px_minmax(0,1fr)_auto] sm:items-center sm:gap-3">
                <span className="font-mono text-small text-book-text-muted">{t.date}</span>
                <div className="min-w-0">
                  <div className="h-2 overflow-hidden rounded-full bg-book-bg-tertiary">
                    <div
                      className="h-full rounded-full bg-book-primary transition-[width]"
                      style={{ width: `${(t.total_calls / callScaleMax) * 100}%` }}
                    />
                  </div>
                  <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-caption text-book-text-muted">
                    <span>令牌 {t.total_tokens.toLocaleString()}</span>
                    {t.success_count > 0 ? <span className="text-success">成功 {t.success_count}</span> : null}
                    {t.fail_count > 0 ? <span className="text-error">失败 {t.fail_count}</span> : null}
                  </div>
                </div>
                <span className="text-book-text-sub sm:text-right">
                  <b className="font-mono text-sm font-semibold text-book-text-main">{t.total_calls}</b>
                  <span className="ml-1 text-caption text-book-text-muted">次调用</span>
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};
