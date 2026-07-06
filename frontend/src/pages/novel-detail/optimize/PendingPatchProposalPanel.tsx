import { CheckCircle2 } from "lucide-react";
import type { ProposeOptimizationPatchesResponse } from "../../../api/optimize";

interface PendingPatchProposalPanelProps {
  proposal: ProposeOptimizationPatchesResponse;
  automaticWriteback: boolean;
  reviewInEditor: boolean;
  taskRunning: boolean;
  onApply: () => void;
  onDiscard: () => void;
}

export function PendingPatchProposalPanel({
  proposal,
  automaticWriteback,
  reviewInEditor,
  taskRunning,
  onApply,
  onDiscard,
}: PendingPatchProposalPanelProps) {
  return (
    <div className="mb-2 min-w-0 rounded border border-book-border bg-book-bg-tertiary px-2.5 py-2 text-xs">
      <div className="flex min-w-0 flex-wrap items-start gap-2">
        <div className="min-w-0 flex-1 basis-36">
          <div className="font-semibold text-book-text-main">
            待应用补丁：{proposal.patches.length} 个
          </div>
          <div className="mt-0.5 whitespace-pre-wrap break-words text-book-text-sub">
            {proposal.summary}
          </div>
        </div>
        {!reviewInEditor ? (
          <div className="flex min-w-0 flex-wrap items-center gap-1">
            <button
              type="button"
              onClick={onApply}
              disabled={taskRunning}
              className="inline-flex h-9 items-center gap-1 rounded border border-book-primary bg-book-primary px-1.5 text-[11px] font-semibold text-white hover:brightness-95 disabled:cursor-not-allowed disabled:opacity-50 sm:h-7"
              title="确认后应用这些局部补丁"
            >
              <CheckCircle2 size={12} />
              {automaticWriteback ? "接受全部" : "应用补丁"}
            </button>
            <button
              type="button"
              onClick={onDiscard}
              disabled={taskRunning}
              className="inline-flex h-9 items-center rounded border border-book-border bg-book-bg-paper px-1.5 text-[11px] text-book-text-sub hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 sm:h-7"
            >
              丢弃
            </button>
          </div>
        ) : null}
      </div>
    </div>
  );
}
