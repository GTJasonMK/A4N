import React from "react";
import { type ChapterData } from "../../api/chapter";
import { isDerivedChapterOutline, type ChapterOutline } from "../../api/outline";

interface AssistantContextPanelProps {
  chapters: ChapterData[];
  outlines: ChapterOutline[];
  activeNumber: number | null;
}

const previousChapterTail = (
  chapters: ChapterData[],
  activeNumber: number | null,
): string | null => {
  if (activeNumber == null) return null;
  const previous = chapters.find((chapter) => chapter.chapter_number === activeNumber - 1);
  const content = previous?.content.trim();
  return content ? content.slice(-180) : null;
};

const nextOutlineText = (
  outlines: ChapterOutline[],
  activeNumber: number | null,
): { title: string | null; summary: string | null } | null => {
  if (activeNumber == null) return null;
  const next = outlines.find(
    (outline) =>
      outline.chapter_number === activeNumber + 1 && !isDerivedChapterOutline(outline),
  );
  if (!next) return null;
  const title = next.title?.trim();
  const summary = next.summary?.trim();
  if (!title && !summary) return null;
  return {
    title: title ? title : null,
    summary: summary ? summary : null,
  };
};

export const AssistantContextPanel: React.FC<AssistantContextPanelProps> = ({
  chapters,
  outlines,
  activeNumber,
}) => {
  const previousTail = previousChapterTail(chapters, activeNumber);
  const nextOutline = nextOutlineText(outlines, activeNumber);

  if (!previousTail && !nextOutline) return null;

  return (
    <details className="text-xs text-book-text-muted">
      <summary className="w-fit cursor-pointer hover:text-book-text-main">参考上下文</summary>
      <div className="a4n-flat-surface mt-2 divide-y divide-book-border/40 overflow-hidden leading-relaxed">
        {previousTail && (
          <section className="px-3 py-2.5">
            <div className="mb-1 font-semibold text-book-text-main">前章末尾</div>
            <div className="line-clamp-4 whitespace-pre-wrap">…{previousTail}</div>
          </section>
        )}
        {nextOutline && (
          <section className="px-3 py-2.5">
            <div className="mb-1 font-semibold text-book-text-main">下章章纲</div>
            <div className="line-clamp-3">
              {nextOutline.title && (
                <span className="text-book-text-main">{nextOutline.title}</span>
              )}
              {nextOutline.summary && (
                <div className="text-book-text-muted">{nextOutline.summary}</div>
              )}
            </div>
          </section>
        )}
      </div>
    </details>
  );
};
