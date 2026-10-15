import { useDeferredValue, useMemo } from "react";
import type { ChapterMetadata, ChapterSummary } from "../../api/chapter";
import type { ChapterOutlineListEntry } from "../../api/outline";
import type { ChapterRow } from "../novel-detail/shared";
import { useI18n } from "../../i18n";

interface UseWritingDeskChapterDerivedStateParams {
  chapters: ChapterMetadata[];
  activeChapter: ChapterSummary | null;
  outlines: ChapterOutlineListEntry[];
  activeNumber: number | null;
  chapterQuery: string;
}

export function useWritingDeskChapterDerivedState({
  chapters,
  activeChapter,
  outlines,
  activeNumber,
  chapterQuery,
}: UseWritingDeskChapterDerivedStateParams) {
  const { t } = useI18n();
  const chaptersByNumber = useMemo(
    () => new Map(chapters.map((chapter) => [chapter.chapter_number, chapter])),
    [chapters],
  );
  const outlinesByNumber = useMemo(
    () => new Map(outlines.map((outline) => [outline.chapter_number, outline])),
    [outlines],
  );
  const activeOutline = useMemo(
    () => (activeNumber == null ? null : outlinesByNumber.get(activeNumber) ?? null),
    [activeNumber, outlinesByNumber],
  );
  const activeOutlineText = useMemo(() => {
    if (!activeOutline) return null;
    const summary = activeOutline.summary?.trim();
    if (summary) return summary;
    const title = activeOutline.title?.trim();
    return title || null;
  }, [activeOutline]);
  const activeChapterContext = useMemo<ChapterRow | null>(
    () =>
      activeChapter
        ? {
            id: activeChapter.id,
            chapter_number: activeChapter.chapter_number,
            title: activeChapter.title,
            content: activeChapter.content,
            status: activeChapter.status,
            summary: activeChapter.summary,
            analysis_data: activeChapter.analysis_data,
          }
        : null,
    [activeChapter],
  );
  const chapterNumbers = useMemo(
    () =>
      Array.from(
        new Set(
          [
            ...chapters.map((chapter) => chapter.chapter_number),
            ...outlines.map((outline) => outline.chapter_number),
            activeNumber ?? 0,
          ],
        ),
      )
        .filter((number) => number > 0)
        .sort((left, right) => left - right),
    [activeNumber, chapters, outlines],
  );
  const deferredChapterQuery = useDeferredValue(chapterQuery);
  const filteredChapterNumbers = useMemo(() => {
    const query = deferredChapterQuery.trim().toLowerCase();
    if (!query) return chapterNumbers;

    return chapterNumbers.filter((number) => {
      const chapter = chaptersByNumber.get(number);
      const outline = outlinesByNumber.get(number);
      const haystack = [
        String(number),
        t("writing.chapterLabel", { number }),
        chapter?.title,
        outline?.title,
        outline?.summary,
        chapter?.summary,
        chapter?.status,
      ]
        .filter(Boolean)
        .join("\n")
        .toLowerCase();
      return haystack.includes(query);
    });
  }, [chapterNumbers, chaptersByNumber, deferredChapterQuery, outlinesByNumber, t]);

  return {
    activeChapter,
    activeOutline,
    activeOutlineText,
    activeChapterContext,
    chapterNumbers,
    filteredChapterNumbers,
  };
}
