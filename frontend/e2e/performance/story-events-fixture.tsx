import React from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { I18nProvider, translateMessage, type Locale } from "../../src/i18n";
import { StoryEventsListCard } from "../../src/pages/novel-detail/story-events/StoryEventsListCard";
import { DEFAULT_STORY_EVENT_FILTERS, extractChapterStorySection, filterStorySections, getStoryEventFilterOptions } from "../../src/pages/novel-detail/story-events/storyEventUtils";
import type { ChapterSummary } from "../../src/api/chapter";
import "../../src/index.css";

const rootElement = document.getElementById("root")!;
const root = createRoot(rootElement);
let version = 0;
let opened: number[] = [];
function chapter(number: number): ChapterSummary {
  return {
    id: number, project_id: 1, chapter_number: number, title: `调查线索 ${number}`,
    content: "正文🙂é", word_count: 2, status: "draft", outline_id: null, edit_revision: `revision-${number}`,
    summary: null, style_tags: null, created_at: null, updated_at: null,
    analysis_data: {
      metadata: { characters: ["林舟", number % 2 ? "沈青" : "陈墨"], locations: ["河岸"], tone: "紧张", timeline_marker: `第${number}天` },
      summaries: { compressed: `调查线索：第${number}章追寻账本下落`, keywords: ["账本", "调查"] },
      key_events: Array.from({ length: 5 }, (_, event) => ({ description: `线索${event}：${number}章发现账本记录 ${"灯火照亮河岸，林舟追寻来客留下的足迹。".repeat(5)}`,
        type: event % 2 ? "conflict" : "discovery", importance: event % 2 ? "high" : "medium", involved_characters: [event % 2 ? "沈青" : "林舟"] })),
      foreshadowing: { planted: ["账本"], resolved: [], tensions: [] },
    },
    analysis_sync_status: null, analysis_sync_can_resume: false, analysis_sync_has_saved_analysis: false,
    analysis_sync_resume_block_reason: null, analysis_sync_error: null,
    derived_sync_status: "completed", derived_sync_workflow_run_id: null, derived_sync_can_resume: false,
    derived_sync_error: null, material_updates: [],
    material_sync: { content_index: "completed", outline_and_foreshadowing: "not_analyzed",
      objects: "not_analyzed", characters: "not_analyzed", scenes: "not_analyzed" },
  };
}
function session(count: number, locale: Locale) {
  const key = ++version;
  const t = (key: Parameters<typeof translateMessage>[1], params?: Parameters<typeof translateMessage>[2]) => translateMessage(locale, key, params);
  const chapterBuildStart = performance.now();
  const chapters = Array.from({ length: count }, (_, i) => chapter(i + 1));
  const chapterBuildMs = performance.now() - chapterBuildStart;
  const sectionBuildStart = performance.now();
  const sections = chapters.map((chapter) => extractChapterStorySection(chapter, t));
  const sectionBuildMs = performance.now() - sectionBuildStart;
  const eventIndexStart = performance.now();
  const options = getStoryEventFilterOptions(sections, sections.flatMap((section) => section.events));
  const eventIndexMs = performance.now() - eventIndexStart;
  const timings = { chapterBuildMs, sectionBuildMs, eventIndexMs, filterMs: 0, reactCommitMs: 0 };
  let filters = { ...DEFAULT_STORY_EVENT_FILTERS };
  let onOpenChapter = (number: number) => { opened.push(number); };
  function render(query: string, character = "all", replaceCallback = false) {
    filters = { ...filters, query, character };
    if (replaceCallback) onOpenChapter = (number) => { opened.push(number + 1000); };
    const filterStart = performance.now();
    const visible = filterStorySections(sections, filters);
    timings.filterMs = performance.now() - filterStart;
    const commitStart = performance.now();
    flushSync(() => root.render(<I18nProvider initialLocale={locale} key={key}>
      <StoryEventsListCard chaptersCount={count} analyzedChapters={count} sections={visible}
        filters={filters} filterOptions={options} onFiltersChange={(next) => render(next.query, next.character)} onOpenChapter={onOpenChapter} />
    </I18nProvider>));
    timings.reactCommitMs = performance.now() - commitStart;
    return visible;
  }
  render("");
  const initialRender = { ...timings };
  return { render, timings, initialRender };
}
async function settle() {
  await new Promise<void>((done) => requestAnimationFrame(() => done()));
  await document.fonts.ready;
}
async function verifyStoryEvents(locale: Locale) {
  opened = [];
  const view = session(10, locale);
  const trace = [];
  for (const [query, character] of [["", "all"], ["调查", "all"], ["线索1", "all"], ["", "沈青"], ["不存在", "all"], ["调查", "all"]]) {
    const expected = view.render(query, character, true);
    await settle();
    if (rootElement.querySelectorAll("ol > li").length !== expected.reduce((sum, section) => sum + section.events.length, 0)) throw new Error("Event rows changed");
    trace.push(rootElement.innerHTML);
  }
  return trace;
}
async function measureStoryEvents(count: number) {
  const view = session(count, "zh-CN");
  const settleStart = performance.now();
  await settle();
  const initialSections = rootElement.querySelectorAll(".a4n-story-event-section");
  const initialLastRect = initialSections[initialSections.length - 1]?.getBoundingClientRect();
  const initial = { ...view.initialRender,
    settleAndLayoutMs: performance.now() - settleStart,
    domSections: initialSections.length,
    eventRows: rootElement.querySelectorAll(".a4n-story-event-section ol > li").length,
    domElements: rootElement.querySelectorAll("*").length,
    documentHeight: document.documentElement.scrollHeight,
    lastSectionBottom: initialLastRect?.bottom ?? null };
  const measurements = [];
  const filterSamples = [];
  const commitSamples = [];
  for (let run = 0; run < 7; run++) {
    const start = performance.now();
    let filterMs = 0;
    let reactCommitMs = 0;
    // Common prefixes retain whole matching chapters; narrowing still removes nonmatching rows.
    for (const query of ["调", "调查", "调查线", "调查线索", "线索1", "线索", "调查", ""]) {
      view.render(query);
      filterMs += view.timings.filterMs;
      reactCommitMs += view.timings.reactCommitMs;
    }
    const elapsed = performance.now() - start;
    if (run >= 2) {
      measurements.push(elapsed);
      filterSamples.push(filterMs);
      commitSamples.push(reactCommitMs);
    }
  }
  const hash = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(rootElement.innerHTML));
  const visibleTextHash = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(rootElement.innerText));
  return { chapters: count, events: count * 5, samplesMs: measurements,
    medianMs: [...measurements].sort((a, b) => a - b)[2],
    initial,
    filterSamplesMs: filterSamples,
    reactCommitSamplesMs: commitSamples,
    htmlSha256: Array.from(new Uint8Array(hash), (value) => value.toString(16).padStart(2, "0")).join(""),
    visibleTextSha256: Array.from(new Uint8Array(visibleTextHash), (value) => value.toString(16).padStart(2, "0")).join("") };
}
Object.assign(window, { verifyStoryEvents, measureStoryEvents, storyEventsOpened: () => opened, performanceReady: true });
