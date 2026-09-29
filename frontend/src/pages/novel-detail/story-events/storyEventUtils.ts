import type { ChapterAnalysisEntry } from "../../../api/chapter";
import {
  analysisTextList,
  analysisTextValue,
  firstAnalysisText,
} from "../../../utils/chapterAnalysisText";
import {
  chapterEventImportanceFilterKey,
  chapterEventImportanceFilterRank,
  chapterEventImportanceLabel,
  chapterEventTypeFilterKey,
  chapterEventTypeLabel,
} from "../../../utils/chapterAnalysisPresentation";
import { asArray, asJsonRecord as asRecord, type JsonRecord } from "../../../utils/jsonRecord";
import type {
  ChapterStorySection,
  StoryEvent,
  StoryEventFilterOptions,
  StoryEventFilters,
} from "./types";
import type { MessageTranslator } from "../../../i18n";

export const ALL_STORY_EVENT_FILTER = "all";

export const DEFAULT_STORY_EVENT_FILTERS: StoryEventFilters = {
  query: "",
  character: ALL_STORY_EVENT_FILTER,
  type: ALL_STORY_EVENT_FILTER,
  importance: ALL_STORY_EVENT_FILTER,
};

const textList = (value: unknown, limit = 32): string[] =>
  analysisTextList(value, { limit, unique: true });

const eventDescription = (raw: unknown): string => {
  if (typeof raw === "string") return analysisTextValue(raw);
  const record = asRecord(raw);
  return firstAnalysisText(record, ["description", "summary", "event", "title"]);
};

const chapterTitle = (chapter: ChapterAnalysisEntry, t: MessageTranslator): string =>
  chapter.title?.trim() || t("writing.chapterLabel", { number: chapter.chapter_number });

const analysisSummary = (analysis: JsonRecord | null, chapter: ChapterAnalysisEntry): string => {
  const summaries = asRecord(analysis?.summaries);
  return (
    firstAnalysisText(summaries, ["compressed", "summary", "short"]) ||
    firstAnalysisText(summaries, ["one_line"]) ||
    chapter.summary?.trim() ||
    ""
  );
};

const eventSearchText = (event: StoryEvent): string =>
  [
    event.description,
    event.type,
    event.importance,
    event.chapterTitle,
    event.chapterNumber,
    event.involvedCharacters.join(" "),
  ].join(" ");

const sectionSearchText = (section: ChapterStorySection): string =>
  [
    section.title,
    section.summary,
    section.timelineMarker,
    section.tone,
    section.characters.join(" "),
    section.locations.join(" "),
    section.keywords.join(" "),
  ].join(" ");

export const extractChapterStorySection = (
  chapter: ChapterAnalysisEntry,
  t: MessageTranslator,
): ChapterStorySection => {
  const analysis = asRecord(chapter.analysis_data);
  const metadata = asRecord(analysis?.metadata);
  const summaries = asRecord(analysis?.summaries);
  const foreshadowing = asRecord(analysis?.foreshadowing);
  const title = chapterTitle(chapter, t);
  const eventRows = asArray(analysis?.key_events)
    .map((raw, index): StoryEvent | null => {
      const description = eventDescription(raw);
      if (!description) return null;
      const record = asRecord(raw);
      const rawType = firstAnalysisText(record, ["type"]);
      const rawImportance = firstAnalysisText(record, ["importance", "priority"]);
      return {
        id: `${chapter.id}-${index}-${description}`,
        chapterId: chapter.id,
        chapterNumber: chapter.chapter_number,
        chapterTitle: title,
        description,
        type: chapterEventTypeLabel(rawType, t) ?? t("novel.eventTypeOther"),
        typeFilterValue: chapterEventTypeFilterKey(rawType),
        importance:
          chapterEventImportanceLabel(rawImportance, t) ?? t("novel.unlabeled"),
        importanceFilterValue: chapterEventImportanceFilterKey(rawImportance),
        involvedCharacters: textList(record?.involved_characters, 12),
      };
    })
    .filter((event): event is StoryEvent => Boolean(event));

  return {
    chapter,
    title,
    summary: analysisSummary(analysis, chapter),
    timelineMarker: firstAnalysisText(metadata, ["timeline_marker"]),
    tone: firstAnalysisText(metadata, ["tone"]),
    characters: textList(metadata?.characters, 24),
    locations: textList(metadata?.locations, 12),
    keywords: textList(summaries?.keywords, 12),
    events: eventRows,
    plantedCount: asArray(foreshadowing?.planted).length,
    resolvedCount: asArray(foreshadowing?.resolved).length,
    tensionCount: asArray(foreshadowing?.tensions).length,
    hasAnalysis: Boolean(analysis),
  };
};

