import React, { useState } from "react";
import { useI18n } from "../../i18n";
import { type ChapterMetadata } from "../../api/chapter";
import { useChapterDetail } from "../../hooks/useChapterDetail";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { isDerivedChapterOutline, type ChapterOutlineListEntry } from "../../api/outline";

interface AssistantContextPanelProps {
  projectId: number;
  chapters: ChapterMetadata[];
  outlines: ChapterOutlineListEntry[];
  activeNumber: number | null;
}

const nextOutlineText = (
  outlines: ChapterOutlineListEntry[],
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
  projectId,
  chapters,
  outlines,
  activeNumber,
}) => {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const previous = activeNumber == null ? null
    : chapters.find((chapter) => chapter.chapter_number === activeNumber - 1);
  const detail = useChapterDetail(projectId, open ? previous?.id ?? null : null);
  const previousTail = detail.chapter?.content.trim().slice(-180);
  const nextOutline = nextOutlineText(outlines, activeNumber);

  if (!previous && !nextOutline) return null;

  return (
    <details className="text-xs text-book-text-muted" onToggle={(event) => setOpen(event.currentTarget.open)}>
      <summary className="w-fit cursor-pointer hover:text-book-text-main">{t("writing.contextReference")}</summary>
      <div className="a4n-flat-surface mt-2 divide-y divide-book-border/40 overflow-hidden leading-relaxed">
        {detail.loading && <LoadingSpinner size="sm" />}
        {detail.error && <p role="alert">{detail.error}</p>}
        {previousTail && (
          <section className="px-3 py-2.5">
            <div className="mb-1 font-semibold text-book-text-main">{t("writing.previousChapterEnding")}</div>
            <div className="line-clamp-4 whitespace-pre-wrap">…{previousTail}</div>
          </section>
        )}
        {nextOutline && (
          <section className="px-3 py-2.5">
            <div className="mb-1 font-semibold text-book-text-main">{t("writing.nextChapterOutline")}</div>
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
