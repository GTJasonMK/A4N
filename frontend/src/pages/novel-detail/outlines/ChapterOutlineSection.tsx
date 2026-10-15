import { useState } from "react";
import { ChevronRight, Eye, MoreHorizontal, Plus } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCardAction } from "../../../components/ui/BookCard";
import { nonEmptyText } from "../../../utils/text";
import { EmptyHint } from "../shared";
import { deleteChapterOutlinesFromAction } from "./outlineTabUtils";
import {
  isDerivedChapterOutline,
  isRetrospectiveChapterOutline,
  type ChapterOutlineListEntry,
} from "../../../api/outline";
import { useI18n } from "../../../i18n";
import {
  MAX_CHAPTER_OUTLINE_GENERATION_COUNT,
  parseNumberInput,
  type NumberInputValue,
} from "./outlineTabUtils";

export interface ChapterOutlineEditForm {
  title: string;
  summary: string;
}

interface ChapterOutlineSectionProps {
  outlines: ChapterOutlineListEntry[];
  loaded: boolean;
  canEdit: boolean;
  chapterCount: NumberInputValue;
  generating: boolean;
  editingId: number | null;
  editForm: ChapterOutlineEditForm;
  savingId: number | null;
  regenId: number | null;
  delActing: string | null;
  disabled?: boolean;
  onChapterCountChange: (value: NumberInputValue) => void;
  onGenerate: () => void;
  onPreviewGenerate: () => void;
  onCreate: () => void;
  onOpenDetail: (outline: ChapterOutlineListEntry) => void;
  onCancelEdit: () => void;
  onEditFormChange: (form: ChapterOutlineEditForm) => void;
  onSave: (outline: ChapterOutlineListEntry) => void;
  onOpenAction: (outline: ChapterOutlineListEntry) => void;
}

