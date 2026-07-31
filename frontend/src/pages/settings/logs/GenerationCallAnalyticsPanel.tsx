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
  type GenerationCallKind,
  type RecentLlmCall,
  type RecentMediaCall,
} from "../../../api/generationAnalytics";
import { useToast } from "../../../components/feedback/Toast";
import { useAuthStore } from "../../../store/auth";
import { logger } from "../../../utils/logger";

const KIND_LABELS: Record<string, string> = {
  llm: "语言模型",
  image: "图片",
  video: "视频",
};

const CHART_WIDTH = 720;
const CHART_HEIGHT = 164;

export const GenerationCallAnalyticsPanel: React.FC = () => {
  const { addToast } = useToast();
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
      const result = await getGenerationCallAnalytics(callerUserId, {
        days,
        recent_limit: 40,
      });
      if (!mountedRef.current || requestSeqRef.current !== requestSeq) return;
      setData(result);
      setLoadedKey(requestKey);
      setErrorState(null);
    } catch (err) {
      if (!mountedRef.current || requestSeqRef.current !== requestSeq) return;
      const message = extractErrorMessage(err, "读取生成调用分析失败");
      setData(null);
      setLoadedKey(requestKey);
      setErrorState({ key: requestKey, message });
      addToast(message, "error");
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
  }, [addToast, callerUserId, canReadDiagnostics, days]);

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
    ? "正在加载…"
    : displayFailed
      ? "读取失败"
      : displayData && displayData.recent_llm_calls.length > 0
        ? `${displayData.recent_llm_calls.length} 条记录`
        : "暂无记录";
  const recentMediaDescription = displayLoading
    ? "正在加载…"
    : displayFailed
      ? "读取失败"
      : displayData && displayData.recent_media_calls.length > 0
        ? `${displayData.recent_media_calls.length} 条记录`
        : "暂无记录";

  if (!canReadDiagnostics) {
    return (
      <div className="border-l-2 border-book-border/70 py-2 pl-4 text-sm text-book-text-muted">
        服务器诊断数据仅管理员可查看。
      </div>
    );
  }

  return (
    <section className="flex flex-col gap-3">
      <div className="flex min-h-8 flex-wrap items-center gap-2 text-xs">
        <label className="inline-flex items-center gap-2 text-book-text-muted">
          统计范围
          <select
            value={days}
            onChange={(event) => setDays(Number(event.target.value))}
            className="pyqt-input w-28 px-2 py-1"
            aria-label="生成调用统计范围"
          >
            <option value={7}>最近 7 天</option>
            <option value={30}>最近 30 天</option>
            <option value={90}>最近 90 天</option>
            <option value={180}>最近 180 天</option>
          </select>
        </label>
        <span className="flex-1" />
        <BookIconButton
          label="刷新生成调用分析"
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
          />
        ))}
      </div>

      <div className="grid grid-cols-1 items-start gap-2 xl:grid-cols-[minmax(0,1.35fr)_minmax(320px,0.65fr)]">
        <section className="a4n-flat-surface min-w-0 p-3">
          <h3 className="mb-2 text-xs font-medium">每日调用量</h3>
          {displayData && displayData.daily.length > 0 ? (
            <DailyCallsChart rows={displayData.daily} />
          ) : (
            <EmptyState
              text={
                displayLoading
                  ? "加载中..."
                  : displayFailed
                    ? "当前范围读取失败"
                    : "暂无调用记录"
              }
            />
          )}
        </section>

        <section className="a4n-flat-surface min-w-0 p-3">
          <h3 className="mb-2 text-xs font-medium">提示词缓存</h3>
          <CacheSummary
            item={llmOverview}
            loading={displayLoading}
            failed={displayFailed}
          />
        </section>
      </div>

      <section aria-label="工作流明细" className="space-y-2">
        <div className="flex items-baseline gap-2 px-1">
          <h3 className="text-xs font-medium text-book-text-main">工作流明细</h3>
          <p className="text-small text-book-text-muted">按调用域查看消耗与生成表现</p>
        </div>
        <div className="grid grid-cols-1 items-start gap-2 xl:grid-cols-2">
          <DiagnosticDomainDetails
            title="语言模型工作流"
            description="令牌、缓存与耗时"
            domain="llm-workflows"
          >
            <MetricTable
              emptyText={displayLoading ? "加载中..." : displayFailed ? "当前范围读取失败" : "暂无语言模型调用"}
              rows={displayData?.llm_by_workflow ?? []}
              renderHeader={() => (
                <>
                  <th className="py-1 pr-2 text-left">步骤</th>
                  <th className="px-2 py-1 text-right">调用</th>
                  <th className="px-2 py-1 text-right">令牌数</th>
                  <th className="px-2 py-1 text-right">缓存</th>
                  <th className="py-1 pl-2 text-right">耗时</th>
                </>
              )}
              renderRow={(row) => (
                <>
                  <td className="min-w-0 py-1 pr-2"><WorkflowLabel workflow={row.workflow} /></td>
                  <td className="px-2 py-1 text-right">{formatInteger(row.calls)}</td>
                  <td className="px-2 py-1 text-right">{formatCompact(row.total_tokens)}</td>
                  <td className="px-2 py-1 text-right">{formatPercent(row.prompt_cache_hit_ratio)}</td>
                  <td className="py-1 pl-2 text-right">{formatDuration(row.avg_duration_ms)}</td>
                </>
              )}
            />
          </DiagnosticDomainDetails>

          <DiagnosticDomainDetails
            title="图片与视频工作流"
            description="成功、失败与耗时"
            domain="media-workflows"
          >
            <MetricTable
              emptyText={displayLoading ? "加载中..." : displayFailed ? "当前范围读取失败" : "暂无媒体生成记录"}
              rows={displayData?.media_by_workflow ?? []}
              renderHeader={() => (
                <>
                  <th className="py-1 pr-2 text-left">步骤</th>
                  <th className="px-2 py-1 text-right">调用</th>
                  <th className="px-2 py-1 text-right">成功</th>
                  <th className="px-2 py-1 text-right">失败</th>
                  <th className="py-1 pl-2 text-right">耗时</th>
                </>
              )}
              renderRow={(row) => (
                <>
                  <td className="min-w-0 py-1 pr-2"><WorkflowLabel workflow={`${kindLabel(row.kind)} / ${row.workflow}`} /></td>
                  <td className="px-2 py-1 text-right">{formatInteger(row.calls)}</td>
                  <td className="px-2 py-1 text-right">{formatInteger(row.success)}</td>
                  <td className="px-2 py-1 text-right">{formatInteger(row.failed)}</td>
                  <td className="py-1 pl-2 text-right">{formatDuration(row.avg_duration_ms)}</td>
                </>
              )}
            />
          </DiagnosticDomainDetails>
        </div>
      </section>

      <section aria-label="最近调用" className="space-y-2">
        <div className="flex items-baseline gap-2 px-1">
          <h3 className="text-xs font-medium text-book-text-main">最近调用</h3>
          <p className="text-small text-book-text-muted">按需查看逐条记录</p>
        </div>
        <div className="grid grid-cols-1 items-start gap-2 xl:grid-cols-2">
          <DiagnosticDomainDetails
            title="语言模型调用"
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
            title="图片与视频调用"
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
  kind: GenerationCallKind;
  loading: boolean;
  failed: boolean;
}> = ({ item, kind, loading, failed }) => {
  const unavailable = (loading || failed) && !item;
  const value = unavailable
    ? loading
      ? "正在加载…"
      : "读取失败"
    : kind === "llm"
      ? `${formatInteger(item?.calls ?? 0)} 次 · ${formatCompact(item?.total_tokens ?? 0)} 令牌 · 缓存命中 ${formatPercent(item?.prompt_cache_hit_ratio ?? null)}`
      : `${formatInteger(item?.calls ?? 0)} 次 · 成功 ${formatInteger(item?.success ?? 0)} · 失败 ${formatInteger(item?.failed ?? 0)} · 平均 ${formatDuration(item?.avg_duration_ms ?? null)}`;
  return (
    <dl
      className="a4n-flat-surface min-w-0 p-3 text-xs"
      data-generation-kind={kind}
      aria-label={`${kindLabel(kind)}调用摘要`}
    >
      <dt className="font-medium text-book-text-main">{kindLabel(kind)}</dt>
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
  if (loading && !item) {
    return (
      <div className="text-xs text-book-text-muted py-3">
        正在加载当前范围缓存统计...
      </div>
    );
  }
  if (failed && !item) {
    return (
      <div className="text-xs text-error py-3">当前范围缓存统计读取失败</div>
    );
  }

  const hit = item?.prompt_cache_hit_tokens ?? 0;
  const miss = item?.prompt_cache_miss_tokens ?? 0;
  const total = hit + miss;
  const hitWidth =
    total > 0 ? Math.max(0, Math.min(100, (hit / total) * 100)) : 0;
  const missWidth = total > 0 ? 100 - hitWidth : 0;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex h-2 overflow-hidden rounded-full bg-book-bg-tertiary">
        <div className="bg-success" style={{ width: `${hitWidth}%` }} />
        <div className="bg-warning" style={{ width: `${missWidth}%` }} />
      </div>
      <div className="grid grid-cols-3 gap-2 text-xs">
        <MetricValue
          label="命中率"
          value={formatPercent(item?.prompt_cache_hit_ratio ?? null)}
        />
        <MetricValue label="命中令牌" value={formatCompact(hit)} />
        <MetricValue label="未命中令牌" value={formatCompact(miss)} />
      </div>
    </div>
  );
};

