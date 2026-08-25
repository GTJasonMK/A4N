import { CheckCircle2 } from "lucide-react";
import type { OptimizePatchSet } from "../../../api/optimize";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { useI18n } from "../../../i18n";

interface PendingPatchProposalPanelProps {
  proposal: OptimizePatchSet;
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
  const { t } = useI18n();
  if (reviewInEditor) {
    return (
      <div className="a4n-flat-surface mb-2 flex min-w-0 items-start gap-2.5 px-3 py-2.5 text-xs">
        <CheckCircle2 size={13} className="mt-0.5 shrink-0 text-book-primary" />
        <div className="min-w-0 flex-1">
          <div className="font-semibold text-book-text-main">
            {t("optimize.pendingPatchReview", { count: proposal.patches.length })}
          </div>
          <div className="mt-0.5 truncate text-small text-book-text-sub" title={proposal.summary}>
            {proposal.summary}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="a4n-flat-surface mb-2 min-w-0 px-3 py-2.5 text-xs">
      <div className="flex min-w-0 flex-wrap items-start gap-2">
        <div className="min-w-0 flex-1 basis-36">
          <div className="font-semibold text-book-text-main">
            {t("optimize.pendingPatches", { count: proposal.patches.length })}
          </div>
          <div className="mt-0.5 whitespace-pre-wrap break-words text-book-text-sub">
            {proposal.summary}
          </div>
        </div>
        <div className="flex min-w-0 items-center gap-2">
          <Dropdown
            items={[
              {
                label: t("optimize.discardPatches"),
                danger: true,
                onClick: onDiscard,
                disabled: taskRunning,
              },
            ]}
          />
          <BookButton
            variant="primary"
            size="sm"
            onClick={onApply}
            disabled={taskRunning}
            className="!text-small"
            title={t("optimize.confirmApplyPatches")}
          >
            <CheckCircle2 size={12} />
            {automaticWriteback ? t("optimize.acceptAll") : t("optimize.applyPatches")}
          </BookButton>
        </div>
      </div>
    </div>
  );
}
