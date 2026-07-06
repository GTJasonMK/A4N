import React from "react";
import { type ChapterData } from "../../api/chapter";
import { type ChapterOutline } from "../../api/outline";

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
  const next = outlines.find((outline) => outline.chapter_number === activeNumber + 1);
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

  return (
    <>
      <h3 className="text-xs font-semibold text-book-text-sub uppercase tracking-wider mb-2">
        上下文
      </h3>
      <div className="text-xs text-book-text-muted leading-relaxed space-y-2">
        {previousTail && (
          <div className="a4n-page-surface px-3 py-2">
            <div className="font-semibold text-book-text-main mb-1">前章末尾</div>
            <div className="line-clamp-4 whitespace-pre-wrap">…{previousTail}</div>
          </div>
        )}
        {nextOutline && (
          <div className="a4n-page-surface px-3 py-2">
            <div className="font-semibold text-book-text-main mb-1">下章章纲</div>
            <div className="line-clamp-3">
              {nextOutline.title && (
                <span className="text-book-text-main">{nextOutline.title}</span>
              )}
              {nextOutline.summary && (
                <div className="text-book-text-muted">{nextOutline.summary}</div>
              )}
            </div>
          </div>
        )}
      </div>
    </>
  );
};
