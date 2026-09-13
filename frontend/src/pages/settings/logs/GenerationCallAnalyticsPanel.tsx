import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { ChevronRight, RefreshCw } from "lucide-react";
import { BookIconButton } from "../../../components/ui/BookButton";
import { extractErrorMessage } from "../../../api";
import {
  getGenerationCallAnalytics,
  type CallKindOverview,
  type DailyCallMetric,
  type GenerationCallAnalytics,
  type RecentLlmCall,
  type RecentMediaCall,
} from "../../../api/generationAnalytics";
import { useAuthStore } from "../../../store/auth";
import { logger } from "../../../utils/logger";
import { formatNumber, useI18n } from "../../../i18n";

const CHART_WIDTH = 720;
const CHART_HEIGHT = 164;

export const GenerationCallAnalyticsPanel: React.FC = () => {
  const { t, locale } = useI18n();
  const user = useAuthStore((state) => state.user);
  const [days, setDays] = useState(30);
  const [data, setData] = useState<GenerationCallAnalytics | null>(null);
  const [loadedKey, setLoadedKey] = useState<string | null>(null);
  const [errorState, setErrorState] = useState<{
    key: string;
    message: string;
  } | null>(null);
  const [loading, setLoading] = useState(false);
  const mountedRef = useRef(true);
  const requestSeqRef = useRef(0);
  const refreshInFlightKeyRef = useRef<string | null>(null);
  const callerUserId = user?.id ?? null;
  const canReadDiagnostics = user?.role === "admin";
  const activeKey = `${callerUserId ?? "none"}:${days}`;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      requestSeqRef.current += 1;
      refreshInFlightKeyRef.current = null;
    };
  }, []);

  const refresh = useCallback(async () => {
    if (!canReadDiagnostics || callerUserId == null) {
      refreshInFlightKeyRef.current = null;
      setData(null);
      setLoadedKey(null);
      setErrorState(null);
      setLoading(false);
      return;
    }
    const requestKey = `${callerUserId}:${days}`;
    if (refreshInFlightKeyRef.current === requestKey) return;
    refreshInFlightKeyRef.current = requestKey;
    const requestSeq = requestSeqRef.current + 1;
    requestSeqRef.current = requestSeq;
    setErrorState(null);
    setLoading(true);
    try {
      const result = await getGenerationCallAnalytics({
        days,
        recent_limit: 40,
      });
      if (!mountedRef.current || requestSeqRef.current !== requestSeq) return;
      setData(result);
      setLoadedKey(requestKey);
      setErrorState(null);
    } catch (err) {
      if (!mountedRef.current || requestSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, t("settingsLogs.analyticsLoadFailed"));
      setData(null);
      setLoadedKey(requestKey);
      setErrorState({ key: requestKey, message });
      logger
        .scoped("GenerationCallAnalyticsPanel")
        .error("refresh failed", { err });
    } finally {
      if (
        refreshInFlightKeyRef.current === requestKey &&
        requestSeqRef.current === requestSeq
      ) {
        refreshInFlightKeyRef.current = null;
      }
      if (mountedRef.current && requestSeqRef.current === requestSeq) {
        setLoading(false);
      }
    }
  }, [callerUserId, canReadDiagnostics, days, t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const displayData = loadedKey === activeKey ? data : null;
  const displayLoading = loading && loadedKey !== activeKey;
  const displayError =
    errorState?.key === activeKey ? errorState.message : null;
  const displayFailed = displayError != null && displayData == null;

  const overviewByKind = useMemo(() => {
    const map = new Map<string, CallKindOverview>();
    for (const item of displayData?.overview ?? []) {
      map.set(item.kind, item);
    }
    return map;
  }, [displayData]);

  const llmOverview = overviewByKind.get("llm");
  const recentLlmDescription = displayLoading
    ? t("settingsLogs.loading")
    : displayFailed
      ? t("settingsLogs.readFailedShort")
      : displayData && displayData.recent_llm_calls.length > 0
        ? t("settingsLogs.recordCount", { count: displayData.recent_llm_calls.length })
        : t("settingsLogs.noRecords");
  const recentMediaDescription = displayLoading
    ? t("settingsLogs.loading")
    : displayFailed
      ? t("settingsLogs.readFailedShort")
      : displayData && displayData.recent_media_calls.length > 0
        ? t("settingsLogs.recordCount", { count: displayData.recent_media_calls.length })
        : t("settingsLogs.noRecords");

  if (!canReadDiagnostics) {
    return (
      <div className="border-l-2 border-book-border/70 py-2 pl-4 text-sm text-book-text-muted">
        {t("settingsLogs.adminOnly")}
      </div>
    );
  }

  return (
    <section className="flex flex-col gap-3">
      <div className="flex min-h-8 flex-wrap items-center gap-2 text-xs">
        <label className="inline-flex items-center gap-2 text-book-text-muted">
          {t("settingsLogs.usageRange")}
          <ThemedSelect
            value={days}
            onChange={(event) => setDays(Number(event.target.value))}
            className="pyqt-input w-28 px-2 py-1"
            aria-label={t("settingsLogs.usageRange")}
          >
            <option value={7}>{t("settingsLogs.recentDays", { count: 7 })}</option>
            <option value={30}>{t("settingsLogs.recentDays", { count: 30 })}</option>
            <option value={90}>{t("settingsLogs.recentDays", { count: 90 })}</option>
            <option value={180}>{t("settingsLogs.recentDays", { count: 180 })}</option>
          </ThemedSelect>
        </label>
        <span className="flex-1" />
        <BookIconButton
          label={t("settingsLogs.refreshAnalytics")}
          size="sm"
          onClick={refresh}
          disabled={loading}
        >
          <RefreshCw
            size={14}
            className={loading ? "animate-spin" : undefined}
          />
        </BookIconButton>
      </div>

      {displayError && !displayLoading && (
        <div className="rounded border border-error/30 bg-error-bg px-3 py-2 text-xs text-error" role="alert">
          {displayError}
        </div>
      )}

      <div className="grid grid-cols-1 items-start gap-2 lg:grid-cols-3">
        {(["llm", "image", "video"] as const).map((kind) => (
          <OverviewRow
            key={kind}
            item={overviewByKind.get(kind)}
            kind={kind}
            loading={displayLoading}
            failed={displayFailed}
            locale={locale}
          />
        ))}
      </div>

      <div className="grid grid-cols-1 items-start gap-2 xl:grid-cols-[minmax(0,1.35fr)_minmax(320px,0.65fr)]">
        <section className="a4n-flat-surface min-w-0 p-3">
          <h3 className="mb-2 text-xs font-medium">{t("settingsLogs.dailyCalls")}</h3>
          {displayData && displayData.daily.length > 0 ? (
            <DailyCallsChart rows={displayData.daily} />
          ) : (
            <EmptyState
              text={
                displayLoading
                  ? t("settingsLogs.loading")
                  : displayFailed
                    ? t("settingsLogs.currentRangeFailed")
                    : t("settingsLogs.noCallRecords")
              }
            />
          )}
        </section>

        <section className="a4n-flat-surface min-w-0 p-3">
          <h3 className="mb-2 text-xs font-medium">{t("settingsLogs.promptCache")}</h3>
          <CacheSummary
            item={llmOverview}
            loading={displayLoading}
            failed={displayFailed}
          />
        </section>
      </div>

      <section aria-label={t("settingsLogs.workflowDetails")} className="space-y-2">
        <div className="flex items-baseline gap-2 px-1">
          <h3 className="text-xs font-medium text-book-text-main">{t("settingsLogs.workflowDetails")}</h3>
          <p className="text-small text-book-text-muted">{t("settingsLogs.workflowDetailsHint")}</p>
        </div>
        <div className="grid grid-cols-1 items-start gap-2 xl:grid-cols-2">
          <DiagnosticDomainDetails
            title={t("settingsLogs.llmWorkflow")}
            description={t("settingsLogs.tokenCacheDuration")}
            domain="llm-workflows"
          >
            <MetricTable
              emptyText={displayLoading ? t("settingsLogs.loading") : displayFailed ? t("settingsLogs.currentRangeFailed") : t("settingsLogs.noLlmCalls")}
              rows={displayData?.llm_by_workflow ?? []}
              renderHeader={() => (
                <>
                  <th className="py-1 pr-2 text-left">{t("settingsLogs.step")}</th>
                  <th className="px-2 py-1 text-right">{t("settingsLogs.callsHeader")}</th>
                  <th className="px-2 py-1 text-right">{t("settingsLogs.tokensHeader")}</th>
                  <th className="px-2 py-1 text-right">{t("settingsLogs.cacheHeader")}</th>
                  <th className="py-1 pl-2 text-right">{t("settingsLogs.durationHeader")}</th>
                </>
              )}
              renderRow={(row) => (
                <>
                  <td className="min-w-0 py-1 pr-2"><WorkflowLabel workflow={row.workflow} /></td>
                  <td className="px-2 py-1 text-right">{formatInteger(row.calls, locale)}</td>
                  <td className="px-2 py-1 text-right">{formatCompact(row.total_tokens, locale)}</td>
                  <td className="px-2 py-1 text-right">{formatPercent(row.prompt_cache_hit_ratio, locale)}</td>
                  <td className="py-1 pl-2 text-right">{formatDuration(row.avg_duration_ms, locale)}</td>
                </>
              )}
            />
          </DiagnosticDomainDetails>

          <DiagnosticDomainDetails
            title={t("settingsLogs.mediaWorkflow")}
            description={t("settingsLogs.successFailureDuration")}
            domain="media-workflows"
          >
            <MetricTable
              emptyText={displayLoading ? t("settingsLogs.loading") : displayFailed ? t("settingsLogs.currentRangeFailed") : t("settingsLogs.noMediaCalls")}
              rows={displayData?.media_by_workflow ?? []}
              renderHeader={() => (
                <>
                  <th className="py-1 pr-2 text-left">{t("settingsLogs.step")}</th>
                  <th className="px-2 py-1 text-right">{t("settingsLogs.callsHeader")}</th>
                  <th className="px-2 py-1 text-right">{t("settingsLogs.successHeader")}</th>
                  <th className="px-2 py-1 text-right">{t("settingsLogs.failureHeader")}</th>
                  <th className="py-1 pl-2 text-right">{t("settingsLogs.durationHeader")}</th>
                </>
              )}
              renderRow={(row) => (
                <>
                  <td className="min-w-0 py-1 pr-2"><WorkflowLabel workflow={`${kindLabel(row.kind, t)} / ${row.workflow}`} /></td>
                  <td className="px-2 py-1 text-right">{formatInteger(row.calls, locale)}</td>
                  <td className="px-2 py-1 text-right">{formatInteger(row.success, locale)}</td>
                  <td className="px-2 py-1 text-right">{formatInteger(row.failed, locale)}</td>
                  <td className="py-1 pl-2 text-right">{formatDuration(row.avg_duration_ms, locale)}</td>
                </>
              )}
            />
          </DiagnosticDomainDetails>
        </div>
      </section>

      <section aria-label={t("settingsLogs.recentCalls")} className="space-y-2">
        <div className="flex items-baseline gap-2 px-1">
          <h3 className="text-xs font-medium text-book-text-main">{t("settingsLogs.recentCalls")}</h3>
          <p className="text-small text-book-text-muted">{t("settingsLogs.recentCallsHint")}</p>
        </div>
        <div className="grid grid-cols-1 items-start gap-2 xl:grid-cols-2">
          <DiagnosticDomainDetails
            title={t("settingsLogs.llmCalls")}
            description={recentLlmDescription}
            domain="recent-llm"
          >
            <RecentLlmCalls
              rows={displayData?.recent_llm_calls ?? []}
              loading={displayLoading}
              failed={displayFailed}
            />
          </DiagnosticDomainDetails>
          <DiagnosticDomainDetails
            title={t("settingsLogs.mediaCalls")}
            description={recentMediaDescription}
            domain="recent-media"
          >
            <RecentMediaCalls
              rows={displayData?.recent_media_calls ?? []}
              loading={displayLoading}
              failed={displayFailed}
            />
          </DiagnosticDomainDetails>
        </div>
      </section>
    </section>
  );
};

const OverviewRow: React.FC<{
  item: CallKindOverview | undefined;
  kind: string;
  loading: boolean;
  failed: boolean;
  locale: ReturnType<typeof useI18n>["locale"];
}> = ({ item, kind, loading, failed, locale }) => {
  const { t } = useI18n();
  const unavailable = (loading || failed) && !item;
  const value = unavailable
    ? loading
      ? t("settingsLogs.loading")
      : t("settingsLogs.readFailedShort")
    : kind === "llm"
      ? t("settingsLogs.llmOverview", {
          calls: formatInteger(item?.calls ?? 0, locale),
          tokens: formatCompact(item?.total_tokens ?? 0, locale),
          cache: formatPercent(item?.prompt_cache_hit_ratio ?? null, locale),
        })
      : t("settingsLogs.mediaOverview", {
          calls: formatInteger(item?.calls ?? 0, locale),
          success: formatInteger(item?.success ?? 0, locale),
          failure: formatInteger(item?.failed ?? 0, locale),
          duration: formatDuration(item?.avg_duration_ms ?? null, locale),
        });
  return (
    <dl
      className="a4n-flat-surface min-w-0 p-3 text-xs"
      data-generation-kind={kind}
      aria-label={t("settingsLogs.kindSummary", { kind: kindLabel(kind, t) })}
    >
      <dt className="font-medium text-book-text-main">{kindLabel(kind, t)}</dt>
      <dd
        className={`mt-1 min-w-0 text-small leading-relaxed ${
          failed && !item ? "text-error" : "text-book-text-muted"
        }`}
      >
        {value}
      </dd>
    </dl>
  );
};

const MetricValue: React.FC<{ label: string; value: string }> = ({
  label,
  value,
}) => (
  <div className="min-w-0">
    <div className="text-book-text-muted">{label}</div>
    <div className="font-medium tabular-nums truncate">{value}</div>
  </div>
);

const CacheSummary: React.FC<{
  item: CallKindOverview | undefined;
  loading: boolean;
  failed: boolean;
}> = ({ item, loading, failed }) => {
  const { t, locale } = useI18n();
  if (loading && !item) {
    return (
      <div className="text-xs text-book-text-muted py-3">
        {t("settingsLogs.loadingCache")}
      </div>
    );
  }
  if (failed && !item) {
    return (
      <div className="text-xs text-error py-3">{t("settingsLogs.cacheReadFailed")}</div>
    );
  }

  const hit = item?.prompt_cache_hit_tokens ?? 0;
  const miss = item?.prompt_cache_miss_tokens ?? 0;
  const ratio = item?.prompt_cache_hit_ratio;
  const hitWidth = ratio == null ? 0 : ratio * 100;
  const missWidth = ratio == null ? 0 : 100 - hitWidth;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex h-2 overflow-hidden rounded-full bg-book-bg-tertiary">
        <div className="bg-success" style={{ width: `${hitWidth}%` }} />
        <div className="bg-warning" style={{ width: `${missWidth}%` }} />
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs">
        <MetricValue
          label={t("settingsLogs.hitRate")}
          value={formatPercent(item?.prompt_cache_hit_ratio ?? null, locale)}
        />
        <MetricValue label={t("settingsLogs.hitTokens")} value={formatCompact(hit, locale)} />
        <MetricValue label={t("settingsLogs.missTokens")} value={formatCompact(miss, locale)} />
        <MetricValue label={t("settingsLogs.writeTokens")} value={formatOptionalCompact(item?.prompt_cache_write_tokens ?? null, locale)} />
      </div>
    </div>
  );
};

