import React from "react";
import { ArrowRight } from "lucide-react";
import { importanceRank } from "./storyEventUtils";
import type { ChapterStorySection } from "./types";

interface ChapterEventSectionProps {
  section: ChapterStorySection;
  onOpenChapter?: (chapterNumber: number) => void;
}

const importanceClass = (importance: string): string => {
  const rank = importanceRank(importance);
  if (rank >= 3) return "text-book-primary";
  if (rank === 2) return "text-warning";
  return "text-book-text-muted";
};

export const ChapterEventSection: React.FC<ChapterEventSectionProps> = ({
  section,
  onOpenChapter,
}) => (
  <section className="a4n-flat-surface p-3 sm:p-4">
    <div className="mb-2 flex min-w-0 items-start gap-3">
      <div className="min-w-0 flex-1">
        <h3 className="truncate text-sm font-semibold text-book-text-main">
          {section.title}
        </h3>
        <div className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[11px] text-book-text-muted">
          <span>第 {section.chapter.chapter_number} 章</span>
          {!section.hasAnalysis ? <span>未分析</span> : null}
          {section.timelineMarker ? <span>{section.timelineMarker}</span> : null}
          {section.tone ? <span>{section.tone}</span> : null}
          {section.locations.length > 0 ? (
            <span>{section.locations.slice(0, 3).join("、")}</span>
          ) : null}
        </div>
      </div>
      {onOpenChapter ? (
        <button
          type="button"
          onClick={() => onOpenChapter(section.chapter.chapter_number)}
          className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
          aria-label={`打开第 ${section.chapter.chapter_number} 章`}
          title={`打开第 ${section.chapter.chapter_number} 章`}
        >
          <ArrowRight size={12} />
        </button>
      ) : null}
    </div>

    {section.summary && (
      <p className="mb-2 text-xs leading-relaxed text-book-text-sub">{section.summary}</p>
    )}

    {(section.characters.length > 0 || section.keywords.length > 0) && (
      <div className="mb-2 space-y-0.5 text-[11px] leading-relaxed text-book-text-muted">
        {section.characters.length > 0 ? (
          <div>人物 · {section.characters.slice(0, 8).join("、")}</div>
        ) : null}
        {section.keywords.length > 0 ? (
          <div>关键词 · {section.keywords.slice(0, 6).join("、")}</div>
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
              <div className="mt-1 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-[10px] text-book-text-muted">
                <span>{event.type}</span>
                <span aria-hidden="true">·</span>
                <span className={importanceClass(event.importance)}>
                  {event.importance}
                </span>
                {event.involvedCharacters.length > 0 ? (
                  <>
                    <span aria-hidden="true">·</span>
                    <span>{event.involvedCharacters.join("、")}</span>
                  </>
                ) : null}
              </div>
            </div>
          </li>
        ))}
      </ol>
    ) : (
      <div className="text-xs text-book-text-muted">本章暂无关键事件。</div>
    )}

    {(section.plantedCount > 0 || section.resolvedCount > 0 || section.tensionCount > 0) && (
      <div className="mt-2 text-[11px] text-book-text-muted">
        伏笔
        {section.plantedCount > 0 ? ` · 埋设 ${section.plantedCount}` : ""}
        {section.resolvedCount > 0 ? ` · 回收 ${section.resolvedCount}` : ""}
        {section.tensionCount > 0 ? ` · 悬念 ${section.tensionCount}` : ""}
      </div>
    )}
  </section>
);
