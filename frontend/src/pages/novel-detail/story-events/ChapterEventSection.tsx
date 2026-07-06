import React from "react";
import { ArrowRight, Clock3, MapPin } from "lucide-react";
import { importanceRank } from "./storyEventUtils";
import type { ChapterStorySection } from "./types";

interface ChapterEventSectionProps {
  section: ChapterStorySection;
  onOpenChapter?: (chapterNumber: number) => void;
}

const importanceClass = (importance: string): string => {
  const rank = importanceRank(importance);
  if (rank >= 3) return "!border-error/45 !text-error";
  if (rank === 2) return "!border-warning/45 !text-warning";
  if (rank === 1) return "!border-success/45 !text-success";
  return "";
};

export const ChapterEventSection: React.FC<ChapterEventSectionProps> = ({
  section,
  onOpenChapter,
}) => (
  <section className="rounded-sm border border-book-border/45 bg-book-bg-paper/35 p-3">
    <div className="mb-2 flex flex-wrap items-start justify-between gap-2">
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="font-mono text-[11px] text-book-text-muted">
            #{section.chapter.chapter_number}
          </span>
          <h4 className="text-sm font-semibold text-book-text-main">{section.title}</h4>
          {!section.hasAnalysis && <span className="pyqt-pill !text-[9px]">未分析</span>}
        </div>
        {(section.timelineMarker || section.tone || section.locations.length > 0) && (
          <div className="mt-1 flex flex-wrap gap-1.5 text-[10px] text-book-text-muted">
            {section.timelineMarker && (
              <span className="inline-flex items-center gap-1">
                <Clock3 size={10} /> {section.timelineMarker}
              </span>
            )}
            {section.tone && <span>{section.tone}</span>}
            {section.locations.slice(0, 3).map((location) => (
              <span key={location} className="inline-flex items-center gap-1">
                <MapPin size={10} /> {location}
              </span>
            ))}
          </div>
        )}
      </div>
      {onOpenChapter && (
        <button
          type="button"
          onClick={() => onOpenChapter(section.chapter.chapter_number)}
          className="pyqt-btn-secondary !min-h-9 !px-2.5 text-[11px] inline-flex items-center gap-1 sm:!min-h-[26px]"
        >
          打开章节 <ArrowRight size={11} />
        </button>
      )}
    </div>

    {section.summary && (
      <p className="mb-2 text-xs leading-relaxed text-book-text-sub">{section.summary}</p>
    )}

    {(section.characters.length > 0 || section.keywords.length > 0) && (
      <div className="mb-2 flex flex-wrap gap-1">
        {section.characters.slice(0, 8).map((name) => (
          <span key={`character-${name}`} className="pyqt-pill !text-[9px]">
            {name}
          </span>
        ))}
        {section.keywords.slice(0, 6).map((keyword) => (
          <span key={`keyword-${keyword}`} className="pyqt-pill !text-[9px] !text-book-primary">
            {keyword}
          </span>
        ))}
      </div>
    )}

    {section.events.length > 0 ? (
      <ol className="space-y-2">
        {section.events.map((event, index) => (
          <li key={event.id} className="grid grid-cols-[24px_1fr] gap-2">
            <span className="mt-0.5 flex h-5 w-5 items-center justify-center rounded-sm border border-book-border/50 bg-book-bg-tertiary text-[10px] font-mono text-book-text-muted">
              {index + 1}
            </span>
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-1.5">
                <span className="text-xs font-medium leading-relaxed text-book-text-main">
                  {event.description}
                </span>
                <span className="pyqt-pill !text-[9px]">{event.type}</span>
                <span className={`pyqt-pill !text-[9px] ${importanceClass(event.importance)}`}>
                  {event.importance}
                </span>
              </div>
              {event.involvedCharacters.length > 0 && (
                <div className="mt-0.5 text-[10px] text-book-text-muted">
                  {event.involvedCharacters.join("、")}
                </div>
              )}
            </div>
          </li>
        ))}
      </ol>
    ) : (
      <div className="text-xs text-book-text-muted">本章暂无关键事件。</div>
    )}

    {(section.plantedCount > 0 || section.resolvedCount > 0 || section.tensionCount > 0) && (
      <div className="mt-2 flex flex-wrap gap-1 text-[10px] text-book-text-muted">
        {section.plantedCount > 0 && (
          <span className="pyqt-pill !text-[9px]">埋设 {section.plantedCount}</span>
        )}
        {section.resolvedCount > 0 && (
          <span className="pyqt-pill !text-[9px]">回收 {section.resolvedCount}</span>
        )}
        {section.tensionCount > 0 && (
          <span className="pyqt-pill !text-[9px]">悬念 {section.tensionCount}</span>
        )}
      </div>
    )}
  </section>
);
