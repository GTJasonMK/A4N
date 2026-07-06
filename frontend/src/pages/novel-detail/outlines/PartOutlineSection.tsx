import { Eye, Loader2, SlidersHorizontal } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { nonEmptyText } from "../../../utils/text";
import { EmptyHint } from "../shared";
import { parseNumberInput, type NumberInputValue } from "./outlineTabUtils";
import type { PartOutline } from "../../../api/outline";

interface PartOutlineSectionProps {
  parts: PartOutline[];
  canEdit: boolean;
  partCount: NumberInputValue;
  chaptersPerPart: NumberInputValue;
  defaultChaptersPerPart: NumberInputValue;
  generating: boolean;
  disabled?: boolean;
  onPartCountChange: (value: NumberInputValue) => void;
  onChaptersPerPartChange: (value: NumberInputValue) => void;
  onGenerate: () => void;
  onPreviewGenerate: () => void;
  onOpenAdvanced: () => void;
  onOpenPart: (part: PartOutline) => void;
}

export function PartOutlineSection({
  parts,
  canEdit,
  partCount,
  chaptersPerPart,
  defaultChaptersPerPart,
  generating,
  disabled = false,
  onPartCountChange,
  onChaptersPerPartChange,
  onGenerate,
  onPreviewGenerate,
  onOpenAdvanced,
  onOpenPart,
}: PartOutlineSectionProps) {
  return (
    <BookCard className="p-5">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
        <h3 className="font-serif text-sm text-book-text-main">
          分部大纲 <span className="text-xs text-book-text-muted">({parts.length})</span>
        </h3>
        {canEdit && (
          <div className="flex items-center gap-2 text-xs">
            <label className="text-book-text-muted">生成部数</label>
            <input
              type="number"
              min={1}
              step={1}
              value={partCount}
              disabled={disabled}
              onChange={(event) => onPartCountChange(parseNumberInput(event.target.value))}
              className="pyqt-input !px-2 !py-1 w-16"
            />
            <label className="text-book-text-muted">每部章数</label>
            <input
              type="number"
              min={1}
              step={1}
              placeholder={defaultChaptersPerPart === "" ? "自动" : undefined}
              value={chaptersPerPart}
              disabled={disabled}
              onChange={(event) => onChaptersPerPartChange(parseNumberInput(event.target.value))}
              className="pyqt-input !px-2 !py-1 w-16"
            />
            <BookButton variant="primary" onClick={onGenerate} disabled={disabled || generating}>
              {generating ? (
                <span className="inline-flex items-center gap-2">
                  <Loader2 size={14} className="animate-spin" /> 生成中…
                </span>
              ) : parts.length > 0 ? (
                "继续生成分部"
              ) : (
                "AI 生成分部"
              )}
            </BookButton>
            <BookButton variant="secondary" onClick={onPreviewGenerate} disabled={disabled || generating}>
              <span className="inline-flex items-center gap-1.5">
                <Eye size={12} /> 预览提示词
              </span>
            </BookButton>
            <BookButton variant="secondary" size="sm" onClick={onOpenAdvanced} disabled={disabled}>
              <span className="inline-flex items-center gap-1.5">
                <SlidersHorizontal size={12} /> 高级工具
              </span>
            </BookButton>
          </div>
        )}
      </div>
      {parts.length === 0 ? (
        <EmptyHint text="尚未生成分部大纲。" />
      ) : (
        <ul className="space-y-1.5">
          {parts.map((part) => (
            <li
              key={part.id}
              role="button"
              tabIndex={0}
              onClick={() => onOpenPart(part)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onOpenPart(part);
                }
              }}
              className="flex cursor-pointer items-start gap-3 rounded-sm border border-book-border bg-book-bg-paper px-3 py-2 transition-colors hover:border-book-primary/60 hover:bg-book-bg-tertiary/40"
              style={{ minHeight: "48px" }}
              title="查看分部大纲详情"
            >
              <span
                className="flex h-9 w-9 shrink-0 items-center justify-center rounded-sm border border-book-primary/40 bg-book-primary/10 font-mono text-xs font-bold text-book-primary"
                title={`第 ${part.part_number} 部`}
              >
                P{part.part_number}
              </span>
              <div className="min-w-0 flex-1">
                <div className="mb-0.5 flex items-center gap-2">
                  <span className="truncate text-sm font-semibold text-book-text-main">
                    {nonEmptyText(part.title) ?? `第 ${part.part_number} 部`}
                  </span>
                </div>
                {nonEmptyText(part.summary) ? (
                  <p className="line-clamp-2 text-[11px] leading-relaxed text-book-text-muted">
                    {nonEmptyText(part.summary)}
                  </p>
                ) : null}
              </div>
            </li>
          ))}
        </ul>
      )}
    </BookCard>
  );
}
