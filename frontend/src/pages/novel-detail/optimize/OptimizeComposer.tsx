import { useLayoutEffect, useRef } from "react";
import { Eye, Loader2, Search, Send, ShieldCheck, Square, Wand2 } from "lucide-react";
import type { ChapterData } from "../../../api/chapter";
import type { ProposeOptimizationPatchesResponse } from "../../../api/optimize";
import { isImeCompositionActive } from "../../../utils/ime";
import { PendingPatchProposalPanel } from "./PendingPatchProposalPanel";

interface OptimizeComposerProps {
  activeChapter: ChapterData | null;
  automaticWriteback: boolean;
  canProposePatches: boolean;
  cancellingOptimize: boolean;
  input: string;
  pendingPatchProposal: ProposeOptimizationPatchesResponse | null;
  reviewInEditor: boolean;
  taskRunning: boolean;
  onAnalyze: () => void;
  onApplyPendingPatches: () => void;
  onCancelTask: () => void;
  onDiscardPendingPatches: () => void;
  onInputChange: (value: string) => void;
  onPreviewAnalyze: () => void;
  onPreviewMessage: () => void;
  onPreviewProposePatches: () => void;
  onProposePatches: () => void;
  onSubmit: () => void;
  onToggleAutomaticWriteback: () => void;
}

