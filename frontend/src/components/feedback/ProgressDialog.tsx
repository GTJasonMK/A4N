import React, { useEffect, useState } from "react";
import { AlertCircle, CheckCircle2, XCircle } from "lucide-react";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import { Modal } from "../ui/Modal";
import { BookButton } from "../ui/BookButton";
import type {
  ProgressMeta,
  ProgressStatus,
  ProgressTimelineItem,
} from "../../api/progress";
import {
  compactProgressEvents,
  progressDisplayModel,
} from "../../utils/progressPresentation";
import { streamLineFromBuffer } from "../../utils/progressDisplay";
import { usePacedStreamText } from "../../hooks/usePacedStreamText";
import { formatNumber, formatTime, useI18n, type MessageTranslator, type Locale } from "../../i18n";

export type ProgressDialogStatus = ProgressStatus;
export type { ProgressTimelineItem };

interface ProgressDialogProps {
  open: boolean;
  title: string;
  subtitle?: React.ReactNode;
  status: ProgressDialogStatus;
  current: number;
  total: number;
  message: string;
  meta?: ProgressMeta | null;
  streamText?: string | null;
  streaming?: boolean;
  events?: ProgressTimelineItem[];
  error?: string | null;
  onClose: () => void;
  closeLabel?: string;
  onCancel?: () => void;
  cancelLabel?: string;
  cancelling?: boolean;
  runningHint?: React.ReactNode;
  formatProgress?: (current: number, total: number, percent: number) => React.ReactNode;
}

