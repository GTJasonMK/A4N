import React from "react";
import { FileText, Loader2, type LucideIcon } from "lucide-react";
import type { ChapterMaterialSyncState } from "../../api/chapter";
import { getOperationStatusLabel } from "../../utils/workflowPresentation";
import { formatNumber, useI18n, type MessageKey } from "../../i18n";

/** 写作桌侧栏中的连续章节行。 */

interface ChapterCardProps {
  chapterNumber: number;
  title: string | null | undefined;
  wordCount?: number | null;
  status?: string | null;
  materialSync?: ChapterMaterialSyncState | null;
  hasDraft?: boolean;
  selected: boolean;
  onSelect: (chapterNumber: number) => void | Promise<unknown>;
}

const STATUS_ICON: Record<string, { Icon: LucideIcon; tone: string; title: MessageKey }> = {
  draft: { Icon: FileText, tone: "text-warning", title: "writing.chapterStatusDraft" },
  generating: { Icon: Loader2, tone: "text-book-primary animate-spin", title: "writing.chapterStatusGenerating" },
  imported: { Icon: FileText, tone: "text-info", title: "writing.chapterStatusImported" },
};

const materialSyncLabel = (state: ChapterMaterialSyncState | null | undefined): MessageKey | null => {
  if (!state) return null;
  const statuses = Object.values(state);
  if (statuses.includes("failed")) return "writing.materialSyncFailed";
  if (statuses.includes("running")) return "writing.materialSyncRunning";
  if (statuses.includes("pending")) return "writing.materialSyncPending";
  if (statuses.includes("needs_analysis")) return "writing.materialNeedsAnalysis";
  if (statuses.includes("not_analyzed")) return "writing.materialNotAnalyzed";
  if (statuses.includes("unknown")) return "writing.materialNeedsReview";
  return "writing.materialSynced";
};

export const ChapterCard = React.memo(function ChapterCard({
  chapterNumber,
  title,
  wordCount,
  status,
  materialSync,
  hasDraft = false,
  selected,
  onSelect,
}: ChapterCardProps) {
  const { t, locale } = useI18n();
  const iconInfo = status ? STATUS_ICON[status] : null;
  const StatusIcon = iconInfo?.Icon;

  const containerClass = selected
    ? "bg-book-primary/8"
    : "bg-transparent hover:bg-book-bg-tertiary/55";

  const metaParts: string[] = [];
  if (wordCount != null) metaParts.push(t("writing.chapterWordCount", { count: formatNumber(wordCount, locale) }));
  if (iconInfo) metaParts.push(t(iconInfo.title));
  if (status && status !== "completed" && !iconInfo) {
    metaParts.push(getOperationStatusLabel(status, locale));
  }
  const syncLabel = materialSyncLabel(materialSync);
  if (syncLabel) metaParts.push(t(syncLabel));

  return (
    <button
      type="button"
      onClick={() => void onSelect(chapterNumber)}
      aria-current={selected ? "page" : undefined}
      className={`a4n-chapter-card w-full px-3 py-3 text-left transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary ${containerClass}`}
      data-selected={selected ? "true" : undefined}
    >
      <div className="flex items-start gap-2">
        {StatusIcon && (
          <span className={`shrink-0 mt-0.5 ${iconInfo.tone}`} title={t(iconInfo.title)}>
            <StatusIcon size={14} />
          </span>
        )}
        <div className="flex-1 min-w-0">
          <div className="flex min-w-0 items-center gap-1.5 text-body">
            <span
              className={`min-w-0 flex-1 truncate ${
                selected ? "text-book-text-main font-semibold" : "text-book-text-sub"
              }`}
            >
              {t("writing.chapterLabel", { number: chapterNumber })}{title?.trim() ? ` · ${title.trim()}` : ""}
            </span>
            {hasDraft ? (
              <span className="a4n-draft-badge shrink-0" title={t("writing.localDraft")}>
                {t("writing.localDraft")}
              </span>
            ) : null}
          </div>
          {metaParts.length > 0 && (
            <div className="mt-0.5 truncate text-small text-book-text-muted">
              {metaParts.join(" · ")}
            </div>
          )}
        </div>
      </div>
    </button>
  );
});
