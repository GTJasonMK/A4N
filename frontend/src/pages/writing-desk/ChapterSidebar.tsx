import React, { useMemo, useState } from "react";
import { Plus, RefreshCw, Search, X } from "lucide-react";
import { type ChapterSummary } from "../../api/chapter";
import {
  isDerivedChapterOutline,
  isRetrospectiveChapterOutline,
  type ChapterOutline,
} from "../../api/outline";
import { BookIconButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import { ChapterCard } from "./ChapterCard";
import { WritingContextCards } from "./WritingContextCards";
import { countChapterBodyCharacters } from "./wordCount";
import { useI18n } from "../../i18n";

interface ChapterSidebarProps {
  projectId: number;
  chapters: ChapterSummary[];
  outlines: ChapterOutline[];
  chapterNumbers: number[];
  filteredChapterNumbers: number[];
  activeNumber: number | null;
  activeEditorContent: string;
  activeOutline: ChapterOutline | null;
  chapterQuery: string;
  draftSet: ReadonlySet<number>;
  profilesRefreshKey: number;
  onChapterQueryChange: (query: string) => void;
  onClearChapterQuery: () => void;
  onOpenProfiles: () => void;
  onRefreshChapters: () => void | Promise<unknown>;
  onCreateChapter: () => void | Promise<unknown>;
  onSelectChapter: (chapterNumber: number) => void | Promise<unknown>;
}

const chapterTitle = (
  chapter: ChapterSummary | undefined,
  outline: ChapterOutline | undefined,
): string | null => {
  const chapterTitle = chapter?.title?.trim();
  if (chapterTitle) return chapterTitle;
  const outlineTitle = outline?.title?.trim();
  return outlineTitle ? outlineTitle : null;
};

const outlineTitle = (outline: ChapterOutline): string | null => {
  const title = outline.title?.trim();
  return title ? title : null;
};

const outlineSummary = (outline: ChapterOutline): string | null => {
  const summary = outline.summary?.trim();
  return summary ? summary : null;
};

const SIDEBAR_GRID_ROWS = "max-content minmax(0, 1fr) max-content";
const CHAPTER_LIST_GRID_ROWS = "auto auto minmax(0, 1fr)";

export const ChapterSidebar: React.FC<ChapterSidebarProps> = ({
  projectId,
  chapters,
  outlines,
  chapterNumbers,
  filteredChapterNumbers,
  activeNumber,
  activeEditorContent,
  activeOutline,
  chapterQuery,
  draftSet,
  profilesRefreshKey,
  onChapterQueryChange,
  onClearChapterQuery,
  onOpenProfiles,
  onRefreshChapters,
  onCreateChapter,
  onSelectChapter,
}) => {
  const { t } = useI18n();
  const [outlineDetailOpen, setOutlineDetailOpen] = useState(false);
  const chaptersByNumber = useMemo(
    () => new Map(chapters.map((chapter) => [chapter.chapter_number, chapter])),
    [chapters],
  );
  const outlinesByNumber = useMemo(
    () => new Map(outlines.map((outline) => [outline.chapter_number, outline])),
    [outlines],
  );
  const currentOutlineTitle = activeOutline ? outlineTitle(activeOutline) : null;
  const currentOutlineSummary = activeOutline ? outlineSummary(activeOutline) : null;
  const outlineSourceLabel = activeOutline
    ? isRetrospectiveChapterOutline(activeOutline)
      ? t("writing.outlineRetrospective")
      : isDerivedChapterOutline(activeOutline)
        ? t("writing.outlineCandidate")
        : t("writing.outlineChapter")
    : t("writing.outlineChapter");
  const currentOutlineLabel = activeOutline
    ? t("writing.outlineChapterWithSource", {
        source: outlineSourceLabel,
      })
    : t("writing.outlineChapter");

  return (
    <aside
      className="a4n-writing-sidebar grid h-full min-h-0 min-w-0 gap-2 overflow-hidden border-r border-book-border/80 p-2.5"
      style={{ gridTemplateRows: SIDEBAR_GRID_ROWS }}
    >
      <WritingContextCards
        projectId={projectId}
        chapterCount={chapters.length}
        totalChapters={outlines.filter((outline) => !isDerivedChapterOutline(outline)).length}
        refreshKey={profilesRefreshKey}
        onOpenProfiles={onOpenProfiles}
      />
      <section
        className="grid min-h-0 min-w-0 overflow-hidden"
        style={{ gridTemplateRows: CHAPTER_LIST_GRID_ROWS }}
      >
        <div className="flex min-w-0 items-center gap-2 px-1 py-1">
          <h2 className="min-w-0 flex-1 text-xs font-semibold text-book-text-main">{t("writing.chapterWorkspace")}</h2>
          {chapterNumbers.length > 0 ? (
            <span className="text-caption text-book-text-muted">
              {filteredChapterNumbers.length === chapterNumbers.length
                ? t("writing.chapterCount", { count: chapterNumbers.length })
                : t("writing.chapterCountFiltered", {
                    filtered: filteredChapterNumbers.length,
                    total: chapterNumbers.length,
                  })}
            </span>
          ) : null}
          <BookIconButton
            label={t("writing.newChapter")}
            onClick={() => void onCreateChapter()}
            variant="secondary"
            size="sm"
          >
            <Plus size={15} />
          </BookIconButton>
          <BookIconButton
            label={t("common.refresh")}
            onClick={() => void onRefreshChapters()}
            variant="secondary"
            size="sm"
          >
            <RefreshCw size={15} />
          </BookIconButton>
        </div>
        <div className="relative mt-1 min-w-0 shrink-0">
          <input
            type="text"
            value={chapterQuery}
            onChange={(event) => onChapterQueryChange(event.target.value)}
            placeholder={t("writing.chapterSearchPlaceholder")}
            aria-label={t("writing.chapterSearchPlaceholder")}
            className="pyqt-input w-full pl-8 pr-8 text-xs"
          />
          <Search
            size={12}
            className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-book-text-muted"
          />
          {chapterQuery.trim() && (
            <button
              type="button"
              onClick={onClearChapterQuery}
              className="absolute right-1 top-1/2 inline-flex h-8 w-8 -translate-y-1/2 items-center justify-center rounded-full text-small text-book-text-muted hover:bg-book-primary/10 hover:text-book-primary sm:right-1.5 sm:h-5 sm:w-5"
              aria-label={t("writing.clearChapterSearch")}
              title={t("writing.clearSearch")}
            >
              <X size={14} aria-hidden="true" />
            </button>
          )}
        </div>
        <div className="custom-scrollbar mt-2 min-h-0 min-w-0 overflow-y-auto pr-1">
          <ul className="a4n-motion-stagger a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
            {filteredChapterNumbers.map((chapterNumber) => {
              const chapter = chaptersByNumber.get(chapterNumber);
              const outline = outlinesByNumber.get(chapterNumber);
              const bodyContent =
                chapterNumber === activeNumber ? activeEditorContent : chapter?.content;
              const wordCount =
                bodyContent == null ? null : countChapterBodyCharacters(bodyContent);
              return (
                <li key={chapterNumber}>
                  <ChapterCard
                    chapterNumber={chapterNumber}
                    title={chapterTitle(chapter, outline)}
                    wordCount={wordCount}
                    status={chapter?.status}
                    materialSync={chapter?.material_sync}
                    hasDraft={draftSet.has(chapterNumber)}
                    selected={chapterNumber === activeNumber}
                    onClick={() => void onSelectChapter(chapterNumber)}
                  />
                </li>
              );
            })}
          </ul>
          {chapterNumbers.length > 0 && filteredChapterNumbers.length === 0 && (
            <div className="mt-4 text-center text-xs text-book-text-muted">{t("writing.noMatchingChapters")}</div>
          )}
          {!chapters.length && !outlines.length && chapterNumbers.length === 0 && (
            <div className="mt-4 text-center text-xs text-book-text-muted">
              {t("writing.noChaptersHint")}
            </div>
          )}
        </div>
      </section>

      <div className="min-w-0 overflow-hidden border-t border-book-border pt-1">
        {activeOutline ? (
          <button
            type="button"
            onClick={() => setOutlineDetailOpen(true)}
            className="w-full rounded-xl px-2 py-2 text-left text-small leading-snug text-book-text-sub transition-colors hover:bg-book-bg-tertiary/55"
            title={t("writing.viewOutline", { label: currentOutlineLabel })}
          >
            <div className="flex min-w-0 items-center gap-1.5">
              <span className="shrink-0 font-semibold text-book-text-muted">
                {currentOutlineLabel}
              </span>
              <span className="min-w-0 flex-1 truncate font-semibold text-book-text-main">
                {currentOutlineTitle ?? t("writing.chapterLabel", { number: activeOutline.chapter_number })}
              </span>
            </div>
            {currentOutlineSummary ? (
              <div className="mt-0.5 truncate text-caption text-book-text-muted">
                {currentOutlineSummary}
              </div>
            ) : null}
          </button>
        ) : (
          <p className="px-2 py-2 text-small text-book-text-muted">{t("writing.noOutlineForChapter")}</p>
        )}
      </div>

      <Modal
        isOpen={outlineDetailOpen && !!activeOutline}
        onClose={() => setOutlineDetailOpen(false)}
        title={currentOutlineLabel}
        maxWidthClassName="max-w-xl"
      >
        {activeOutline && (
          <div className="space-y-3 text-xs leading-relaxed text-book-text-sub">
            <div>
              <div className="mb-1 text-small font-semibold text-book-text-muted">
                {t("writing.chapterLabel", { number: activeOutline.chapter_number })}
              </div>
              <div className="font-serif text-base font-semibold text-book-text-main">
                {currentOutlineTitle ?? t("writing.unnamedChapter")}
              </div>
            </div>
            <div className="whitespace-pre-wrap text-book-text-main">
              {currentOutlineSummary ?? t("writing.outlineSummaryMissing")}
            </div>
            {isDerivedChapterOutline(activeOutline) ? (
              <div className="text-caption text-book-text-muted">
                {t("writing.derivedOutlineNote", {
                  source: outlineSourceLabel,
                })}
              </div>
            ) : null}
          </div>
        )}
      </Modal>
    </aside>
  );
};
