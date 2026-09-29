import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useCallback, useState } from "react";
import { RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { getUsageTrends, type DailyUsagePoint } from "../../../api/admin";
import { BookIconButton } from "../../../components/ui/BookButton";
import { useAdminSectionLoader } from "./useAdminSectionLoader";
import { formatNumber, useI18n } from "../../../i18n";

export const TrendsSection: React.FC = () => {
  const { t, locale } = useI18n();
  const [trends, setTrends] = useState<DailyUsagePoint[]>([]);
  const [days, setDays] = useState(30);
  const [loadedKey, setLoadedKey] = useState<string | null>(null);
  const activeKey = String(days);
  const loadTrends = useCallback(
    () => getUsageTrends(days),
    [days],
  );
  const applyTrends = useCallback(
    (nextTrends: DailyUsagePoint[]) => {
      setTrends(nextTrends);
      setLoadedKey(activeKey);
    },
    [activeKey],
  );
  const { loading, error, load } = useAdminSectionLoader(
    activeKey,
    loadTrends,
    applyTrends,
    "settingsAdmin.loadFailed",
  );

  const displayTrends = loadedKey === activeKey ? trends : [];
  const displayLoading = loading || (loadedKey !== activeKey && !error);
  const maxCalls = displayTrends.reduce((max, trend) => Math.max(max, trend.total_calls), 0);
  const callScaleMax = maxCalls > 0 ? maxCalls : 1;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex min-h-8 items-center gap-2 text-xs">
        <label className="inline-flex items-center gap-2 text-book-text-muted">
          {t("settingsAdmin.usageRange")}
          <ThemedSelect
            value={days}
            onChange={(selectedValue) => setDays(Number(selectedValue))}
            className="pyqt-input w-32 !px-2 !py-1"
            aria-label={t("settingsAdmin.usageRange")}
          >
            <option value={7}>{t("settingsAdmin.recentDays", { count: 7 })}</option>
            <option value={14}>{t("settingsAdmin.recentDays", { count: 14 })}</option>
            <option value={30}>{t("settingsAdmin.recentDays", { count: 30 })}</option>
            <option value={90}>{t("settingsAdmin.recentDays", { count: 90 })}</option>
          </ThemedSelect>
        </label>
        <span className="flex-1" />
        <BookIconButton label={t("settingsAdmin.refreshTrends")} onClick={load} size="sm" disabled={loading}>
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      {error ? <p role="alert" className="text-xs text-error">{error}</p> : null}
      {displayLoading ? (
        <div className="a4n-empty-state a4n-empty-state--compact inline-flex items-center justify-center gap-2 text-xs">
          <LoadingSpinner size="sm" />
          {t("settingsAdmin.loadingTrends")}
        </div>
      ) : error && displayTrends.length === 0 ? null : displayTrends.length === 0 ? (
        <div className="a4n-empty-state a4n-empty-state--compact text-xs">
          {t("settingsAdmin.noData")}
        </div>
      ) : (
        <div className="a4n-flat-surface overflow-hidden">
          <ul className="divide-y divide-book-border/30">
            {displayTrends.map((trend) => (
              <li key={trend.date} className="grid gap-1.5 px-3 py-2.5 text-xs sm:grid-cols-[82px_minmax(0,1fr)_auto] sm:items-center sm:gap-3">
                <span className="font-mono text-small text-book-text-muted">{trend.date}</span>
                <div className="min-w-0">
                  <div className="h-2 overflow-hidden rounded-full bg-book-bg-tertiary">
                    <div
                      className="h-full rounded-full bg-book-primary transition-[width]"
                      style={{ width: `${(trend.total_calls / callScaleMax) * 100}%` }}
                    />
                  </div>
                  <div className="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-caption text-book-text-muted">
                    <span>{t("settingsAdmin.tokenCount", { count: formatNumber(trend.total_tokens, locale) })}</span>
                    {trend.success_count > 0 ? <span className="text-success">{t("settingsAdmin.successCount", { count: formatNumber(trend.success_count, locale) })}</span> : null}
                    {trend.fail_count > 0 ? <span className="text-error">{t("settingsAdmin.failureCount", { count: formatNumber(trend.fail_count, locale) })}</span> : null}
                  </div>
                </div>
                <span className="text-book-text-sub sm:text-right">
                  <b className="font-mono text-sm font-semibold text-book-text-main">{trend.total_calls}</b>
                  <span className="ml-1 text-caption text-book-text-muted">{t("settingsAdmin.calls")}</span>
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};
