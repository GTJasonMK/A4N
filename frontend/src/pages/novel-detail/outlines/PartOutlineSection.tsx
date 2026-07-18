import { useState } from "react";
import { Eye, Loader2, SlidersHorizontal } from "lucide-react";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
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
  const [generationOpen, setGenerationOpen] = useState(parts.length === 0);
  const hasParts = parts.length > 0;
  const generationActionLabel = hasParts ? "追加分部" : "生成分部";

  return (
    <section className="space-y-2.5">
      <header className="flex items-start justify-between gap-3 px-1">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold text-book-text-main">分部大纲</h2>
          <p className="mt-0.5 text-[11px] leading-relaxed text-book-text-muted">
            {hasParts ? `共 ${parts.length} 部，按故事阶段组织章节` : "先规划故事的阶段与章节范围"}
          </p>
        </div>
        {canEdit ? (
          <BookIconButton
            label="打开大纲高级工具"
            onClick={onOpenAdvanced}
            disabled={disabled}
            variant="ghost"
            size="sm"
          >
            <SlidersHorizontal size={13} />
          </BookIconButton>
        ) : null}
      </header>

      {canEdit ? (
        <details
          className="a4n-flat-surface group overflow-hidden px-4 py-2.5"
          open={generationOpen}
          onToggle={(event) => setGenerationOpen(event.currentTarget.open)}
        >
          <summary className="flex cursor-pointer items-center justify-between gap-3 py-1 text-xs font-medium text-book-text-sub">
            <span>{generationActionLabel}</span>
            <span className="text-[10px] font-normal text-book-text-muted">设置部数与章节范围</span>
          </summary>
          <div className="mt-3 grid grid-cols-1 gap-3 border-t border-book-border/35 pt-3 sm:grid-cols-2">
            <label className="flex min-w-0 flex-col gap-1 text-[11px] text-book-text-muted">
              {hasParts ? "本次追加部数" : "生成部数"}
              <input
                type="number"
                min={1}
                step={1}
                value={partCount}
                disabled={disabled}
                onChange={(event) => onPartCountChange(parseNumberInput(event.target.value))}
                className="pyqt-input w-full"
              />
            </label>
            <label className="flex min-w-0 flex-col gap-1 text-[11px] text-book-text-muted">
              每部章节数
              <input
                type="number"
                min={1}
                step={1}
                placeholder={defaultChaptersPerPart === "" ? "自动规划" : undefined}
                value={chaptersPerPart}
                disabled={disabled}
                onChange={(event) =>
                  onChaptersPerPartChange(parseNumberInput(event.target.value))
                }
                className="pyqt-input w-full"
              />
            </label>
          </div>
          <div className="mt-3 flex items-center justify-end gap-2">
            {!generating ? (
              <Dropdown
                items={[
                  {
                    label: hasParts ? "查看追加请求" : "查看生成请求",
                    icon: <Eye size={12} />,
                    onClick: onPreviewGenerate,
                    disabled,
                  },
                ]}
              />
            ) : null}
            <BookButton
              variant="primary"
              size="sm"
              onClick={onGenerate}
              disabled={disabled || generating}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {generating ? (
                <span className="inline-flex items-center gap-1.5">
                  <Loader2 size={13} className="animate-spin" />
                  {hasParts ? "追加中…" : "生成中…"}
                </span>
              ) : (
                generationActionLabel
              )}
            </BookButton>
          </div>
        </details>
      ) : null}

      {parts.length === 0 ? (
        <EmptyHint text="尚未生成分部大纲。" />
      ) : (
        <ul className="a4n-flat-surface divide-y divide-book-border/35 overflow-hidden">
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
              className="flex min-h-14 cursor-pointer items-start gap-3 px-4 py-3 transition-colors hover:bg-book-bg-tertiary/35 focus-visible:outline-2 focus-visible:outline-book-primary/35"
              title="查看分部大纲详情"
            >
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-semibold text-book-text-main">
                  第 {part.part_number} 部
                  {nonEmptyText(part.title) ? ` · ${nonEmptyText(part.title)}` : ""}
                </div>
                <div className="mt-0.5 text-[10px] text-book-text-muted">
                  第 {part.start_chapter}-{part.end_chapter} 章
                </div>
                {nonEmptyText(part.summary) ? (
                  <p className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-book-text-sub">
                    {nonEmptyText(part.summary)}
                  </p>
                ) : null}
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