export const ProgressDialog: React.FC<ProgressDialogProps> = ({
  open,
  title,
  subtitle,
  status,
  current,
  total,
  message,
  meta,
  streamText,
  streaming = false,
  events = [],
  error,
  onClose,
  closeLabel,
  onCancel,
  cancelLabel,
  cancelling = false,
  runningHint,
  formatProgress,
}) => {
  const { locale, t } = useI18n();
  const effectiveCloseLabel = closeLabel ?? t("feedback.progressClose");
  const effectiveCancelLabel = cancelLabel ?? t("feedback.progressCancel");
  const effectiveRunningHint = runningHint ?? t("feedback.progressRunningHint");
  const pacedStream = usePacedStreamText(
    streamText ?? "",
    streaming && status === "running",
    streaming,
  );
  const displayStatus = streaming && status !== "running" && !pacedStream.isDrained
    ? "running"
    : status;
  const [timing, setTiming] = useState<{ startedAt: number | null; now: number }>({
    startedAt: null,
    now: Date.now(),
  });
  const boundedTotal = Math.max(1, total || 1);
  const boundedCurrent = Math.max(0, Math.min(current || 0, boundedTotal));
  const percent =
    displayStatus === "success" ? 100 : Math.round((boundedCurrent / boundedTotal) * 100);
  const canClose =
    displayStatus === "success"
    || displayStatus === "error"
    || displayStatus === "idle"
    || displayStatus === "cancelled";
  const tone =
    displayStatus === "error"
      ? "danger"
      : displayStatus === "success"
        ? "success"
        : displayStatus === "cancelled"
          ? "warning"
          : "default";
  const icon =
    displayStatus === "error" ? (
      <AlertCircle size={15} />
    ) : displayStatus === "success" ? (
      <CheckCircle2 size={15} />
    ) : displayStatus === "cancelled" ? (
      <XCircle size={15} />
    ) : (
      <LoadingSpinner size="sm" />
    );
  const progressLabel = formatProgress
    ? formatProgress(boundedCurrent, boundedTotal, percent)
    : defaultProgressLabel(boundedCurrent, boundedTotal, percent, locale);
  const display = progressDisplayModel({
    current: boundedCurrent,
    total: boundedTotal,
    message,
    meta,
    status: displayStatus,
    locale,
    t,
    fallbackStage: statusStageLabel(displayStatus, t),
  });
  const timingLabel = progressTimingLabel(
    timing.startedAt,
    timing.now,
    displayStatus,
    percent,
    display.etaMode,
    locale,
    t,
  );
  const barDetail = display.detail ?? statusDetailLabel(displayStatus, t);
  const displayedStreamText = streamLineFromBuffer(pacedStream.displayedText);
  const hasStreamText = displayedStreamText.length > 0;
  const showStreamPreview = hasStreamText || (streaming && pacedStream.isThinking);
  const timeline = compactProgressEvents(events, 12);

  useEffect(() => {
    if (!open) {
      setTiming({ startedAt: null, now: Date.now() });
      return undefined;
    }
    const startedAt = Date.now();
    setTiming({ startedAt, now: startedAt });
    return undefined;
  }, [open, title]);

  useEffect(() => {
    if (!open || displayStatus !== "running") {
      setTiming((prev) => ({ ...prev, now: Date.now() }));
      return undefined;
    }
    const timer = window.setInterval(() => {
      setTiming((prev) => ({ ...prev, now: Date.now() }));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [displayStatus, open]);

  return (
    <Modal
      isOpen={open}
      onClose={canClose ? onClose : () => undefined}
      title={title}
      subtitle={subtitle}
      icon={icon}
      tone={tone}
      maxWidthClassName="max-w-2xl"
      zIndexClassName="z-[90]"
      closeOnBackdrop={canClose}
      closeOnEscape={canClose}
      showCloseButton={canClose}
      footer={
        canClose ? (
          <BookButton variant="primary" size="sm" onClick={onClose} className="!min-h-8 !text-xs">
            {effectiveCloseLabel}
          </BookButton>
        ) : onCancel ? (
          <>
            <span className="mr-auto text-small leading-relaxed text-book-text-sub">
              {effectiveRunningHint}
            </span>
            <BookButton
              variant="danger"
              size="sm"
              onClick={onCancel}
              disabled={cancelling}
              className="!min-h-8 !text-xs"
            >
              {cancelling ? (
                <LoadingSpinner size="sm" text={t("feedback.progressStopping")} />
              ) : (
                effectiveCancelLabel
              )}
            </BookButton>
          </>
        ) : (
          <span className="text-small text-book-text-sub">
            {effectiveRunningHint}
          </span>
        )
      }
    >
      <div className="space-y-4" aria-live="polite">
        <section>
          <div className="mb-2 flex min-w-0 items-baseline justify-between gap-3">
            <span className="min-w-0 truncate text-sm font-semibold text-book-text-main">
              {display.stage}
            </span>
            <span className="shrink-0 font-mono text-xs tabular-nums text-book-text-sub">
              {progressLabel}
            </span>
          </div>
          <div
            className="relative h-2.5 overflow-hidden rounded-full bg-book-bg-tertiary"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={percent}
            aria-label={display.stage}
          >
            <div
              className={`absolute inset-y-0 left-0 transition-all duration-300 ${
                displayStatus === "error"
                  ? "bg-error"
                  : displayStatus === "success"
                    ? "bg-success"
                    : displayStatus === "cancelled"
                      ? "bg-warning"
                      : "bg-book-primary/30"
              }`}
              style={{ width: `${percent}%` }}
            />
          </div>
          <div className="mt-2.5 flex min-w-0 items-start justify-between gap-4 text-xs">
            <div className="min-w-0 flex-1 leading-relaxed text-book-text-sub">
              <div className="flex min-w-0 items-baseline gap-2">
                <span className="min-w-0 truncate" title={barDetail}>{barDetail}</span>
                {display.countLabel ? (
                  <span className="shrink-0 font-mono text-small text-book-text-muted">
                    {display.countLabel}
                  </span>
                ) : null}
              </div>
              {showStreamPreview ? (
                <div className="mt-1 flex min-w-0 items-center gap-1.5 font-mono text-small text-book-text-muted">
                  {hasStreamText ? (
                    <span className="min-w-0 truncate" title={displayedStreamText}>
                      {displayedStreamText}
                    </span>
                  ) : null}
                  {streaming && pacedStream.isThinking ? (
                    <ThinkingDots label={t("feedback.progressThinking")} />
                  ) : null}
                </div>
              ) : null}
            </div>
            {timingLabel ? (
              <span className="shrink-0 text-right text-small leading-relaxed text-book-text-muted">
                {timingLabel}
              </span>
            ) : null}
          </div>
        </section>

        {error ? (
          <div role="alert" className="rounded-[var(--pyqt-radius-lg)] border border-error/45 bg-error-bg px-3.5 py-3 text-xs leading-relaxed text-book-text-main">
            {error}
          </div>
        ) : null}

        {timeline.length > 1 ? (
          <details className="border-t border-book-border/55 pt-3 text-xs text-book-text-muted">
            <summary className="cursor-pointer select-none font-medium transition-colors hover:text-book-text-main">
              {t("feedback.recentProgress", { count: formatNumber(timeline.length, locale) })}
            </summary>
            <ol className="mt-2 divide-y divide-book-border/45">
              {timeline.map((item) => (
                <ProgressTimelineRow key={item.id} item={item} locale={locale} t={t} />
              ))}
            </ol>
          </details>
        ) : null}

      </div>
    </Modal>
  );
};

const ThinkingDots: React.FC<{ label: string }> = ({ label }) => (
  <span className="inline-flex shrink-0 items-center gap-1 text-book-primary" role="status" aria-label={label}>
    <span aria-hidden="true" className="h-1 w-1 animate-pulse rounded-full bg-current" style={{ animationDelay: "-300ms" }} />
    <span aria-hidden="true" className="h-1 w-1 animate-pulse rounded-full bg-current" style={{ animationDelay: "-150ms" }} />
    <span aria-hidden="true" className="h-1 w-1 animate-pulse rounded-full bg-current" />
  </span>
);

const ProgressTimelineRow: React.FC<{
  item: ProgressTimelineItem;
  locale: Locale;
  t: MessageTranslator;
}> = ({ item, locale, t }) => {
  const display = progressDisplayModel({
    ...item,
    locale,
    t,
    fallbackStage: statusStageLabel(item.status, t),
  });
  const time = formatTime(item.timestamp, locale, t("common.unknown"));
  const dotClass =
    display.tone === "danger"
      ? "bg-error"
      : display.tone === "success"
        ? "bg-success"
        : display.tone === "warning"
          ? "bg-warning"
          : "bg-book-primary";

  return (
    <li className="grid grid-cols-[auto_1fr_auto] gap-2 py-2 text-small">
      <span className={`mt-1 h-1.5 w-1.5 rounded-full ${dotClass}`} />
      <span className="min-w-0">
        <span className="block truncate font-semibold text-book-text-main" title={display.stage}>
          {display.stage}
        </span>
        {display.detail ? (
          <span className="block truncate text-book-text-muted" title={display.detail}>
            {display.detail}
          </span>
        ) : null}
      </span>
      <span className="shrink-0 font-mono text-book-text-muted">{time}</span>
    </li>
  );
};

function defaultProgressLabel(current: number, total: number, percent: number, locale: Locale): string {
  const currentText = formatNumber(current, locale);
  const totalText = formatNumber(total, locale);
  const percentText = formatNumber(percent, locale);
  if (total === 100) return `${percentText}%`;
  return `${currentText}/${totalText} (${percentText}%)`;
}

function statusStageLabel(status: ProgressDialogStatus, t: MessageTranslator): string {
  switch (status) {
    case "success":
      return t("feedback.progressCompleted");
    case "error":
      return t("feedback.progressFailed");
    case "cancelled":
      return t("feedback.progressCancelled");
    case "idle":
      return t("feedback.progressPreparing");
    case "running":
    default:
      return t("feedback.progressRunning");
  }
}

function statusDetailLabel(status: ProgressDialogStatus, t: MessageTranslator): string {
  switch (status) {
    case "success":
      return t("feedback.progressDone");
    case "error":
      return t("feedback.progressCheckError");
    case "cancelled":
      return t("feedback.progressStopped");
    case "idle":
      return t("feedback.progressWaiting");
    case "running":
    default:
      return t("feedback.progressWaitingSubstage");
  }
}

function progressTimingLabel(
  startedAt: number | null,
  now: number,
  status: ProgressDialogStatus,
  percent: number,
  etaMode: "hidden" | "rough" | "linear",
  locale: Locale,
  t: MessageTranslator,
): string | null {
  if (!startedAt) return null;
  const elapsedSecs = Math.max(0, Math.floor((now - startedAt) / 1000));
  const elapsed = formatDuration(elapsedSecs, locale, t);
  if (status !== "running") return t("feedback.elapsed", { duration: elapsed });
  if (etaMode === "hidden") return t("feedback.elapsed", { duration: elapsed });
  if (percent <= 1 || percent >= 100) return t("feedback.elapsed", { duration: elapsed });
  const remainingSecs = Math.max(0, Math.round((elapsedSecs * (100 - percent)) / percent));
  const prefix = etaMode === "linear" ? t("feedback.etaExpected") : t("feedback.etaRough");
  return t("feedback.eta", {
    elapsed,
    prefix,
    remaining: formatDuration(remainingSecs, locale, t),
  });
}

function formatDuration(totalSecs: number, locale: Locale, t: MessageTranslator): string {
  const secs = Math.max(0, Math.floor(totalSecs));
  const minutes = Math.floor(secs / 60);
  const seconds = secs % 60;
  if (minutes <= 0) return t("feedback.seconds", { count: formatNumber(seconds, locale) });
  return t("feedback.minutes", {
    minutes: formatNumber(minutes, locale),
    seconds: formatNumber(seconds, locale, { minimumIntegerDigits: 2 }),
  });
}