const DailyCallsChart: React.FC<{ rows: DailyCallMetric[] }> = ({ rows }) => {
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
        aria-label="每日生成调用量"
      >
        <line
          x1={leftPad}
          y1={topPad + plotHeight}
          x2={CHART_WIDTH - 8}
          y2={topPad + plotHeight}
          className="stroke-book-border"
        />
        <text x={2} y={topPad + 8} className="fill-book-text-muted text-caption">
          {formatInteger(maxCalls)}
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
        <Legend colorClass="bg-success" label="图片" />
        <Legend colorClass="bg-warning" label="视频" />
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
}> = ({ rows, loading, failed }) => (
  <div className="min-w-0 p-3">
    {rows.length === 0 ? (
      <EmptyState
        text={
          loading ? "加载中..." : failed ? "当前范围读取失败" : "暂无语言模型调用"
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
              <div>{formatTokenValue(row.total_tokens)}</div>
              <div className="text-small text-book-text-muted">
                输入 {formatOptionalCompact(row.prompt_tokens)} / 输出{" "}
                {formatOptionalCompact(row.completion_tokens)}
              </div>
              <div className="text-small text-book-text-muted">
                {formatCacheDetail(row)} / {formatDuration(row.duration_ms)}
              </div>
            </div>
          </div>
        ))}
      </div>
    )}
  </div>
);

