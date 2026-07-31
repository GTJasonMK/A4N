import React from "react";
import { Play, Square } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";

interface StreamControlBarProps {
  activeNumber: number | null;
  streamTaskId: string | null;
  streamBusy: boolean;
  streamMessage?: string | null;
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
  canStart,
  startDisabledReason,
  resetBusy,
  onStart,
  onCancel,
}) => (
  <div className="min-w-0 space-y-2 text-xs">
    {resetBusy ? (
      <div className="flex min-w-0 items-center gap-2 text-small text-book-text-muted">
        <LoadingSpinner size="sm" className="shrink-0" />
        <span className="truncate">正在重置生成数据…</span>
      </div>
    ) : streamTaskId ? (
      <div className="flex min-w-0 items-center gap-2 text-small text-book-text-muted">
        <LoadingSpinner size="sm" className="shrink-0" />
        <span className="truncate">
          {activeNumber != null ? `正在生成第 ${activeNumber} 章…` : "正在生成正文…"}
        </span>
      </div>
    ) : null}
    {streamTaskId && (
      <details className="text-caption text-book-text-muted">
        <summary className="w-fit cursor-pointer transition-colors hover:text-book-text-main">
          诊断信息
        </summary>
        <div className="mt-1 break-all border-l border-book-border/40 pl-2 font-mono">
          任务编号：{streamTaskId}
        </div>
      </details>
    )}
    {streamMessage ? (
      <div className="flex min-w-0 items-center gap-1.5 rounded-lg bg-book-bg-tertiary/55 px-2.5 py-2 text-small text-book-text-muted">
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
              <span className="truncate">开始生成</span>
            </span>
          )}
        </BookButton>
      ) : (
        <button
          type="button"
          onClick={onCancel}
          disabled={streamBusy}
          className="pyqt-btn-secondary inline-flex min-w-0 flex-1 items-center justify-center gap-1 !min-h-9 !border-error/50 !px-2 text-small !text-error sm:!min-h-[28px]"
          title="取消当前章节生成任务"
        >
          {streamBusy ? (
            <LoadingSpinner size="sm" className="shrink-0" />
          ) : (
            <Square size={11} className="shrink-0" />
          )}
          <span className="truncate">取消生成</span>
        </button>
      )}
    </div>
  </div>
);