export function ChapterOutlineSection({
  outlines,
  loaded,
  canEdit,
  chapterCount,
  generating,
  editingId,
  editForm,
  savingId,
  regenId,
  delActing,
  disabled = false,
  onChapterCountChange,
  onGenerate,
  onPreviewGenerate,
  onCreate,
  onOpenDetail,
  onCancelEdit,
  onEditFormChange,
  onSave,
  onOpenAction,
}: ChapterOutlineSectionProps) {
  const { t } = useI18n();
  const [generationOpen, setGenerationOpen] = useState(outlines.length === 0);
  const hasOutlines = outlines.length > 0;
  const derivedCount = outlines.filter(isDerivedChapterOutline).length;
  const generationActionLabel = hasOutlines
    ? t("outline.chapterSection.appendAction")
    : t("outline.chapterSection.generateAction");
  const sourceLabel = (outline: ChapterOutlineListEntry) =>
    isRetrospectiveChapterOutline(outline)
      ? t("outline.chapterSection.sourceRetrospective")
      : isDerivedChapterOutline(outline)
        ? t("outline.chapterSection.sourceAnalysis")
        : t("outline.chapterSection.sourceChapter");

  return (
    <section className="space-y-2.5">
      <header className="flex items-start justify-between gap-3 px-1">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold text-book-text-main">{t("outline.chapterSection.title")}</h2>
          <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
            {hasOutlines
              ? t("outline.chapterSection.summary", {
                  count: outlines.length,
                  derived:
                    derivedCount > 0
                      ? t("outline.chapterSection.derived", { count: derivedCount })
                      : "",
                })
              : loaded ? t("outline.chapterSection.emptySummary") : null}
          </p>
        </div>
        {canEdit ? (
          <BookButton
            variant="secondary"
            size="sm"
            onClick={onCreate}
            disabled={disabled || !loaded}
            className="shrink-0"
          >
            <Plus size={13} /> {t("outline.chapterSection.directEntry")}
          </BookButton>
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
            <span className="text-caption font-normal text-book-text-muted">{t("outline.chapterSection.appendHint")}</span>
          </summary>
          <div className="mt-3 grid grid-cols-1 items-end gap-3 border-t border-book-border/35 pt-3 md:grid-cols-[minmax(12rem,20rem)_auto]">
            <label className="flex min-w-0 flex-col gap-1 text-small text-book-text-muted">
              {t("outline.chapterSection.appendCount")}
              <input
                type="number"
                min={1}
                max={MAX_CHAPTER_OUTLINE_GENERATION_COUNT}
                value={chapterCount}
                disabled={disabled || !loaded}
                onChange={(event) => onChapterCountChange(parseNumberInput(event.target.value))}
                className="pyqt-input h-10 w-full"
              />
            </label>
            <div className="flex items-center justify-end gap-2 md:justify-start">
              {!generating ? (
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={onPreviewGenerate}
                  disabled={disabled || !loaded}
                  className="inline-flex h-10 !min-h-10 min-w-0 flex-1 items-center justify-center gap-1.5 !px-2 !text-xs md:flex-none"
                >
                  <Eye size={14} />
                  {hasOutlines ? t("outline.chapterSection.viewAppendRequest") : t("outline.chapterSection.viewGenerateRequest")}
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
                    {hasOutlines ? t("outline.chapterSection.appending") : t("outline.chapterSection.generating")}
                  </span>
                ) : (
                  generationActionLabel
                )}
              </BookButton>
            </div>
          </div>
        </details>
      ) : null}

      {outlines.length === 0 && loaded ? (
        <EmptyHint text={t("outline.chapterSection.empty")} />
      ) : outlines.length > 0 ? (
        <ul className="space-y-2">
          {outlines.map((outline) => {
            const isEditing = editingId === outline.id;
            const deleting = delActing === deleteChapterOutlinesFromAction(outline.id);
            return (
              <li
                key={outline.id}
                className="a4n-flat-surface relative flex min-h-14 items-start gap-3 px-4 py-3 transition-colors hover:border-book-primary/35"
              >
                {isEditing ? (
                  <div className="flex flex-1 flex-col gap-2">
                    <span className="text-small font-medium text-book-text-muted">
                      {t("outline.chapterSection.chapter", { chapter: outline.chapter_number })}
                    </span>
                    <input
                      type="text"
                      aria-label={t("outline.chapterSection.titlePlaceholder")}
                      value={editForm.title}
                      onChange={(event) =>
                        onEditFormChange({ ...editForm, title: event.target.value })
                      }
                      disabled={disabled || savingId === outline.id}
                      placeholder={t("outline.chapterSection.titlePlaceholder")}
                      className="pyqt-input !py-1 text-sm"
                    />
                    <textarea
                      aria-label={t("outline.chapterSection.summaryPlaceholder")}
                      value={editForm.summary}
                      onChange={(event) =>
                        onEditFormChange({ ...editForm, summary: event.target.value })
                      }
                      disabled={disabled || savingId === outline.id}
                      placeholder={t("outline.chapterSection.summaryPlaceholder")}
                      className="book-control min-h-40 resize-y p-2 text-xs leading-relaxed"
                    />
                    <div className="flex flex-wrap items-center justify-end gap-1.5">
                      <BookButton
                        type="button"
                        variant="secondary"
                        size="sm"
                        onClick={onCancelEdit}
                        disabled={disabled || savingId === outline.id}
                      >
                        {t("outline.chapterSection.cancel")}
                      </BookButton>
                      <BookButton
                        type="button"
                        variant="primary"
                        size="sm"
                        onClick={() => onSave(outline)}
                        disabled={disabled || savingId === outline.id}
                      >
                        {savingId === outline.id ? (
                          <LoadingSpinner size="sm" />
                        ) : (
                          t("outline.chapterSection.save")
                        )}
                      </BookButton>
                    </div>
                  </div>
                ) : (
                  <>
                    <BookCardAction label={`${t("outline.chapterSection.chapter", { chapter: outline.chapter_number })} · ${outline.title ?? t("outline.chapterSection.viewDetails")}`} onClick={() => onOpenDetail(outline)} />
                    <div className="min-w-0 flex-1 pr-8">
                      <div className="flex min-w-0 items-center gap-1.5 text-sm font-semibold text-book-text-main">
                        <span className="truncate">
                          {t("outline.chapterSection.chapter", { chapter: outline.chapter_number })}
                          {nonEmptyText(outline.title) ? ` · ${nonEmptyText(outline.title)}` : ""}
                        </span>
                        {isDerivedChapterOutline(outline) ? (
                          <span className="shrink-0 rounded-full bg-book-bg-tertiary px-1.5 py-0.5 text-[10px] font-normal text-book-text-muted">
                            {sourceLabel(outline)}
                          </span>
                        ) : null}
                      </div>
                      {nonEmptyText(outline.summary) ? (
                        <p className="mt-0.5 line-clamp-2 text-small leading-relaxed text-book-text-muted">
                          {nonEmptyText(outline.summary)}
                        </p>
                      ) : null}
                    </div>
                    {canEdit && (
                      <div className="relative z-20 flex shrink-0 items-center">
                        <button
                          type="button"
                          onClick={(event) => {
                            event.stopPropagation();
                            onOpenAction(outline);
                          }}
                          disabled={disabled || regenId === outline.id || deleting}
                          title={t("outline.chapterSection.moreActions")}
                          aria-label={t("outline.chapterSection.moreActions")}
                          className="pyqt-btn-icon !h-9 !w-9 sm:!h-8 sm:!w-8"
                        >
                          {regenId === outline.id || deleting ? (
                            <LoadingSpinner size="sm" />
                          ) : (
                            <MoreHorizontal size={13} />
                          )}
                        </button>
                      </div>
                    )}
                    <span
                      aria-hidden="true"
                      className={`chapter-outline-open-indicator pointer-events-none absolute top-1/2 z-20 -translate-y-1/2 text-book-text-muted ${canEdit ? "right-12" : "right-3"}`}
                    >
                      <ChevronRight size={14} />
                    </span>
                  </>
                )}
              </li>
            );
          })}
        </ul>
      ) : null}
    </section>
  );
}