const DailyCallsChart: React.FC<{ rows: DailyCallMetric[] }> = ({ rows }) => {
  const { t, locale } = useI18n();
  const visibleRows = rows.slice(-30);
  const maxCalls = Math.max(
    1,
    ...visibleRows.map(
      (row) => row.llm_calls + row.image_calls + row.video_calls,
    ),
  );
  const barGap = 4;
  const leftPad = 34;
  const bottomPad = 24;
  const topPad = 10;
  const plotWidth = CHART_WIDTH - leftPad - 12;
  const plotHeight = CHART_HEIGHT - topPad - bottomPad;
  const barWidth = Math.max(
    3,
    (plotWidth - barGap * Math.max(0, visibleRows.length - 1)) /
      Math.max(1, visibleRows.length),
  );

  return (
    <div className="w-full overflow-x-auto">
      <svg
        viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
        className="h-44 w-full sm:min-w-[520px]"
        role="img"
        aria-label={t("settingsLogs.dailyGenerationCalls")}
      >
        <line
          x1={leftPad}
          y1={topPad + plotHeight}
          x2={CHART_WIDTH - 8}
          y2={topPad + plotHeight}
          className="stroke-book-border"
        />
        <text x={2} y={topPad + 8} className="fill-book-text-muted text-caption">
          {formatInteger(maxCalls, locale)}
        </text>
        {visibleRows.map((row, index) => {
          const x = leftPad + index * (barWidth + barGap);
          const llmHeight = (row.llm_calls / maxCalls) * plotHeight;
          const imageHeight = (row.image_calls / maxCalls) * plotHeight;
          const videoHeight = (row.video_calls / maxCalls) * plotHeight;
          let y = topPad + plotHeight;
          y -= llmHeight;
          const llmY = y;
          y -= imageHeight;
          const imageY = y;
          y -= videoHeight;
          const videoY = y;
          const showLabel =
            index === 0 || index === visibleRows.length - 1 || index % 6 === 0;
          return (
            <g key={row.date}>
              <rect
                x={x}
                y={llmY}
                width={barWidth}
                height={llmHeight}
                className="fill-info"
              />
              <rect
                x={x}
                y={imageY}
                width={barWidth}
                height={imageHeight}
                className="fill-success"
              />
              <rect
                x={x}
                y={videoY}
                width={barWidth}
                height={videoHeight}
                className="fill-warning"
              />
              {showLabel && (
                <text
                  x={x + barWidth / 2}
                  y={CHART_HEIGHT - 6}
                  textAnchor="middle"
                  className="fill-book-text-muted text-caption"
                >
                  {row.date.slice(5)}
                </text>
              )}
            </g>
          );
        })}
      </svg>
      <div className="flex flex-wrap items-center gap-3 text-small text-book-text-muted">
        <Legend colorClass="bg-info" label="LLM" />
        <Legend colorClass="bg-success" label={t("settingsAdmin.image")} />
        <Legend colorClass="bg-warning" label={t("settingsAdmin.video")} />
      </div>
    </div>
  );
};

