import React, { useEffect, useState } from "react";
import { AlertCircle, CheckCircle2, Loader2, XCircle } from "lucide-react";
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
  streamVersion?: number;
  events?: ProgressTimelineItem[];
  error?: string | null;
  connectionWarning?: string | null;
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
  streamVersion = 0,
  events = [],
  error,
  connectionWarning,
  onClose,
  closeLabel = "关闭",
  onCancel,
  cancelLabel = "停止生成",
  cancelling = false,
  runningHint = "生成过程中请保持当前窗口打开",
  formatProgress,
}) => {
  const [streamFrame, setStreamFrame] = useState<{
    current: string;
    previous: string;
    version: number;
  }>({ current: "", previous: "", version: 0 });
  const [timing, setTiming] = useState<{ startedAt: number | null; now: number }>({
    startedAt: null,
    now: Date.now(),
  });
  const boundedTotal = Math.max(1, total || 1);
  const boundedCurrent = Math.max(0, Math.min(current || 0, boundedTotal));
  const percent =
    status === "success" ? 100 : Math.round((boundedCurrent / boundedTotal) * 100);
  const canClose =
    status === "success" || status === "error" || status === "idle" || status === "cancelled";
  const tone =
    status === "error"
      ? "danger"
      : status === "success"
        ? "success"
        : status === "cancelled"
          ? "warning"
          : "default";
  const icon =
    status === "error" ? (
      <AlertCircle size={15} />
    ) : status === "success" ? (
      <CheckCircle2 size={15} />
    ) : status === "cancelled" ? (
      <XCircle size={15} />
    ) : (
      <Loader2 size={15} className="animate-spin" />
    );
  const progressLabel = formatProgress
    ? formatProgress(boundedCurrent, boundedTotal, percent)
    : defaultProgressLabel(boundedCurrent, boundedTotal, percent);
  const display = progressDisplayModel({
    current: boundedCurrent,
    total: boundedTotal,
    message,
    meta,
    streamText,
    status,
    fallbackStage: statusStageLabel(status),
  });
  const timingLabel = progressTimingLabel(
    timing.startedAt,
    timing.now,
    status,
    percent,
    display.etaMode,
  );
  const barDetail = display.detail ?? statusDetailLabel(status);
  const incomingStreamText = streamText?.trim() ?? "";
  const hasStreamText = incomingStreamText.length > 0 && streamFrame.current.trim().length > 0;
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
    if (!open || status !== "running") {
      setTiming((prev) => ({ ...prev, now: Date.now() }));
      return undefined;
    }
    const timer = window.setInterval(() => {
      setTiming((prev) => ({ ...prev, now: Date.now() }));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [open, status]);

  useEffect(() => {
    setStreamFrame((prev) => {
      if (prev.current === incomingStreamText) return prev;
      if (!incomingStreamText) {
        return { current: "", previous: "", version: prev.version + 1 };
      }
      return {
        current: incomingStreamText,
        previous: prev.current,
        version: Math.max(prev.version + 1, streamVersion),
      };
    });
  }, [incomingStreamText, streamVersion]);

  return (
    <Modal
      isOpen={open}
      onClose={canClose ? onClose : () => undefined}
      title={title}
      subtitle={subtitle}
      icon={icon}
      tone={tone}
      maxWidthClassName="max-w-2xl"
      closeOnBackdrop={canClose}
      closeOnEscape={canClose}
      showCloseButton={canClose}
      footer={
        canClose ? (
          <BookButton variant="secondary" onClick={onClose}>
            {closeLabel}
          </BookButton>
        ) : onCancel ? (
          <>
            <span className="text-[11px] text-book-text-sub">
              {runningHint}
            </span>
            <BookButton variant="danger" onClick={onCancel} disabled={cancelling}>
              {cancelling ? (
                <span className="inline-flex items-center gap-2">
                  <Loader2 size={13} className="animate-spin" /> 正在停止…
                </span>
              ) : (
                cancelLabel
              )}
            </BookButton>
          </>
        ) : (
          <span className="text-[11px] text-book-text-sub">
            {runningHint}
          </span>
        )
      }
    >
      <div className="space-y-4">
        <div>
          <div className="mb-1.5 flex items-center justify-between gap-3 text-xs">
            <span className="min-w-0 truncate font-semibold text-book-text-main">
              {display.stage}
            </span>
            <span className="shrink-0 font-mono text-book-text-sub">
              {progressLabel}
            </span>
          </div>
          <div
            className="relative h-7 overflow-hidden rounded-sm border border-book-border bg-book-bg-tertiary"
          >
            <div
              className={`absolute inset-y-0 left-0 transition-all duration-300 ${
                status === "error"
                  ? "bg-error"
                  : status === "success"
                    ? "bg-success"
                    : status === "cancelled"
                      ? "bg-warning"
                      : "bg-book-primary/30"
              }`}
              style={{ width: `${percent}%` }}
            />
            {hasStreamText ? (
              <div className="pointer-events-none absolute inset-x-2 top-1/2 h-4 -translate-y-1/2 overflow-hidden">
                {streamFrame.previous ? (
                  <span
                    key={`previous-${streamFrame.version}`}
                    className="a4n-progress-stream-line a4n-progress-stream-line-exit text-book-text-sub"
                  >
                    {streamFrame.previous}
                  </span>
                ) : null}
                <span
                  key={`current-${streamFrame.version}`}
                  className="a4n-progress-stream-line a4n-progress-stream-line-enter text-book-text-main"
                >
                  {streamFrame.current}
                </span>
              </div>
            ) : (
              null
            )}
          </div>
          <div className="mt-2 grid gap-1.5 text-xs">
            <div className="min-w-0 rounded-sm border border-book-border/60 bg-book-bg-secondary px-2.5 py-2">
              <div className="flex items-center justify-between gap-2">
                <span className="min-w-0 truncate font-semibold text-book-text-main" title={barDetail}>
                  {barDetail}
                </span>
                {display.countLabel ? (
                  <span className="shrink-0 font-mono text-[11px] text-book-text-muted">
                    {display.countLabel}
                  </span>
                ) : null}
              </div>
              {hasStreamText ? (
                <div className="mt-1 truncate text-[11px] text-book-text-muted" title={streamFrame.current}>
                  {streamFrame.current}
                </div>
              ) : null}
            </div>
          </div>
          {timingLabel ? (
            <div className="mt-1.5 min-h-5 pb-0.5 text-right text-[11px] leading-5 text-book-text-sub">
              <span className="inline-block max-w-full break-words align-top">
                {timingLabel}
              </span>
            </div>
          ) : null}
        </div>

        {error ? (
          <div className="rounded-sm border border-error bg-error-bg px-3 py-2 text-xs leading-relaxed text-book-text-main">
            {error}
          </div>
        ) : null}

        {connectionWarning ? (
          <div className="rounded-sm border border-warning/60 bg-warning-bg px-3 py-2 text-xs leading-relaxed text-book-text-main">
            {connectionWarning}
          </div>
        ) : null}

        {timeline.length > 1 ? (
          <div className="rounded-sm border border-book-border/70 bg-book-bg-secondary">
            <div className="border-b border-book-border/60 px-3 py-2 text-[11px] font-semibold text-book-text-sub">
              最近进展
            </div>
            <ol className="max-h-56 overflow-y-auto px-3 py-2 custom-scrollbar">
              {timeline.map((item) => (
                <ProgressTimelineRow key={item.id} item={item} />
              ))}
            </ol>
          </div>
        ) : null}

      </div>
    </Modal>
  );
};

const ProgressTimelineRow: React.FC<{ item: ProgressTimelineItem }> = ({ item }) => {
  const display = progressDisplayModel({
    ...item,
    fallbackStage: statusStageLabel(item.status),
  });
  const time = new Date(item.timestamp).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
  const dotClass =
    display.tone === "danger"
      ? "bg-error"
      : display.tone === "success"
        ? "bg-success"
        : display.tone === "warning"
          ? "bg-warning"
          : "bg-book-primary";

  return (
    <li className="grid grid-cols-[auto_1fr_auto] gap-2 py-1.5 text-[11px]">
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

function defaultProgressLabel(current: number, total: number, percent: number): string {
  if (total === 100) return `${percent}%`;
  return `${current}/${total} (${percent}%)`;
}

function statusStageLabel(status: ProgressDialogStatus): string {
  switch (status) {
    case "success":
      return "任务已完成";
    case "error":
      return "任务失败";
    case "cancelled":
      return "任务已停止";
    case "idle":
      return "准备中";
    case "running":
    default:
      return "任务执行中";
  }
}

function statusDetailLabel(status: ProgressDialogStatus): string {
  switch (status) {
    case "success":
      return "已完成";
    case "error":
      return "请查看错误信息";
    case "cancelled":
      return "已停止";
    case "idle":
      return "等待开始";
    case "running":
    default:
      return "等待子阶段更新…";
  }
}

function progressTimingLabel(
  startedAt: number | null,
  now: number,
  status: ProgressDialogStatus,
  percent: number,
  etaMode: "hidden" | "rough" | "linear",
): string | null {
  if (!startedAt) return null;
  const elapsedSecs = Math.max(0, Math.floor((now - startedAt) / 1000));
  if (status !== "running") return `已用 ${formatDuration(elapsedSecs)}`;
  if (etaMode === "hidden") return `已用 ${formatDuration(elapsedSecs)}`;
  if (percent <= 1 || percent >= 100) return `已用 ${formatDuration(elapsedSecs)}`;
  const remainingSecs = Math.max(0, Math.round((elapsedSecs * (100 - percent)) / percent));
  const prefix = etaMode === "linear" ? "预计剩余" : "粗略剩余";
  return `已用 ${formatDuration(elapsedSecs)} · ${prefix} ${formatDuration(remainingSecs)}`;
}

function formatDuration(totalSecs: number): string {
  const secs = Math.max(0, Math.floor(totalSecs));
  const minutes = Math.floor(secs / 60);
  const seconds = secs % 60;
  if (minutes <= 0) return `${seconds} 秒`;
  return `${minutes} 分 ${seconds.toString().padStart(2, "0")} 秒`;
}
