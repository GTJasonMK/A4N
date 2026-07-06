import React from "react";
import {
  Loader2,
  Play,
  RotateCcw,
  Square,
  Zap,
} from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";

interface StreamControlBarProps {
  activeNumber: number | null;
  streamTaskId: string | null;
  streamBusy: boolean;
  streamMessage?: string | null;
  canStart: boolean;
  startDisabledReason?: string | null;
  resetBusy: boolean;
  canReset: boolean;
  resetDisabledReason?: string | null;
  onStart: () => void;
  onCancel: () => void;
  onReset: () => void;
}

/**
 * 流式生成控制条：启动 / 取消。
 */
export const StreamControlBar: React.FC<StreamControlBarProps> = ({
  activeNumber,
  streamTaskId,
  streamBusy,
  streamMessage,
  canStart,
  startDisabledReason,
  resetBusy,
  canReset,
  resetDisabledReason,
  onStart,
  onCancel,
  onReset,
}) => (
  <div className="min-w-0 space-y-2 rounded border border-book-border/50 bg-book-bg-paper/30 p-2 text-xs">
    <div className="flex min-w-0 flex-wrap items-center gap-1.5">
      <span className="inline-flex min-w-0 flex-1 items-center gap-1.5 text-book-text-muted">
        <Zap size={11} className="shrink-0" />
        <span className="truncate">流式生成（可控）</span>
      </span>
      {streamTaskId && (
        <span className="pyqt-pill !text-[10px] !border-book-primary !text-book-primary">
          运行中
        </span>
      )}
    </div>
    {streamTaskId && (
      <div className="break-all font-mono text-[10px] text-book-text-muted">
        task: {streamTaskId}
      </div>
    )}
    {streamMessage && (
      <div className="flex min-w-0 items-center gap-1.5 rounded border border-book-border/40 bg-book-bg-paper/50 px-2 py-1 text-[11px] text-book-text-muted">
        {streamTaskId ? (
          <Loader2 size={10} className="shrink-0 animate-spin" />
        ) : (
          <Zap size={10} className="shrink-0" />
        )}
        <span className="min-w-0 break-words">{streamMessage}</span>
      </div>
    )}
    <div className="flex min-w-0 gap-2">
      {!streamTaskId ? (
        <>
          <BookButton
            variant="secondary"
            onClick={onStart}
            disabled={streamBusy || resetBusy || !activeNumber || !canStart}
            className="min-w-0 flex-1 !px-2 !whitespace-normal"
            title={startDisabledReason === null ? undefined : startDisabledReason}
          >
            {streamBusy ? (
              <Loader2 size={11} className="animate-spin" />
            ) : (
              <span className="inline-flex min-w-0 items-center gap-1">
                <Play size={11} className="shrink-0" />
                <span className="truncate">启动流式</span>
              </span>
            )}
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={onReset}
            disabled={streamBusy || resetBusy || !canReset}
            className="min-w-0 flex-1 !border-error/50 !px-2 !text-error !whitespace-normal"
            title={resetDisabledReason ?? "重置项目生成数据"}
          >
            {resetBusy ? (
              <Loader2 size={11} className="animate-spin" />
            ) : (
              <span className="inline-flex min-w-0 items-center gap-1">
                <RotateCcw size={11} className="shrink-0" />
                <span className="truncate">重置</span>
              </span>
            )}
          </BookButton>
        </>
      ) : (
        <button
          type="button"
          onClick={onCancel}
          disabled={streamBusy}
          className="pyqt-btn-secondary inline-flex min-w-0 flex-1 items-center justify-center gap-1 !min-h-9 !border-error/50 !px-2 text-[11px] !text-error sm:!min-h-[28px]"
          title="取消当前章节生成任务"
        >
          {streamBusy ? (
            <Loader2 size={11} className="shrink-0 animate-spin" />
          ) : (
            <Square size={11} className="shrink-0" />
          )}
          <span className="truncate">取消生成</span>
        </button>
      )}
    </div>
  </div>
);
