import { ThemedSelect } from "../../../components/ui/ThemedSelect";
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
import { useI18n } from "../../../i18n";

interface StoryEventsListCardProps {
  chaptersCount: number;
  analyzedChapters: number;
  sections: ChapterStorySection[];
  filters: StoryEventFilters;
  filterOptions: StoryEventFilterOptions;
  onFiltersChange: (filters: StoryEventFilters) => void;
  onOpenChapter?: (chapterNumber: number) => void;
}

export const StoryEventsListCard: React.FC<StoryEventsListCardProps> = ({
  chaptersCount,
  analyzedChapters,
  sections,
  filters,
  filterOptions,
  onFiltersChange,
  onOpenChapter,
}) => {
  const { t } = useI18n();
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
              placeholder={t("novel.storyEvents.searchPlaceholder")}
            />
          </div>
          {hasSecondaryFilters ? (
            <div className="flex min-w-0 gap-2 overflow-x-auto pb-1 custom-scrollbar sm:pb-0">
              {filterOptions.characters.length > 1 ? (
                <FilterSelect
                  value={filters.character}
                  onChange={(character) => updateFilters({ character })}
                  options={filterOptions.characters}
                  allLabel={t("novel.storyEvents.allCharacters")}
                />
              ) : null}
              {filterOptions.eventTypes.length > 1 ? (
                <FilterSelect
                  value={filters.type}
                  onChange={(type) => updateFilters({ type })}
                  options={filterOptions.eventTypes}
                  allLabel={t("novel.storyEvents.allTypes")}
                />
              ) : null}
              {filterOptions.importances.length > 1 ? (
                <FilterSelect
                  value={filters.importance}
                  onChange={(importance) => updateFilters({ importance })}
                  options={filterOptions.importances}
                  allLabel={t("novel.storyEvents.allImportance")}
                />
              ) : null}
            </div>
          ) : null}
        </div>
      ) : null}

      {chaptersCount === 0 ? (
        <EmptyHint text={t("novel.storyEvents.noChapters")} />
      ) : analyzedChapters === 0 ? (
        <EmptyHint text={t("novel.storyEvents.noAnalysis")} />
      ) : sections.length === 0 ? (
        <EmptyHint text={t("novel.storyEvents.noMatchingEvents")} />
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
    <ThemedSelect
      value={value}
      onChange={onChange}
      className="pyqt-input min-w-[7.5rem] !px-2 !py-1.5 text-xs"
    >
      <option value={ALL_STORY_EVENT_FILTER}>{allLabel}</option>
      {options.map((option) => (
        <option key={option} value={option}>
          {option}
        </option>
      ))}
    </ThemedSelect>
  </label>
);
