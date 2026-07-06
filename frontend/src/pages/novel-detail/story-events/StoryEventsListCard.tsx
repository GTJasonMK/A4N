import React from "react";
import { Filter, Search, Sparkles, UserRound } from "lucide-react";
import { BookCard } from "../../../components/ui/BookCard";
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

  return (
    <BookCard className="p-4">
      <div className="mb-3 flex flex-wrap items-center gap-2 text-xs">
        <div className="relative min-w-0 flex-1 sm:min-w-[220px]">
          <Search
            size={12}
            className="pointer-events-none absolute left-2 top-2.5 text-book-text-muted"
          />
          <input
            value={filters.query}
            onChange={(event) => updateFilters({ query: event.target.value })}
            className="pyqt-input w-full !py-1.5 !pl-7 text-xs"
            placeholder="搜索事件、章节、地点、角色"
          />
        </div>
        <FilterSelect
          icon={<UserRound size={11} />}
          value={filters.character}
          onChange={(character) => updateFilters({ character })}
          options={filterOptions.characters}
          allLabel="全部角色"
        />
        <FilterSelect
          icon={<Filter size={11} />}
          value={filters.type}
          onChange={(type) => updateFilters({ type })}
          options={filterOptions.eventTypes}
          allLabel="全部类型"
        />
        <FilterSelect
          icon={<Sparkles size={11} />}
          value={filters.importance}
          onChange={(importance) => updateFilters({ importance })}
          options={filterOptions.importances}
          allLabel="全部重要性"
        />
      </div>

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
        <div className="space-y-3">
          {sections.map((section) => (
            <ChapterEventSection
              key={section.chapter.id}
              section={section}
              onOpenChapter={onOpenChapter}
            />
          ))}
        </div>
      )}
    </BookCard>
  );
};

const FilterSelect: React.FC<{
  icon: React.ReactNode;
  value: string;
  onChange: (value: string) => void;
  options: string[];
  allLabel: string;
}> = ({ icon, value, onChange, options, allLabel }) => (
  <label className="inline-flex items-center gap-1.5 text-book-text-muted">
    {icon}
    <select
      value={value}
      onChange={(event) => onChange(event.target.value)}
      className="pyqt-input !py-1.5 !px-2 text-xs"
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
