import React from "react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { QueueStatsSummary } from "../../../api/task";
import { formatNumber, useI18n, type Locale } from "../../../i18n";

interface QueueStatsGridProps {
  stats: QueueStatsSummary | null;
}

export const QueueStatsGrid: React.FC<QueueStatsGridProps> = ({ stats }) => {
  const { locale, t } = useI18n();
  return (
    <section className="a4n-flat-surface overflow-hidden">
      <div className="flex items-center gap-3 border-b border-book-border/35 px-3 py-2.5">
        <h3 className="text-xs font-semibold text-book-text-main">{t("settingsQueue.queueLoad")}</h3>
        <span className="flex-1" />
        <QueueTotal locale={locale} stats={stats} />
      </div>
      <dl className="divide-y divide-book-border/30">
        <QueueMetric locale={locale} label={t("settingsQueue.languageModel")} waiting={stats?.llm_queue.waiting} running={stats?.llm_queue.running} />
        <QueueMetric locale={locale} label={t("settingsQueue.imageGeneration")} waiting={stats?.image_queue.waiting} running={stats?.image_queue.running} />
        <QueueMetric locale={locale} label={t("settingsQueue.videoGeneration")} waiting={stats?.video_queue.waiting} running={stats?.video_queue.running} />
        <QueueMetric locale={locale} label={t("settingsQueue.embedding")} waiting={stats?.embedding_queue.waiting} running={stats?.embedding_queue.running} />
        <QueueMetric locale={locale} label={t("settingsQueue.backgroundTasks")} waiting={stats?.background_queue.waiting} running={stats?.background_queue.running} />
      </dl>
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-book-border/35 px-3 py-2 text-small text-book-text-muted">
        <span className="font-medium text-book-text-sub">{t("settingsQueue.waitingPriority")}</span>
        <PrioritySummary stats={stats} />
      </div>
    </section>
  );
};

const QueueMetric: React.FC<{
  locale: Locale;
  label: string;
  waiting: number | undefined;
  running: number | undefined;
}> = ({ locale, label, waiting, running }) => {
  const { t } = useI18n();
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-4 px-3 py-2 text-xs">
      <dt className="min-w-0 truncate font-medium text-book-text-main">{label}</dt>
      <MetricValue locale={locale} label={t("settingsQueue.waiting")} value={waiting} />
      <MetricValue locale={locale} label={t("settingsQueue.executing")} value={running} active />
    </div>
  );
};

const MetricValue: React.FC<{
  locale: Locale;
  label: string;
  value: number | undefined;
  active?: boolean;
}> = ({ locale, label, value, active = false }) => (
  <dd className="flex min-w-[3.5rem] items-baseline justify-end gap-1 text-right">
    <span className="text-caption text-book-text-muted">{label}</span>
    <span className={`font-mono text-sm font-semibold ${active ? "text-book-primary" : "text-book-text-main"}`}>
      {value == null ? <LoadingSpinner size="sm" className="text-book-text-muted" /> : formatNumber(value, locale)}
    </span>
  </dd>
);

const QueueTotal: React.FC<{ locale: Locale; stats: QueueStatsSummary | null }> = ({ locale, stats }) => {
  const { t } = useI18n();
  if (!stats) return <LoadingSpinner size="sm" className="text-book-text-muted" />;
  const queues = [
    stats.llm_queue,
    stats.image_queue,
    stats.video_queue,
    stats.embedding_queue,
    stats.background_queue,
  ];
  const waiting = queues.reduce((sum, queue) => sum + queue.waiting, 0);
  const running = queues.reduce((sum, queue) => sum + queue.running, 0);
  return (
    <span className="text-small text-book-text-muted">
      {t("settingsQueue.waiting")} <b className="font-mono font-semibold text-book-text-main">{formatNumber(waiting, locale)}</b>
      <span className="mx-1.5">·</span>
      {t("settingsQueue.executing")} <b className="font-mono font-semibold text-book-primary">{formatNumber(running, locale)}</b>
    </span>
  );
};

const PrioritySummary: React.FC<{ stats: QueueStatsSummary | null }> = ({ stats }) => {
  const { locale, t } = useI18n();
  if (!stats) return <span>{t("settingsQueue.loading")}</span>;
  const total =
    stats.high_priority_waiting +
    stats.medium_priority_waiting +
    stats.low_priority_waiting;
  if (total === 0) return <span>{t("settingsQueue.noWaitingTasks")}</span>;
  return (
    <>
      <PriorityMetric locale={locale} label={t("settingsQueue.high")} value={stats.high_priority_waiting} className="text-error" />
      <PriorityMetric locale={locale} label={t("settingsQueue.medium")} value={stats.medium_priority_waiting} className="text-book-primary" />
      <PriorityMetric locale={locale} label={t("settingsQueue.low")} value={stats.low_priority_waiting} />
    </>
  );
};

const PriorityMetric: React.FC<{
  locale: Locale;
  label: string;
  value: number | undefined;
  className?: string;
}> = ({ locale, label, value, className = "text-book-text-sub" }) => (
  <span className="inline-flex items-center gap-1">
    {label}
    <b className={`font-mono ${className}`}>{value == null ? "—" : formatNumber(value, locale)}</b>
  </span>
);
