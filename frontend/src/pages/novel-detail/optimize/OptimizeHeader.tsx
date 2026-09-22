import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import type { ChapterSummary } from "../../../api/chapter";
import { useI18n } from "../../../i18n";

interface OptimizeHeaderProps {
  activeChapter: ChapterSummary | null;
  chapters: ChapterSummary[];
  isPinned: boolean;
  selectedChapter: number | "";
  onSelectChapter: (chapterId: number | "") => void;
}

export function OptimizeHeader({
  activeChapter,
  chapters,
  isPinned,
  selectedChapter,
  onSelectChapter,
}: OptimizeHeaderProps) {
  const { t } = useI18n();
  const formatChapterLabel = (chapter: ChapterSummary) =>
    `${t("writing.chapterLabel", { number: chapter.chapter_number })}${
      chapter.title?.trim() ? ` · ${chapter.title.trim()}` : ""
    }`;
  const chapterOptions = chapters.map((chapter) => ({
    id: chapter.id,
    label: formatChapterLabel(chapter),
  }));
  const chapterLabel = activeChapter
    ? formatChapterLabel(activeChapter)
    : t("writing.assistantChapterNotSelected");

  return (
    <header className="flex min-w-0 shrink-0 flex-wrap items-center gap-2 border-b border-book-border/45 bg-book-bg-paper/75 px-3 py-2.5">
      <div className="min-w-0 flex-1 basis-44">
        <p className="truncate text-xs font-semibold text-book-text-main">
          {chapterLabel}
        </p>
        <p className="mt-0.5 truncate text-small text-book-text-muted">
          {activeChapter
            ? t("agent.domain.contentSubtitle", { resource: chapterLabel })
            : t("writing.chooseGeneratedChapterForAssistant")}
        </p>
      </div>
      {!isPinned ? (
        <label className="min-w-0 flex-1 basis-48">
          <span className="sr-only">{t("writing.chooseChapterTitle")}</span>
          <ThemedSelect
            value={String(selectedChapter)}
            title={t("writing.chooseChapterTitle")}
            aria-label={t("writing.chooseChapterTitle")}
            onChange={(selectedValue) => {
              const chapterId = Number(selectedValue);
              onSelectChapter(
                Number.isSafeInteger(chapterId) && chapterId > 0 ? chapterId : "",
              );
            }}
            className="pyqt-input h-9 w-full min-w-0 !py-1.5 text-small sm:h-8"
          >
            {chapterOptions.length === 0 ? (
              <option value="">{t("writing.chooseChapterTitle")}</option>
            ) : null}
            {chapterOptions.map((chapter) => (
              <option key={chapter.id} value={String(chapter.id)}>
                {chapter.label}
              </option>
            ))}
          </ThemedSelect>
        </label>
      ) : null}
    </header>
  );
}