export function OptimizeComposer({
  activeChapter,
  automaticWriteback,
  canProposePatches,
  cancellingOptimize,
  input,
  pendingPatchProposal,
  reviewInEditor,
  taskRunning,
  onAnalyze,
  onApplyPendingPatches,
  onCancelTask,
  onDiscardPendingPatches,
  onInputChange,
  onPreviewAnalyze,
  onPreviewMessage,
  onPreviewProposePatches,
  onProposePatches,
  onSubmit,
  onToggleAutomaticWriteback,
}: OptimizeComposerProps) {
  const inputRef = useRef<HTMLTextAreaElement | null>(null);
  const composingRef = useRef(false);
  const lastCompositionEndRef = useRef(0);

  useLayoutEffect(() => {
    const inputElement = inputRef.current;
    if (!inputElement) return;
    if (composingRef.current) return;
    if (!input) {
      inputElement.style.height = "";
      return;
    }
    inputElement.style.height = "auto";
    inputElement.style.height = `${inputElement.scrollHeight}px`;
  }, [input]);

  const hasInstruction = input.trim().length > 0;
  const canSubmitInstruction = hasInstruction && !taskRunning && !!activeChapter;
  const canUsePatches = canSubmitInstruction && canProposePatches;

  return (
    <form
      className="min-w-0 shrink-0 border-t border-book-border bg-book-bg-paper px-2.5 py-2"
      onSubmit={(event) => {
        event.preventDefault();
        const nativeEvent = event.nativeEvent as SubmitEvent;
        const submitter = nativeEvent.submitter;
        if (
          submitter instanceof HTMLElement &&
          submitter.dataset.optimizeSubmit === "send"
        ) {
          onSubmit();
        }
      }}
      onKeyDown={(event) => {
        if (
          event.key === "Enter" &&
          !event.shiftKey &&
          !isImeCompositionActive(
            event.nativeEvent,
            composingRef.current,
            lastCompositionEndRef.current,
          )
        ) {
          event.preventDefault();
          onSubmit();
        }
      }}
    >
      <div className="mb-2 flex min-w-0 flex-wrap gap-1.5">
        <button
          type="button"
          onClick={onToggleAutomaticWriteback}
          disabled={taskRunning || !activeChapter}
          className={`inline-flex h-9 min-w-0 items-center gap-1.5 rounded border px-2 text-[11px] font-semibold disabled:cursor-not-allowed disabled:opacity-50 sm:h-7 ${
            automaticWriteback
              ? "border-warning bg-warning-bg text-warning"
              : "border-book-border bg-book-bg-tertiary text-book-text-sub hover:bg-book-bg-paper hover:text-book-text-main"
          }`}
          title={automaticWriteback ? "Agent 完成后批量审阅补丁" : "Agent 每处补丁逐条审阅"}
        >
          <ShieldCheck size={12} />
          {automaticWriteback ? "自动模式已开启" : "自动模式"}
        </button>
        <button
          type="button"
          onClick={onAnalyze}
          disabled={!canSubmitInstruction}
          className="inline-flex h-9 min-w-0 items-center gap-1.5 rounded border border-book-border bg-book-bg-paper px-2 text-[11px] font-semibold text-book-text-sub hover:bg-book-bg-tertiary hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 sm:h-7"
          title="执行内容优化分析"
        >
          <Search size={12} />
          分析
        </button>
        <button
          type="button"
          onClick={onPreviewAnalyze}
          disabled={!canSubmitInstruction}
          className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded border border-book-border bg-book-bg-paper text-book-text-sub hover:bg-book-bg-tertiary hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 sm:h-7 sm:w-7"
          title="预览分析模型请求"
        >
          <Eye size={12} />
        </button>
        <button
          type="button"
          onClick={onProposePatches}
          disabled={!canUsePatches}
          className="inline-flex h-9 min-w-0 items-center gap-1.5 rounded border border-book-border bg-book-bg-paper px-2 text-[11px] font-semibold text-book-text-sub hover:bg-book-bg-tertiary hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 sm:h-7"
          title="根据分析问题生成局部修复补丁"
        >
          <Wand2 size={12} />
          补丁
        </button>
        <button
          type="button"
          onClick={onPreviewProposePatches}
          disabled={!canUsePatches}
          className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded border border-book-border bg-book-bg-paper text-book-text-sub hover:bg-book-bg-tertiary hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 sm:h-7 sm:w-7"
          title="预览局部补丁模型请求"
        >
          <Eye size={12} />
        </button>
      </div>

      {pendingPatchProposal ? (
        <PendingPatchProposalPanel
          proposal={pendingPatchProposal}
          automaticWriteback={automaticWriteback}
          reviewInEditor={reviewInEditor}
          taskRunning={taskRunning}
          onApply={onApplyPendingPatches}
          onDiscard={onDiscardPendingPatches}
        />
      ) : null}

      <div className="flex min-w-0 items-end gap-2">
        <textarea
          ref={inputRef}
          value={input}
          onCompositionStart={() => {
            composingRef.current = true;
          }}
          onCompositionEnd={(event) => {
            lastCompositionEndRef.current = Date.now();
            onInputChange(event.currentTarget.value);
            window.setTimeout(() => {
              composingRef.current = false;
            }, 0);
          }}
          onChange={(event) => onInputChange(event.target.value)}
          rows={1}
          maxLength={1600}
          disabled={taskRunning || !activeChapter}
          placeholder="输入问题或优化要求"
          className="pyqt-input h-10 max-h-32 min-h-10 min-w-0 flex-1 resize-none overflow-y-auto text-xs leading-4 sm:h-8 sm:min-h-8"
        />
        {taskRunning ? (
          <button
            type="button"
            onClick={onCancelTask}
            disabled={cancellingOptimize}
            className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded border border-warning bg-warning-bg text-warning hover:brightness-95 disabled:cursor-not-allowed disabled:opacity-60 sm:h-8 sm:w-8"
            title={cancellingOptimize ? "正在停止" : "停止当前 Agent 任务"}
          >
            {cancellingOptimize ? (
              <Loader2 size={15} className="animate-spin" />
            ) : (
              <Square size={14} />
            )}
          </button>
        ) : (
          <>
            <button
              type="button"
              onClick={onPreviewMessage}
              disabled={!canSubmitInstruction}
              className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded border border-book-border bg-book-bg-paper text-book-text-sub hover:bg-book-bg-tertiary hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 sm:h-8 sm:w-8"
              title="预览对话模型请求"
            >
              <Eye size={14} />
            </button>
            <button
              type="submit"
              data-optimize-submit="send"
              disabled={!canSubmitInstruction}
              className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded border border-book-primary bg-book-primary text-white hover:brightness-95 disabled:cursor-not-allowed disabled:border-book-border disabled:bg-book-bg-tertiary disabled:text-book-text-muted sm:h-8 sm:w-8"
              title="发送给内容优化 Agent"
            >
              <Send size={15} />
            </button>
          </>
        )}
      </div>
    </form>
  );
}
