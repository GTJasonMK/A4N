import { describe, expect, it } from "vitest";
import { translateMessage, type MessageTranslator } from "../../../i18n";
import type { ChapterStoryEventEntry } from "../../../api/chapter";
import {
  chapterEventImportanceFilterKey,
  chapterEventImportanceFilterRank,
  chapterEventTypeFilterKey,
} from "../../../utils/chapterAnalysisPresentation";
import {
  extractChapterStorySection,
  filterStorySections,
  getStoryEventFilterOptions,
  reconcileStoryEventFilters,
} from "./storyEventUtils";

const chapter: ChapterStoryEventEntry = {
  id: 9,
  chapter_number: 3,
  title: "第三章",
  summary: null,
  analysis_data: {
    metadata: {
      characters: ["林舟", "沈默"],
      locations: ["书院"],
      tone: "紧张",
      timeline_marker: "第二天",
      items: ["不显示"],
    },
    summaries: { compressed: "线索现身", keywords: ["账本"], unused: "不显示" },
    key_events: [
      { description: "林舟离开书院", type: "departure", importance: "major", involved_characters: ["林舟"] },
      { description: "沈默抵达书院", type: "arrival", importance: "low", involved_characters: ["沈默"] },
      { description: "其他战斗", type: "battle", importance: "major", involved_characters: ["沈默"] },
    ],
  },
  foreshadowing_counts: { planted: 1, resolved: 1, tensions: 2 },
};

describe("story event filters", () => {
  it("uses projected display fields and foreshadowing counts", () => {
    const t: MessageTranslator = (key) => key;
    const section = extractChapterStorySection(chapter, t);
    expect(section).toMatchObject({
      summary: "线索现身",
      timelineMarker: "第二天",
      tone: "紧张",
      characters: ["林舟", "沈默"],
      locations: ["书院"],
      keywords: ["账本"],
      plantedCount: 1,
      resolvedCount: 1,
      tensionCount: 2,
      hasAnalysis: true,
    });
    expect(section.events.map((event) => event.description)).toEqual([
      "林舟离开书院",
      "沈默抵达书院",
      "其他战斗",
    ]);
  });

  it("keeps type and importance selections stable when labels change locale", () => {
    const zh: MessageTranslator = (key, params) => translateMessage("zh-CN", key, params);
    const en: MessageTranslator = (key, params) => translateMessage("en-US", key, params);
    const zhSection = extractChapterStorySection(chapter, zh);
    const enSection = extractChapterStorySection(chapter, en);
    const optionsZh = getStoryEventFilterOptions(zhSection.events);
    const optionsEn = getStoryEventFilterOptions(enSection.events);
    expect(optionsZh.eventTypes.map((option) => option.value).sort()).toEqual(optionsEn.eventTypes.map((option) => option.value).sort());
    expect(optionsZh.importances.map((option) => option.value).sort()).toEqual(optionsEn.importances.map((option) => option.value).sort());
    expect(optionsZh.importances.map((option) => option.value)).toEqual([
      "novel.importance.high",
      "novel.importance.low",
    ]);
    expect(optionsZh.eventTypes.find((option) => option.label !== optionsEn.eventTypes.find((enOption) => enOption.value === option.value)?.label)).toBeDefined();
  });

  it("filters the events associated with a character without admitting unrelated chapter events", () => {
    const t: MessageTranslator = (key) => key;
    const section = extractChapterStorySection(chapter, t);
    const options = getStoryEventFilterOptions(section.events);
    const filters = reconcileStoryEventFilters({ query: "", character: "林舟", type: "all", importance: "all" }, options);
    const filtered = filterStorySections([section], filters);
    expect(filtered).toHaveLength(1);
    expect(filtered[0].events.map((event) => event.description)).toEqual(["林舟离开书院"]);
  });

  it("keeps importance ordering independent of translated label sort order", () => {
    const t: MessageTranslator = (key) => key;
    const section = extractChapterStorySection(chapter, t);
    expect(getStoryEventFilterOptions(section.events).importances.map((option) => option.value)).toEqual([
      "novel.importance.high",
      "novel.importance.low",
    ]);
  });

  it("projects displayed importance color from the stable filter category", () => {
    expect(chapterEventImportanceFilterRank(chapterEventImportanceFilterKey("major"))).toBe(3);
    expect(chapterEventImportanceFilterRank(chapterEventImportanceFilterKey("low"))).toBe(1);
  });

  it("falls back to all when a selected category disappears from refreshed results", () => {
    const noEventsOptions = getStoryEventFilterOptions([]);
    expect(reconcileStoryEventFilters({ query: "留下", character: "林舟", type: chapterEventTypeFilterKey("departure"), importance: chapterEventImportanceFilterKey("major") }, noEventsOptions))
      .toEqual({ query: "留下", character: "all", type: "all", importance: "all" });
  });
});
