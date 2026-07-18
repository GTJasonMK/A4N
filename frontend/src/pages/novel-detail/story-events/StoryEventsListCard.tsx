import React from "react";
import { Search } from "lucide-react";
import { EmptyHint } from "../shared";
import { ChapterEventSection } from "./ChapterEventSection";
import { ALL_STORY_EVENT_FILTER } from "./storyEventUtils";
import type {
  ChapterStorySection,
  StoryEventFilterOptions,
  StoryEventFilters,
} from "./types";

interface StoryEventsListCardProps {
  chaptersCount: number;
  analyzedChapters: number;
  sections: ChapterStorySection[];
  filters: StoryEventFilters;
  filterOptions: StoryEventFilterOptions;
  onFiltersChange: (filters: StoryEventFilters) => void;
  onOpenWritingDesk?: () => void;
  onOpenChapter?: (chapterNumber: number) => void;
}

export const StoryEventsListCard: React.FC<StoryEventsListCardProps> = ({
  chaptersCount,
  analyzedChapters,
  sections,
  filters,
  filterOptions,
  onFiltersChange,
  onOpenWritingDesk,
  onOpenChapter,
}) => {
  const updateFilters = (patch: Partial<StoryEventFilters>) =>
    onFiltersChange({ ...filters, ...patch });
  const hasSecondaryFilters =
    filterOptions.characters.length > 1 ||
    filterOptions.eventTypes.length > 1 ||
    filterOptions.importances.length > 1;

  return (
    <section>
      {analyzedChapters > 0 ? (
        <div className="mb-3 grid gap-2 border-b border-book-border/40 pb-3 sm:grid-cols-[minmax(220px,1fr)_auto] sm:items-center">
          <div className="relative min-w-0">
            <Search
              size={12}
              className="pointer-events-none absolute left-2.5 top-2.5 text-book-text-muted"
            />
            <input
              value={filters.query}
              onChange={(event) => updateFilters({ query: event.target.value })}
              className="pyqt-input w-full !py-1.5 !pl-7 text-xs"
              placeholder="搜索事件、章节、地点、角色"
            />
          </div>
          {hasSecondaryFilters ? (
            <div className="flex min-w-0 gap-2 overflow-x-auto pb-1 custom-scrollbar sm:pb-0">
              {filterOptions.characters.length > 1 ? (
                <FilterSelect
                  value={filters.character}
                  onChange={(character) => updateFilters({ character })}
                  options={filterOptions.characters}
                  allLabel="全部角色"
                />
              ) : null}
              {filterOptions.eventTypes.length > 1 ? (
                <FilterSelect
                  value={filters.type}
                  onChange={(type) => updateFilters({ type })}
                  options={filterOptions.eventTypes}
                  allLabel="全部类型"
                />
              ) : null}
              {filterOptions.importances.length > 1 ? (
                <FilterSelect
                  value={filters.importance}
                  onChange={(importance) => updateFilters({ importance })}
                  options={filterOptions.importances}
                  allLabel="全部重要性"
                />
              ) : null}
            </div>
          ) : null}
        </div>
      ) : null}

      {chaptersCount === 0 ? (
        <EmptyHint
          text="项目还没有章节。"
          actionText={onOpenWritingDesk ? "进入写作台" : undefined}
          onAction={onOpenWritingDesk}
        />
      ) : analyzedChapters === 0 ? (
        <EmptyHint
          text="还没有章节分析数据。"
          actionText={onOpenWritingDesk ? "进入写作台生成分析" : undefined}
          onAction={onOpenWritingDesk}
        />
      ) : sections.length === 0 ? (
        <EmptyHint text="没有匹配当前筛选条件的事件。" />
      ) : (
        <div className="grid grid-cols-1 items-start gap-3">
          {sections.map((section) => (
            <ChapterEventSection
              key={section.chapter.id}
              section={section}
              onOpenChapter={onOpenChapter}
            />
          ))}
        </div>
      )}
    </section>
  );
};

const FilterSelect: React.FC<{
  value: string;
  onChange: (value: string) => void;
  options: string[];
  allLabel: string;
}> = ({ value, onChange, options, allLabel }) => (
  <label className="inline-flex shrink-0 items-center text-book-text-muted">
    <span className="sr-only">{allLabel}</span>
    <select
      value={value}
      onChange={(event) => onChange(event.target.value)}
      className="pyqt-input min-w-[7.5rem] !px-2 !py-1.5 text-xs"
    >
      <option value={ALL_STORY_EVENT_FILTER}>{allLabel}</option>
      {options.map((option) => (
        <option key={option} value={option}>
          {option}
        </option>
      ))}
    </select>
  </label>
);
