import { useState } from "react";
import { ChevronRight, Eye, Plus, SlidersHorizontal } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCardAction } from "../../../components/ui/BookCard";
import { nonEmptyText } from "../../../utils/text";
import { EmptyHint } from "../shared";
import { parseNumberInput, type NumberInputValue } from "./outlineTabUtils";
import type { PartOutline } from "../../../api/outline";
import { useI18n } from "../../../i18n";

interface PartOutlineSectionProps {
  parts: PartOutline[];
  loaded: boolean;
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
  loaded,
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
              : loaded ? t("outline.partSection.emptySummary") : null}
          </p>
        </div>
        {canEdit ? (
          <div className="flex shrink-0 flex-wrap items-center justify-end gap-1.5">
            <BookButton
              variant="secondary"
              size="sm"
              onClick={onCreatePart}
              disabled={disabled || !loaded}
            >
              <Plus size={13} /> {t("outline.partSection.directEntry")}
            </BookButton>
            <BookButton
              aria-label={t("outline.partSection.openAdvanced")}
              variant="secondary"
              size="sm"
              onClick={onOpenAdvanced}
              disabled={disabled || !loaded}
              className="inline-flex shrink-0 items-center gap-1.5"
            >
              <SlidersHorizontal size={13} aria-hidden="true" />
              {t("outline.partSection.advancedTools")}
            </BookButton>
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
            <span>{t("outline.partSection.settingsTitle")}</span>
            <span className="text-caption font-normal text-book-text-muted">{t("outline.partSection.settingsHint")}</span>
            <ChevronRight size={13} className="shrink-0 transition-transform group-open:rotate-90" />
          </summary>
          <div className="mt-3 grid grid-cols-1 items-end gap-3 border-t border-book-border/35 pt-3 md:grid-cols-[minmax(10rem,1fr)_minmax(10rem,1fr)_auto]">
            <label className="flex min-w-0 flex-col gap-1 text-small text-book-text-muted">
              {hasParts ? t("outline.partSection.appendCount") : t("outline.partSection.generateCount")}
              <input
                type="number"
                min={1}
                step={1}
                value={partCount}
                disabled={disabled || !loaded}
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
                disabled={disabled || !loaded}
                onChange={(event) =>
                  onChaptersPerPartChange(parseNumberInput(event.target.value))
                }
                className="pyqt-input h-10 w-full"
              />
            </label>
            <div className="flex flex-wrap items-center justify-end gap-2 md:pb-0">
              {!generating ? (
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={onPreviewGenerate}
                  disabled={disabled || !loaded}
                  className="inline-flex h-10 !min-h-10 min-w-0 shrink-0 items-center gap-1.5 !px-2 !text-xs"
                >
                  <Eye size={14} aria-hidden="true" />
                  {t(hasParts ? "outline.partSection.viewAppendRequest" : "outline.partSection.viewGenerateRequest")}
                </BookButton>
              ) : null}
              <BookButton
                variant="primary"
                size="sm"
                onClick={onGenerate}
                disabled={disabled || !loaded || generating}
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

      {parts.length === 0 && loaded ? (
        <EmptyHint text={t("outline.partSection.empty")} />
      ) : parts.length > 0 ? (
        <ul className="space-y-2">
          {parts.map((part) => (
            <li
              key={part.id}
              className="a4n-flat-surface relative flex min-h-14 items-start gap-3 px-4 py-3 transition-colors hover:border-book-primary/35"
            >
              <BookCardAction label={`${t("outline.partSection.part", { part: part.part_number })} · ${part.title}`} onClick={() => onOpenPart(part)} />
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
              <ChevronRight size={14} className="mt-1 shrink-0 text-book-text-muted" />
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
