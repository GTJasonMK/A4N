import React, { memo } from "react";
import { ArrowRight } from "lucide-react";
import { chapterEventImportanceFilterRank } from "../../../utils/chapterAnalysisPresentation";
import type { ChapterStorySection } from "./types";
import { useI18n } from "../../../i18n";

interface ChapterEventSectionProps {
  section: ChapterStorySection;
  onOpenChapter?: (chapterNumber: number) => void;
}

const importanceClass = (importanceFilterKey: string): string => {
  const rank = chapterEventImportanceFilterRank(importanceFilterKey as Parameters<typeof chapterEventImportanceFilterRank>[0]);
  if (rank >= 3) return "text-book-primary";
  if (rank === 2) return "text-warning";
  return "text-book-text-muted";
};

export const ChapterEventSection: React.FC<ChapterEventSectionProps> = memo(({
  section,
  onOpenChapter,
}) => {
  const { t } = useI18n();
  return (
  <section className="a4n-story-event-section a4n-flat-surface p-3 sm:p-4 [content-visibility:auto] [contain-intrinsic-block-size:auto_40rem] print:[content-visibility:visible]">
    <div className="mb-2 flex min-w-0 items-start gap-3">
      <div className="min-w-0 flex-1">
        <h3 className="truncate text-sm font-semibold text-book-text-main">
          {section.title}
        </h3>
        <div className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-small text-book-text-muted">
          <span>{t("writing.chapterLabel", { number: section.chapter.chapter_number })}</span>
          {!section.hasAnalysis ? <span>{t("novel.storyEvents.notAnalyzed")}</span> : null}
          {section.timelineMarker ? <span>{section.timelineMarker}</span> : null}
          {section.tone ? <span>{section.tone}</span> : null}
          {section.locations.length > 0 ? (
            <span>{section.locations.slice(0, 3).join(t("common.listSeparator"))}</span>
          ) : null}
        </div>
      </div>
      {onOpenChapter ? (
        <button
          type="button"
          onClick={() => onOpenChapter(section.chapter.chapter_number)}
          className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
          aria-label={t("novel.storyEvents.openChapter", { number: section.chapter.chapter_number })}
          title={t("novel.storyEvents.openChapter", { number: section.chapter.chapter_number })}
        >
          <ArrowRight size={12} />
        </button>
      ) : null}
    </div>

    {section.summary && (
      <p className="mb-2 text-xs leading-relaxed text-book-text-sub">{section.summary}</p>
    )}

    {(section.characters.length > 0 || section.keywords.length > 0) && (
      <div className="mb-2 space-y-0.5 text-small leading-relaxed text-book-text-muted">
        {section.characters.length > 0 ? (
          <div>{t("novel.storyEvents.characters", { value: section.characters.slice(0, 8).join(t("common.listSeparator")) })}</div>
        ) : null}
        {section.keywords.length > 0 ? (
          <div>{t("novel.storyEvents.keywords", { value: section.keywords.slice(0, 6).join(t("common.listSeparator")) })}</div>
        ) : null}
      </div>
    )}

    {section.events.length > 0 ? (
      <ol className="divide-y divide-book-border/40 border-y border-book-border/40">
        {section.events.map((event) => (
          <li key={event.id} className="flex gap-2.5 py-2.5">
            <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-book-primary/75" />
            <div className="min-w-0">
              <div className="text-xs font-medium leading-relaxed text-book-text-main">
                {event.description}
              </div>
              <div className="mt-1 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-caption text-book-text-muted">
                <span>{event.type}</span>
                <span aria-hidden="true">·</span>
                <span className={importanceClass(event.importanceFilterValue)}>
                  {event.importance}
                </span>
                {event.involvedCharacters.length > 0 ? (
                  <>
                    <span aria-hidden="true">·</span>
                    <span>{event.involvedCharacters.join(t("common.listSeparator"))}</span>
                  </>
                ) : null}
              </div>
            </div>
          </li>
        ))}
      </ol>
    ) : (
      <div className="text-xs text-book-text-muted">{t("novel.storyEvents.noKeyEvents")}</div>
    )}

    {(section.plantedCount > 0 || section.resolvedCount > 0 || section.tensionCount > 0) && (
      <div className="mt-2 text-small text-book-text-muted">
        {t("novel.storyEvents.foreshadowing")}
        {section.plantedCount > 0 ? ` · ${t("novel.storyEvents.planted", { count: section.plantedCount })}` : ""}
        {section.resolvedCount > 0 ? ` · ${t("novel.storyEvents.resolved", { count: section.resolvedCount })}` : ""}
        {section.tensionCount > 0 ? ` · ${t("novel.storyEvents.tensions", { count: section.tensionCount })}` : ""}
      </div>
    )}
  </section>
  );
});
