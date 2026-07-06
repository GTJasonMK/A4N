import React from "react";
import { Check, Edit2, FileText, Loader2, RotateCw, Trash2, type LucideIcon } from "lucide-react";
import { useContextMenu } from "../../components/ui/ContextMenu";

/**
 * 章节卡片
 *
 * 布局：
 *   ┌─────────────────────────────────────┐
 *   │ [状态图标]  N. 章节标题（可换行）        │
 *   │            字数 · 状态                  │
 *   └─────────────────────────────────────┘
 *
 * 状态：
 *   - 选中：ACCENT_PALE 背景 + 2px 主色边框（不是整块填充）
 *   - 悬停：浅灰背景 + 1px 边框
 *   - 普通：透明背景 + 浅边框
 *
 * 右键菜单：编辑大纲 / 重新生成大纲 / 清空章节数据
 */

interface ChapterCardProps {
  chapterNumber: number;
  title: string | null | undefined;
  wordCount?: number | null;
  status?: string | null;
  hasContent: boolean;
  hasDraft?: boolean;
  selected: boolean;
  onClick: () => void;
  onEditOutline?: () => void;
  onRegenerateOutline?: () => void;
  onClearChapter?: () => void;
}

const STATUS_ICON: Record<string, { Icon: LucideIcon; tone: string; title: string }> = {
  completed: { Icon: Check, tone: "text-success", title: "已完成" },
  draft: { Icon: FileText, tone: "text-warning", title: "草稿" },
  generating: { Icon: Loader2, tone: "text-book-primary animate-spin", title: "生成中" },
  imported: { Icon: FileText, tone: "text-info", title: "已导入" },
};

export const ChapterCard: React.FC<ChapterCardProps> = ({
  chapterNumber,
  title,
  wordCount,
  status,
  hasContent,
  hasDraft = false,
  selected,
  onClick,
  onEditOutline,
  onRegenerateOutline,
  onClearChapter,
}) => {
  const menu = useContextMenu();

  const effectiveStatus = status ?? (hasContent ? "completed" : null);
  const iconInfo = effectiveStatus ? STATUS_ICON[effectiveStatus] : null;
  const StatusIcon = iconInfo?.Icon;

  const containerClass = selected
    ? "border border-book-primary bg-book-primary/10"
    : "border border-book-border/50 bg-book-bg-paper/45 hover:border-book-primary/50 hover:bg-book-bg-tertiary/45";

  const metaParts: string[] = [];
  if (wordCount != null) metaParts.push(`${wordCount.toLocaleString()} 字`);
  if (iconInfo) metaParts.push(iconInfo.title);
  if (status && !iconInfo) metaParts.push(status);

  return (
    <button
      type="button"
      onClick={onClick}
      onContextMenu={menu.handle([
        { label: "编辑章纲", icon: <Edit2 size={12} />, onClick: () => onEditOutline?.(), disabled: !onEditOutline },
        {
          label: "重新生成大纲",
          icon: <RotateCw size={12} />,
          onClick: () => onRegenerateOutline?.(),
          disabled: !onRegenerateOutline,
        },
        { divider: true },
        {
          label: "清空本章数据",
          icon: <Trash2 size={12} />,
          onClick: () => onClearChapter?.(),
          disabled: !onClearChapter,
          danger: true,
        },
      ])}
      className={`w-full text-left rounded-sm px-2 py-1.5 transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary/35 ${containerClass}`}
    >
      <div className="flex items-start gap-2">
        {StatusIcon && (
          <span className={`shrink-0 mt-0.5 ${iconInfo.tone}`} title={iconInfo.title}>
            <StatusIcon size={11} />
          </span>
        )}
        <div className="flex-1 min-w-0">
          <div className="flex items-baseline gap-1 text-xs">
            <span className="font-mono text-book-text-muted shrink-0">{chapterNumber}.</span>
            <span
              className={`flex-1 min-w-0 truncate ${
                selected ? "text-book-text-main font-semibold" : "text-book-text-sub"
              }`}
            >
              {title?.trim()}
              {hasDraft && (
                <span className="ml-1 text-book-accent" title="存在本地草稿">
                  *
                </span>
              )}
            </span>
          </div>
          {metaParts.length > 0 && (
            <div className="text-[10px] text-book-text-muted mt-0.5 truncate">
              {metaParts.join(" · ")}
            </div>
          )}
        </div>
      </div>
    </button>
  );
};