const Legend: React.FC<{ colorClass: string; label: string }> = ({
  colorClass,
  label,
}) => (
  <span className="inline-flex items-center gap-1">
    <span className={`h-2 w-2 rounded-full ${colorClass}`} />
    {label}
  </span>
);

const DiagnosticDomainDetails: React.FC<{
  title: string;
  description: string;
  domain: string;
  children: React.ReactNode;
}> = ({ title, description, domain, children }) => (
  <details
    className="a4n-flat-surface group min-w-0 overflow-hidden"
    data-diagnostic-domain={domain}
  >
    <summary className="flex min-h-11 cursor-pointer list-none items-center gap-3 px-3 py-2.5 text-xs marker:content-none">
      <span className="font-medium text-book-text-main">{title}</span>
      <span className="min-w-0 flex-1 truncate text-right text-small text-book-text-muted">
        {description}
      </span>
      <ChevronRight
        size={13}
        className="shrink-0 text-book-text-muted transition-transform duration-200 group-open:rotate-90"
      />
    </summary>
    <div className="border-t border-book-border/30">{children}</div>
  </details>
);

function MetricTable<T>({
  emptyText,
  rows,
  renderHeader,
  renderRow,
}: {
  emptyText: string;
  rows: T[];
  renderHeader: () => React.ReactNode;
  renderRow: (row: T) => React.ReactNode;
}) {
  return (
    <div className="min-w-0 p-3">
      {rows.length === 0 ? (
        <EmptyState text={emptyText} />
      ) : (
        <div className="overflow-x-auto custom-scrollbar">
          <table className="w-full text-xs">
            <thead className="text-book-text-muted border-b border-book-border">
              <tr>{renderHeader()}</tr>
            </thead>
            <tbody>
              {rows.map((row, index) => (
                <tr
                  key={index}
                  className="border-b border-book-border/60 last:border-b-0"
                >
                  {renderRow(row)}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

const WorkflowLabel: React.FC<{ workflow: string }> = ({ workflow }) => (
  <div className="min-w-0" title={workflow}>
    <div className="font-medium truncate">{workflow}</div>
  </div>
);

const RecentLlmCalls: React.FC<{
  rows: RecentLlmCall[];
  loading: boolean;
  failed: boolean;
}> = ({ rows, loading, failed }) => {
  const { t, locale } = useI18n();
  return <div className="min-w-0 p-3">
    {rows.length === 0 ? (
      <EmptyState
        text={
          loading ? t("settingsLogs.loading") : failed ? t("settingsLogs.currentRangeFailed") : t("settingsLogs.noLlmCalls")
        }
      />
    ) : (
      <div>
        {rows.map((row) => (
          <div
            key={row.id}
            className="grid grid-cols-[minmax(0,1fr)_auto] gap-2 py-1.5 border-b border-book-border/60 last:border-b-0 text-xs"
          >
            <WorkflowLabel workflow={row.workflow} />
            <div className="text-right tabular-nums">
                <div>{formatTokenValue(row.total_tokens, locale, t)}</div>
              <div className="text-small text-book-text-muted">
                {t("settingsLogs.inputOutput", {
                  input: formatOptionalCompact(row.prompt_tokens, locale),
                  output: formatOptionalCompact(row.completion_tokens, locale),
                })}
              </div>
              <div className="text-small text-book-text-muted">
                {formatCacheDetail(row, t, locale)} / {formatDuration(row.duration_ms, locale)}
              </div>
            </div>
          </div>
        ))}
      </div>
    )}
  </div>;
};

const RecentMediaCalls: React.FC<{
  rows: RecentMediaCall[];
  loading: boolean;
  failed: boolean;
}> = ({ rows, loading, failed }) => {
  const { locale, t } = useI18n();
  return <div className="min-w-0 p-3">
    {rows.length === 0 ? (
      <EmptyState
        text={
          loading
            ? t("settingsLogs.loading")
            : failed
              ? t("settingsLogs.currentRangeFailed")
              : t("settingsLogs.noMediaCalls")
        }
      />
    ) : (
      <div>
        {rows.map((row) => (
          <div
            key={row.id}
            className="grid grid-cols-[minmax(0,1fr)_auto] gap-2 py-1.5 border-b border-book-border/60 last:border-b-0 text-xs"
          >
            <div className="min-w-0">
              <div className="font-medium truncate">
                {kindLabel(row.kind, t)} / {row.title}
              </div>
              <div
                className="text-small text-book-text-muted truncate"
                title={row.workflow}
              >
                {row.workflow}
              </div>
            </div>
            <div className="text-right tabular-nums">
              <div>{statusLabel(row.status, t)}</div>
              <div className="text-small text-book-text-muted">
                {formatDuration(row.duration_ms, locale)}
              </div>
            </div>
          </div>
        ))}
      </div>
    )}
  </div>;
};

const EmptyState: React.FC<{ text: string }> = ({ text }) => (
  <div className="a4n-empty-state a4n-empty-state--compact text-xs">{text}</div>
);

function kindLabel(kind: string, t: ReturnType<typeof useI18n>["t"]): string {
  if (kind === "llm") return t("settingsLogs.kindLlm");
  if (kind === "image") return t("settingsAdmin.image");
  if (kind === "video") return t("settingsAdmin.video");
  return t("settingsLogs.otherCall");
}

function statusLabel(status: string, t: ReturnType<typeof useI18n>["t"]): string {
  if (status === "completed" || status === "success" || status === "succeeded")
    return t("settingsAdmin.success");
  if (status === "failed" || status === "error") return t("settingsAdmin.failure");
  if (status === "cancelled" || status === "canceled") return t("settingsLogs.cancelled");
  if (status === "pending" || status === "queued" || status === "waiting") return t("settingsLogs.waiting");
  if (status === "running" || status === "processing") return t("settingsLogs.running");
  return t("settingsLogs.unknownStatus");
}

function formatInteger(value: number, locale: ReturnType<typeof useI18n>["locale"]): string {
  return formatNumber(value, locale, { maximumFractionDigits: 0 });
}

function formatCompact(value: number, locale: ReturnType<typeof useI18n>["locale"]): string {
  return formatNumber(value, locale, {
    notation: "compact",
    maximumFractionDigits: 1,
  });
}

function formatOptionalCompact(value: number | null, locale: ReturnType<typeof useI18n>["locale"]): string {
  return value == null ? "-" : formatCompact(value, locale);
}

function formatTokenValue(value: number | null, locale: ReturnType<typeof useI18n>["locale"], t: ReturnType<typeof useI18n>["t"]): string {
  return value == null ? t("settingsLogs.tokensNotReturned") : t("settingsLogs.tokenValue", { count: formatCompact(value, locale) });
}

function formatPercent(value: number | null, locale: ReturnType<typeof useI18n>["locale"]): string {
  if (value == null) return "-";
  return `${formatNumber(value * 100, locale, { maximumFractionDigits: 1 })}%`;
}

function formatCacheDetail(row: RecentLlmCall, t: ReturnType<typeof useI18n>["t"], locale: ReturnType<typeof useI18n>["locale"]): string {
  const hit = row.prompt_cache_hit_tokens;
  const miss = row.prompt_cache_miss_tokens;
  const write = row.prompt_cache_write_tokens;
  if (hit == null && miss == null && write == null && row.prompt_cache_hit_ratio == null) {
    return t("settingsLogs.cacheNotReturned");
  }
  return t("settingsLogs.cacheValue", {
    hit: formatOptionalCompact(hit, locale),
    miss: formatOptionalCompact(miss, locale),
    write: formatOptionalCompact(write, locale),
    ratio: formatPercent(row.prompt_cache_hit_ratio, locale),
  });
}

function formatDuration(value: number | null, locale: ReturnType<typeof useI18n>["locale"]): string {
  if (value == null) return "-";
  if (value < 1000) return `${formatNumber(value, locale, { maximumFractionDigits: 0 })}ms`;
  if (value < 60_000) return `${formatNumber(value / 1000, locale, { maximumFractionDigits: 1 })}s`;
  return `${formatNumber(value / 60_000, locale, { maximumFractionDigits: 1 })}m`;
}
