import React, { useEffect, useState } from "react";
import type { ChapterSummary } from "../../api/chapter";
import { DomainAgentConsole } from "../../components/agent/DomainAgentConsole";
import { OptimizeHeader } from "./optimize/OptimizeHeader";

interface OptimizeTabProps {
  projectId: number;
  chapters: ChapterSummary[];
  fixedChapterId?: number | null;
  embedded?: boolean;
  onChapterContentChanged?: (chapterId: number) => void | Promise<void>;
}

export const OptimizeTab: React.FC<OptimizeTabProps> = ({
  projectId,
  chapters,
  fixedChapterId,
  embedded = false,
  onChapterContentChanged,
}) => {
  const [selectedChapter, setSelectedChapter] = useState<number | "">(
    fixedChapterId ?? chapters[0]?.id ?? "",
  );

  useEffect(() => {
    setSelectedChapter((current) => {
      if (fixedChapterId != null) return fixedChapterId;
      if (
        typeof current === "number" &&
        chapters.some((chapter) => chapter.id === current)
      ) {
        return current;
      }
      return chapters[0]?.id ?? "";
    });
  }, [chapters, fixedChapterId]);

  const boundChapterId =
    fixedChapterId ??
    (typeof selectedChapter === "number" ? selectedChapter : null);
  const activeChapter =
    boundChapterId == null
      ? null
      : chapters.find((chapter) => chapter.id === boundChapterId) ?? null;

  return (
    <section
      className={
        embedded
          ? "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-book-bg-paper/40"
          : "mx-auto flex h-[min(72dvh,calc(100dvh-6rem))] max-w-5xl flex-col overflow-hidden rounded-2xl border border-book-border/55 bg-book-bg-paper/60 md:h-[calc(100vh-10rem)]"
      }
    >
      <OptimizeHeader
        activeChapter={activeChapter}
        chapters={chapters}
        isPinned={fixedChapterId != null}
        selectedChapter={selectedChapter}
        onSelectChapter={setSelectedChapter}
      />
      <DomainAgentConsole
        capability="content_optimization"
        projectId={projectId}
        chapterId={boundChapterId}
        resourceLabel={activeChapter?.title ?? null}
        onResourceChanged={
          boundChapterId != null && onChapterContentChanged
            ? () => onChapterContentChanged(boundChapterId)
            : undefined
        }
        fill
        className="min-h-0 flex-1 rounded-none border-0 shadow-none"
      />
    </section>
  );
};
