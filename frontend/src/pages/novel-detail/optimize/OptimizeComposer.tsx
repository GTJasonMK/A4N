import { useLayoutEffect, useRef } from "react";
import { Eye, Search, Send, ShieldCheck, Square, Wand2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { ChapterData } from "../../../api/chapter";
import type { ProposeOptimizationPatchesResponse } from "../../../api/optimize";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
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
  const toolItems: DropdownItem[] = [
    {
      label: "分析当前要求",
      icon: <Search size={12} />,
      onClick: onAnalyze,
      disabled: !canSubmitInstruction,
    },
    {
      label: "生成局部补丁",
      icon: <Wand2 size={12} />,
      onClick: onProposePatches,
      disabled: !canUsePatches,
    },
    { type: "divider" },
    {
      label: "查看对话请求",
      icon: <Eye size={12} />,
      onClick: onPreviewMessage,
      disabled: !canSubmitInstruction,
    },
    {
      label: "查看分析请求",
      icon: <Eye size={12} />,
      onClick: onPreviewAnalyze,
      disabled: !canSubmitInstruction,
    },
    {
      label: "查看补丁请求",
      icon: <Eye size={12} />,
      onClick: onPreviewProposePatches,
      disabled: !canUsePatches,
    },
  ];
  const showReviewMode = canProposePatches && pendingPatchProposal == null;

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
      {showReviewMode ? (
        <div className="mb-2 flex min-w-0 items-center gap-2">
          <span className="min-w-0 flex-1 text-small text-book-text-muted">补丁处理方式</span>
          <SegmentedControl
            ariaLabel="补丁审阅方式"
            value={automaticWriteback ? "batch" : "single"}
            onChange={(mode) => {
              if ((mode === "batch") !== automaticWriteback) onToggleAutomaticWriteback();
            }}
            options={[
              {
                id: "single",
                label: "逐条审阅",
                icon: <ShieldCheck size={11} />,
                disabled: taskRunning || !activeChapter,
              },
              {
                id: "batch",
                label: "批量审阅",
                disabled: taskRunning || !activeChapter,
              },
            ]}
          />
        </div>
      ) : null}

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
        <Dropdown label="工具" items={toolItems} />
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
            className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-warning/55 bg-warning-bg text-warning transition-[transform,filter] active:scale-[0.96] hover:brightness-95 disabled:cursor-not-allowed disabled:opacity-60 sm:h-8 sm:w-8"
            title={cancellingOptimize ? "正在停止" : "停止当前 Agent 任务"}
          >
            {cancellingOptimize ? (
              <LoadingSpinner size="sm" />
            ) : (
              <Square size={14} />
            )}
          </button>
        ) : (
          <button
            type="submit"
            data-optimize-submit="send"
            disabled={!canSubmitInstruction}
            className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-book-primary bg-book-primary text-white shadow-[var(--shadow-soft)] transition-[transform,filter] active:scale-[0.96] hover:brightness-95 disabled:cursor-not-allowed disabled:border-book-border disabled:bg-book-bg-tertiary disabled:text-book-text-muted disabled:shadow-none sm:h-8 sm:w-8"
            title="发送给内容优化"
          >
            <Send size={15} />
          </button>
        )}
      </div>
    </form>
  );
}
