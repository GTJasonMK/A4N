import React from "react";
import { FileText, Loader2, type LucideIcon } from "lucide-react";
import { getOperationStatusLabel } from "../../utils/workflowPresentation";

/** 写作桌侧栏中的连续章节行。 */

interface ChapterCardProps {
  chapterNumber: number;
  title: string | null | undefined;
  wordCount?: number | null;
  status?: string | null;
  hasDraft?: boolean;
  selected: boolean;
  onClick: () => void;
}

const STATUS_ICON: Record<string, { Icon: LucideIcon; tone: string; title: string }> = {
  draft: { Icon: FileText, tone: "text-warning", title: "草稿" },
  generating: { Icon: Loader2, tone: "text-book-primary animate-spin", title: "生成中" },
  imported: { Icon: FileText, tone: "text-info", title: "已导入" },
};

export const ChapterCard: React.FC<ChapterCardProps> = ({
  chapterNumber,
  title,
  wordCount,
  status,
  hasDraft = false,
  selected,
  onClick,
}) => {
  const iconInfo = status ? STATUS_ICON[status] : null;
  const StatusIcon = iconInfo?.Icon;

  const containerClass = selected
    ? "bg-book-primary/10 shadow-[inset_2px_0_0_rgb(var(--color-primary))]"
    : "bg-transparent hover:bg-book-bg-tertiary/55";

  const metaParts: string[] = [];
  if (wordCount != null) metaParts.push(`${wordCount.toLocaleString()} 字`);
  if (iconInfo) metaParts.push(iconInfo.title);
  if (status && status !== "completed" && !iconInfo) {
    metaParts.push(getOperationStatusLabel(status));
  }

  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={selected ? "page" : undefined}
      className={`w-full px-3 py-2.5 text-left transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary/35 ${containerClass}`}
    >
      <div className="flex items-start gap-2">
        {StatusIcon && (
          <span className={`shrink-0 mt-0.5 ${iconInfo.tone}`} title={iconInfo.title}>
            <StatusIcon size={11} />
          </span>
        )}
        <div className="flex-1 min-w-0">
          <div className="flex items-baseline text-xs">
            <span
              className={`min-w-0 flex-1 truncate ${
                selected ? "text-book-text-main font-semibold" : "text-book-text-sub"
              }`}
            >
              第 {chapterNumber} 章{title?.trim() ? ` · ${title.trim()}` : ""}
              {hasDraft && (
                <span className="ml-1 text-book-accent" title="存在本地草稿">
                  *
                </span>
              )}
            </span>
          </div>
          {metaParts.length > 0 && (
            <div className="mt-0.5 truncate text-[11px] text-book-text-muted">
              {metaParts.join(" · ")}
            </div>
          )}
        </div>
      </div>
    </button>
  );
};