const RecentMediaCalls: React.FC<{
  rows: RecentMediaCall[];
  loading: boolean;
  failed: boolean;
}> = ({ rows, loading, failed }) => (
  <div className="min-w-0 p-3">
    {rows.length === 0 ? (
      <EmptyState
        text={
          loading
            ? "加载中..."
            : failed
              ? "当前范围读取失败"
              : "暂无媒体生成记录"
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
                {kindLabel(row.kind)} / {row.title}
              </div>
              <div
                className="text-small text-book-text-muted truncate"
                title={row.workflow}
              >
                {row.workflow}
              </div>
            </div>
            <div className="text-right tabular-nums">
              <div>{statusLabel(row.status)}</div>
              <div className="text-small text-book-text-muted">
                {formatDuration(row.duration_ms)}
              </div>
            </div>
          </div>
        ))}
      </div>
    )}
  </div>
);

const EmptyState: React.FC<{ text: string }> = ({ text }) => (
  <div className="a4n-empty-state a4n-empty-state--compact text-xs">{text}</div>
);

function kindLabel(kind: GenerationCallKind): string {
  return KIND_LABELS[kind] ?? "其他调用";
}

function statusLabel(status: string): string {
  if (status === "completed" || status === "success" || status === "succeeded")
    return "成功";
  if (status === "failed" || status === "error") return "失败";
  if (status === "cancelled" || status === "canceled") return "取消";
  if (status === "pending" || status === "queued" || status === "waiting") return "等待中";
  if (status === "running" || status === "processing") return "进行中";
  return "未知状态";
}

function formatInteger(value: number): string {
  return new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 0 }).format(
    value,
  );
}

function formatCompact(value: number): string {
  return new Intl.NumberFormat("zh-CN", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}

function formatOptionalCompact(value: number | null): string {
  return value == null ? "-" : formatCompact(value);
}

function formatTokenValue(value: number | null): string {
  return value == null ? "未返回令牌" : `${formatCompact(value)} 令牌`;
}

function formatPercent(value: number | null): string {
  if (value == null) return "-";
  return `${Math.round(value * 1000) / 10}%`;
}

function formatCacheDetail(row: RecentLlmCall): string {
  const hit = row.prompt_cache_hit_tokens;
  const miss = row.prompt_cache_miss_tokens;
  if (hit == null && miss == null && row.prompt_cache_hit_ratio == null) {
    return "缓存未返回";
  }
  return `缓存 ${formatOptionalCompact(hit)}/${formatOptionalCompact(miss)} (${formatPercent(
    row.prompt_cache_hit_ratio,
  )})`;
}

function formatDuration(value: number | null): string {
  if (value == null) return "-";
  if (value < 1000) return `${value}ms`;
  if (value < 60_000) return `${Math.round(value / 100) / 10}s`;
  return `${Math.round(value / 6000) / 10}m`;
}