const uniqueOptions = (
  events: StoryEvent[],
  value: (event: StoryEvent) => string,
  label: (event: StoryEvent) => string,
): StoryEventFilterOptions["eventTypes"] => {
  const options = new Map<string, string>();
  for (const event of events) options.set(value(event), label(event));
  return Array.from(options, ([optionValue, optionLabel]) => ({ value: optionValue, label: optionLabel }))
    .sort((left, right) => left.value.localeCompare(right.value));
};

export const getStoryEventFilterOptions = (events: StoryEvent[]): StoryEventFilterOptions => ({
  characters: Array.from(new Set(events.flatMap((event) => event.involvedCharacters)))
    .sort((a, b) => a.localeCompare(b, "zh-Hans-CN")),
  eventTypes: uniqueOptions(events, (event) => event.typeFilterValue, (event) => event.type),
  importances: Array.from(new Map(events.map((event) => [event.importanceFilterValue, event.importance])), ([value, label]) => ({ value, label }))
    .sort((left, right) => chapterEventImportanceFilterRank(right.value as Parameters<typeof chapterEventImportanceFilterRank>[0]) - chapterEventImportanceFilterRank(left.value as Parameters<typeof chapterEventImportanceFilterRank>[0]) || left.value.localeCompare(right.value)),
});

export const reconcileStoryEventFilters = (
  filters: StoryEventFilters,
  options: StoryEventFilterOptions,
): StoryEventFilters => ({
  ...filters,
  character: filters.character === ALL_STORY_EVENT_FILTER || options.characters.includes(filters.character)
    ? filters.character
    : ALL_STORY_EVENT_FILTER,
  type: filters.type === ALL_STORY_EVENT_FILTER || options.eventTypes.some((option) => option.value === filters.type)
    ? filters.type
    : ALL_STORY_EVENT_FILTER,
  importance: filters.importance === ALL_STORY_EVENT_FILTER || options.importances.some((option) => option.value === filters.importance)
    ? filters.importance
    : ALL_STORY_EVENT_FILTER,
});

export const filterStorySections = (
  sections: ChapterStorySection[],
  filters: StoryEventFilters,
): ChapterStorySection[] => {
  const normalizedQuery = filters.query.trim().toLowerCase();
  const visibleSections: ChapterStorySection[] = [];

  for (const section of sections) {
    const sectionText = sectionSearchText(section).toLowerCase();
    const events = section.events.filter((event) => {
      if (
        filters.character !== ALL_STORY_EVENT_FILTER &&
        !event.involvedCharacters.includes(filters.character)
      ) {
        return false;
      }
      if (filters.type !== ALL_STORY_EVENT_FILTER && event.typeFilterValue !== filters.type) return false;
      if (
        filters.importance !== ALL_STORY_EVENT_FILTER &&
        event.importanceFilterValue !== filters.importance
      ) {
        return false;
      }
      if (!normalizedQuery) return true;
      return eventSearchText(event).toLowerCase().includes(normalizedQuery) || sectionText.includes(normalizedQuery);
    });

    const sectionMatches = Boolean(normalizedQuery) && sectionText.includes(normalizedQuery);
    if (events.length > 0 || sectionMatches) {
      visibleSections.push(events.length === section.events.length ? section : { ...section, events });
    }
  }

  return visibleSections;
};

export const countAnalyzedChapters = (sections: ChapterStorySection[]): number =>
  sections.filter((section) => section.hasAnalysis).length;
