import { useState } from "react";
import { Eye, Plus, SlidersHorizontal } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { nonEmptyText } from "../../../utils/text";
import { EmptyHint } from "../shared";
import { parseNumberInput, type NumberInputValue } from "./outlineTabUtils";
import type { PartOutline } from "../../../api/outline";
import { useI18n } from "../../../i18n";

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
  onCreatePart: () => void;
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
  onCreatePart,
  onOpenAdvanced,
  onOpenPart,
}: PartOutlineSectionProps) {
  const { t } = useI18n();
  const [generationOpen, setGenerationOpen] = useState(parts.length === 0);
  const hasParts = parts.length > 0;
  const generationActionLabel = hasParts
    ? t("outline.partSection.appendAction")
    : t("outline.partSection.generateAction");

  return (
    <section className="space-y-2.5">
      <header className="flex items-start justify-between gap-3 px-1">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold text-book-text-main">{t("outline.partSection.title")}</h2>
          <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
            {hasParts
              ? t("outline.partSection.summary", { count: parts.length })
              : t("outline.partSection.emptySummary")}
          </p>
        </div>
        {canEdit ? (
          <div className="flex shrink-0 items-center gap-1.5">
            <BookButton
              variant="secondary"
              size="sm"
              onClick={onCreatePart}
              disabled={disabled}
            >
              <Plus size={13} /> {t("outline.partSection.directEntry")}
            </BookButton>
            <BookIconButton
              label={t("outline.partSection.openAdvanced")}
              onClick={onOpenAdvanced}
              disabled={disabled}
              variant="ghost"
              size="sm"
            >
              <SlidersHorizontal size={13} />
            </BookIconButton>
          </div>
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
            <span className="text-caption font-normal text-book-text-muted">{t("outline.partSection.settingsHint")}</span>
          </summary>
          <div className="mt-3 grid grid-cols-1 items-end gap-3 border-t border-book-border/35 pt-3 md:grid-cols-[minmax(10rem,1fr)_minmax(10rem,1fr)_auto]">
            <label className="flex min-w-0 flex-col gap-1 text-small text-book-text-muted">
              {hasParts ? t("outline.partSection.appendCount") : t("outline.partSection.generateCount")}
              <input
                type="number"
                min={1}
                step={1}
                value={partCount}
                disabled={disabled}
                onChange={(event) => onPartCountChange(parseNumberInput(event.target.value))}
                className="pyqt-input h-10 w-full"
              />
            </label>
            <label className="flex min-w-0 flex-col gap-1 text-small text-book-text-muted">
              {t("outline.partSection.chaptersPerPart")}
              <input
                type="number"
                min={1}
                step={1}
                placeholder={defaultChaptersPerPart === "" ? t("outline.partSection.autoPlan") : undefined}
                value={chaptersPerPart}
                disabled={disabled}
                onChange={(event) =>
                  onChaptersPerPartChange(parseNumberInput(event.target.value))
                }
                className="pyqt-input h-10 w-full"
              />
            </label>
            <div className="flex items-center justify-end gap-2 md:pb-0">
              {!generating ? (
                <BookIconButton
                  label={hasParts ? t("outline.partSection.viewAppendRequest") : t("outline.partSection.viewGenerateRequest")}
                  variant="secondary"
                  size="md"
                  onClick={onPreviewGenerate}
                  disabled={disabled}
                  className="!h-10 !w-10"
                >
                  <Eye size={14} />
                </BookIconButton>
              ) : null}
              <BookButton
                variant="primary"
                size="sm"
                onClick={onGenerate}
                disabled={disabled || generating}
                className="h-10 min-w-0 flex-1 !min-h-10 !px-4 !text-xs md:flex-none"
              >
                {generating ? (
                  <span className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" />
                    {hasParts ? t("outline.partSection.appending") : t("outline.partSection.generating")}
                  </span>
                ) : (
                  generationActionLabel
                )}
              </BookButton>
            </div>
          </div>
        </details>
      ) : null}

      {parts.length === 0 ? (
        <EmptyHint text={t("outline.partSection.empty")} />
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
              title={t("outline.partSection.viewDetails")}
            >
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-semibold text-book-text-main">
                  {t("outline.partSection.part", { part: part.part_number })}
                  {nonEmptyText(part.title) ? ` · ${nonEmptyText(part.title)}` : ""}
                </div>
                <div className="mt-0.5 text-caption text-book-text-muted">
                  {t("outline.partSection.chapterRange", {
                    start: part.start_chapter,
                    end: part.end_chapter,
                  })}
                </div>
                {nonEmptyText(part.summary) ? (
                  <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-sub">
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
