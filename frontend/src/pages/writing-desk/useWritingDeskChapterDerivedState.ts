import { useDeferredValue, useMemo } from "react";
import type { ChapterData } from "../../api/chapter";
import type { ChapterOutline } from "../../api/outline";
import type { OptimizationPatchPreviewController } from "../novel-detail/optimize/patchPreviewTypes";
import type { ChapterRow } from "../novel-detail/shared";

interface UseWritingDeskChapterDerivedStateParams {
  chapters: ChapterData[];
  outlines: ChapterOutline[];
  activeNumber: number | null;
  chapterQuery: string;
  optimizationPatchPreview: OptimizationPatchPreviewController | null;
}

export function useWritingDeskChapterDerivedState({
  chapters,
  outlines,
  activeNumber,
  chapterQuery,
  optimizationPatchPreview,
}: UseWritingDeskChapterDerivedStateParams) {
  const chaptersByNumber = useMemo(
    () => new Map(chapters.map((chapter) => [chapter.chapter_number, chapter])),
    [chapters],
  );
  const outlinesByNumber = useMemo(
    () => new Map(outlines.map((outline) => [outline.chapter_number, outline])),
    [outlines],
  );
  const activeChapter = useMemo(
    () => (activeNumber == null ? null : chaptersByNumber.get(activeNumber) ?? null),
    [activeNumber, chaptersByNumber],
  );
  const activeOutline = useMemo(
    () => (activeNumber == null ? null : outlinesByNumber.get(activeNumber) ?? null),
    [activeNumber, outlinesByNumber],
  );
  const activeOutlineTitle = useMemo(() => {
    const title = activeOutline?.title?.trim();
    return title ? title : null;
  }, [activeOutline]);
  const activeOutlineText = useMemo(() => {
    if (!activeOutline) return null;
    const summary = activeOutline.summary?.trim();
    if (summary) return summary;
    return activeOutlineTitle;
  }, [activeOutline, activeOutlineTitle]);
  const visibleOptimizationPatchPreview =
    optimizationPatchPreview?.proposal.chapter_id === activeChapter?.id
      ? optimizationPatchPreview
      : null;
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
          [...chapters, ...outlines].map((item) =>
            "chapter_number" in item ? item.chapter_number : 0,
          ),
        ),
      )
        .filter((number) => number > 0)
        .sort((left, right) => left - right),
    [chapters, outlines],
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
        `第${number}章`,
        `第 ${number} 章`,
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
  }, [chapterNumbers, chaptersByNumber, deferredChapterQuery, outlinesByNumber]);

  return {
    activeChapter,
    activeOutline,
    activeOutlineTitle,
    activeOutlineText,
    visibleOptimizationPatchPreview,
    activeChapterContext,
    chapterNumbers,
    filteredChapterNumbers,
  };
}
