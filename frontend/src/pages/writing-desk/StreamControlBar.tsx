import React from "react";
import { Play, Square } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { useI18n } from "../../i18n";

interface StreamControlBarProps {
  activeNumber: number | null;
  streamTaskId: string | null;
  streamBusy: boolean;
  streamMessage?: string | null;
  streamThinking?: boolean;
  canStart: boolean;
  startDisabledReason?: string | null;
  resetBusy: boolean;
  onStart: () => void;
  onCancel: () => void;
}

/**
 * 流式生成控制条：启动 / 取消。
 */
export const StreamControlBar: React.FC<StreamControlBarProps> = ({
  activeNumber,
  streamTaskId,
  streamBusy,
  streamMessage,
  streamThinking = false,
  canStart,
  startDisabledReason,
  resetBusy,
  onStart,
  onCancel,
}) => {
  const { t } = useI18n();
  const generationActive = Boolean(streamTaskId || streamBusy || resetBusy);
  return (
  <div
    className="a4n-stream-control min-w-0 space-y-2 text-xs"
    data-stream-active={generationActive ? "true" : undefined}
    aria-busy={generationActive}
  >
    {resetBusy ? (
      <div className="a4n-stream-state flex min-w-0 items-center gap-2 text-small" role="status" aria-live="polite">
        <LoadingSpinner size="sm" className="shrink-0" />
        <span className="truncate">{t("writing.resettingProjectData")}…</span>
      </div>
    ) : streamTaskId ? (
      <div className="a4n-stream-state flex min-w-0 items-center gap-2 text-small" role="status" aria-live="polite">
        <LoadingSpinner
          size="sm"
          text={streamThinking ? t("feedback.progressThinking") : undefined}
          className="shrink-0"
        />
        <span className="truncate">
          {activeNumber != null
            ? `${t("writing.generatingChapter")} ${t("writing.chapterLabel", { number: activeNumber })}`
            : t("writing.generatingBody")}
        </span>
      </div>
    ) : null}
    {streamTaskId && (
      <details className="text-caption text-book-text-muted">
        <summary className="w-fit cursor-pointer transition-colors hover:text-book-text-main">
          {t("writing.diagnostics")}
        </summary>
        <div className="mt-1 break-all border-l border-book-border/40 pl-2 font-mono">
          {t("writing.taskId")}: {streamTaskId}
        </div>
      </details>
    )}
    {streamMessage ? (
      <div className="a4n-stream-message flex min-w-0 items-center gap-1.5 rounded-lg px-2.5 py-2 text-small">
        <span className="min-w-0 break-words">{streamMessage}</span>
      </div>
    ) : null}
    <div className="flex min-w-0 gap-2">
      {!streamTaskId ? (
        <BookButton
          variant="primary"
          onClick={onStart}
          disabled={streamBusy || resetBusy || !activeNumber || !canStart}
          className="min-w-0 flex-1 !px-2 !whitespace-normal"
          title={startDisabledReason === null ? undefined : startDisabledReason}
        >
          {streamBusy ? (
            <LoadingSpinner size="sm" />
          ) : (
            <span className="inline-flex min-w-0 items-center gap-1">
              <Play size={11} className="shrink-0" />
              <span className="truncate">{t("writing.startGeneration")}</span>
            </span>
          )}
        </BookButton>
      ) : (
        <button
          type="button"
          onClick={onCancel}
          disabled={streamBusy}
          className="a4n-touch-target pyqt-btn-secondary inline-flex min-w-0 flex-1 items-center justify-center gap-1 !border-error/50 !px-2 text-small !text-error sm:!min-h-[28px]"
          title={t("writing.cancelCurrentGeneration")}
        >
          {streamBusy ? (
            <LoadingSpinner size="sm" className="shrink-0" />
          ) : (
            <Square size={11} className="shrink-0" />
          )}
          <span className="truncate">{t("writing.cancelGeneration")}</span>
        </button>
      )}
    </div>
  </div>
  );
};
