import type { ChapterSummary } from "../../../api/chapter";
import {
  analysisTextList,
  analysisTextValue,
  firstAnalysisText,
} from "../../../utils/chapterAnalysisText";
import {
  chapterEventImportanceLabel,
  chapterEventImportanceRank,
  chapterEventTypeLabel,
} from "../../../utils/chapterAnalysisPresentation";
import { asArray, asJsonRecord as asRecord, type JsonRecord } from "../../../utils/jsonRecord";
import type {
  ChapterStorySection,
  StoryEvent,
  StoryEventFilterOptions,
  StoryEventFilters,
} from "./types";

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

const chapterTitle = (chapter: ChapterSummary): string =>
  chapter.title?.trim() || `第 ${chapter.chapter_number} 章`;

const analysisSummary = (analysis: JsonRecord | null, chapter: ChapterSummary): string => {
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

export const extractChapterStorySection = (chapter: ChapterSummary): ChapterStorySection => {
  const analysis = asRecord(chapter.analysis_data);
  const metadata = asRecord(analysis?.metadata);
  const summaries = asRecord(analysis?.summaries);
  const foreshadowing = asRecord(analysis?.foreshadowing);
  const title = chapterTitle(chapter);
  const eventRows = asArray(analysis?.key_events)
    .map((raw, index): StoryEvent | null => {
      const description = eventDescription(raw);
      if (!description) return null;
      const record = asRecord(raw);
      return {
        id: `${chapter.id}-${index}-${description}`,
        chapterId: chapter.id,
        chapterNumber: chapter.chapter_number,
        chapterTitle: title,
        description,
        type:
          chapterEventTypeLabel(firstAnalysisText(record, ["type"])) ??
          "其他事件",
        importance:
          chapterEventImportanceLabel(
            firstAnalysisText(record, ["importance", "priority"]),
          ) ?? "未标注",
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

export const importanceRank = (importance: string): number => {
  return chapterEventImportanceRank(importance);
};

export const getStoryEventFilterOptions = (
  sections: ChapterStorySection[],
  events: StoryEvent[],
): StoryEventFilterOptions => ({
  characters: Array.from(
    new Set(
      sections.flatMap((section) => [
        ...section.characters,
        ...section.events.flatMap((event) => event.involvedCharacters),
      ]),
    ),
  ).sort((a, b) => a.localeCompare(b, "zh-Hans-CN")),
  eventTypes: Array.from(new Set(events.map((event) => event.type)))
    .filter(Boolean)
    .sort((a, b) => a.localeCompare(b, "zh-Hans-CN")),
  importances: Array.from(new Set(events.map((event) => event.importance)))
    .filter(Boolean)
    .sort((a, b) => importanceRank(b) - importanceRank(a) || a.localeCompare(b, "zh-Hans-CN")),
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
        !event.involvedCharacters.includes(filters.character) &&
        !section.characters.includes(filters.character)
      ) {
        return false;
      }
      if (filters.type !== ALL_STORY_EVENT_FILTER && event.type !== filters.type) return false;
      if (
        filters.importance !== ALL_STORY_EVENT_FILTER &&
        event.importance !== filters.importance
      ) {
        return false;
      }
      if (!normalizedQuery) return true;
      return eventSearchText(event).toLowerCase().includes(normalizedQuery) || sectionText.includes(normalizedQuery);
    });

    const sectionMatches = Boolean(normalizedQuery) && sectionText.includes(normalizedQuery);
    if (events.length > 0 || sectionMatches) {
      visibleSections.push({ ...section, events });
    }
  }

  return visibleSections;
};

export const countAnalyzedChapters = (sections: ChapterStorySection[]): number =>
  sections.filter((section) => section.hasAnalysis).length;

export const countForeshadowingSignals = (sections: ChapterStorySection[]): number =>
  sections.reduce(
    (sum, section) => sum + section.plantedCount + section.resolvedCount + section.tensionCount,
    0,
  );
