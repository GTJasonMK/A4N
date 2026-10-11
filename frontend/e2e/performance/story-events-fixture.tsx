import React from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { I18nProvider, translateMessage, type Locale } from "../../src/i18n";
import { StoryEventsListCard } from "../../src/pages/novel-detail/story-events/StoryEventsListCard";
import { DEFAULT_STORY_EVENT_FILTERS, extractChapterStorySection, filterStorySections, getStoryEventFilterOptions } from "../../src/pages/novel-detail/story-events/storyEventUtils";
import type { ChapterAnalysisEntry, ChapterStoryEventEntry, ChapterSummary } from "../../src/api/chapter";
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
  const gc = (globalThis as typeof globalThis & { gc?: () => void }).gc;
  const memory = (performance as Performance & { memory?: { usedJSHeapSize: number } }).memory;
  if (gc) gc();
  const initialHeapBytes = memory?.usedJSHeapSize ?? null;
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
  const renderedTextHash = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(rootElement.textContent ?? ""));
  const semanticText = (rootElement.textContent ?? "")
    .replace(/\s*·\s*/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  const semanticTextHash = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(semanticText));
  if (gc) gc();
  return { chapters: count, events: count * 5, samplesMs: measurements,
    medianMs: [...measurements].sort((a, b) => a - b)[2],
    initial,
    initialHeapBytes,
    finalHeapBytes: memory?.usedJSHeapSize ?? null,
    semanticTextSha256: Array.from(new Uint8Array(semanticTextHash), (value) => value.toString(16).padStart(2, "0")).join(""),
    filterSamplesMs: filterSamples,
    reactCommitSamplesMs: commitSamples,
    htmlSha256: Array.from(new Uint8Array(hash), (value) => value.toString(16).padStart(2, "0")).join(""),
    renderedTextSha256: Array.from(new Uint8Array(renderedTextHash), (value) => value.toString(16).padStart(2, "0")).join("") };
}

function fullStoryEventAnalysis(chapterNumber: number): NonNullable<ChapterAnalysisEntry["analysis_data"]> {
  const characters = ["林舟", "沈青", "陈墨"];
  const repeated = (value: string, count: number) => value.repeat(count);
  const characterStates = Object.fromEntries(characters.map((character, index) => [character, {
    location: repeated(`第${chapterNumber}章所在地点记录。`, 4),
    status: repeated("正在追查账本留下的线索。", 4),
    changes: [repeated("发现一项新的证据。", 3), repeated("与同伴核对事件经过。", 3)],
    emotional_state: `保持警觉 ${index}`,
  }]));
  return {
    metadata: {
      characters,
      locations: ["旧城", "河岸"],
      items: ["账本", repeated("未用物件标签。", 8)],
      tags: ["调查", repeated("未用分类标签。", 8)],
      tone: "紧张",
      timeline_marker: `第${chapterNumber}天`,
    },
    summaries: {
      compressed: `第${chapterNumber}章调查账本的来历`,
      one_line: "账本现身",
      keywords: ["账本", "调查"],
    },
    character_states: characterStates,
    world_facts: [
      {
        field: "history_timeline",
        subject: "旧城历史",
        statement: repeated("旧城曾发生一场影响组织势力变化的冲突。", 3),
        assertion: "reported",
        source_span_ids: [`chapter:${chapterNumber}:span:1`],
        evidence: [repeated("正文支持旧城历史变化。", 3)],
      },
      {
        field: "core_rules",
        subject: "账本记录规则",
        statement: repeated("账本会记录交易双方以及日期。", 3),
        assertion: "observed",
        source_span_ids: [`chapter:${chapterNumber}:span:2`],
        evidence: [repeated("账本内容显示交易信息。", 3)],
      },
    ],
    foreshadowing: {
      planted: [{
        description: repeated("异常来客留下未解线索。", 3),
        original_text: repeated("原始正文伏笔片段。", 8),
        category: "plot_twist",
        priority: "high",
        related_entities: characters,
      }],
      resolved: [{
        id: `foreshadowing-${chapterNumber}`,
        resolution: repeated("线索在本章得到解释。", 4),
        source_span_ids: [`chapter:${chapterNumber}:span:3`],
      }],
      tensions: [repeated("尚未解决的矛盾。", 4)],
      progressed: [{
        id: `progressed-${chapterNumber}`,
        status: "partially_revealed",
        resolution: repeated("伏笔推进说明。", 3),
        source_span_ids: [`chapter:${chapterNumber}:span:4`],
      }],
    },
    key_events: Array.from({ length: 5 }, (_, event) => ({
      description: `线索${event}：${chapterNumber}章发现账本记录 ${"灯火照亮河岸，林舟追寻来客留下的足迹。".repeat(5)}`,
      type: "discovery",
      importance: "medium",
      involved_characters: [characters[event % characters.length]],
    })),
  } as NonNullable<ChapterAnalysisEntry["analysis_data"]>;
}

function storyEventEntry(chapterNumber: number): ChapterAnalysisEntry {
  const source = chapter(chapterNumber);
  return {
    id: source.id,
    chapter_number: source.chapter_number,
    title: source.title,
    summary: source.summary,
    analysis_data: chapterNumber % 10 === 0 ? null : fullStoryEventAnalysis(chapterNumber),
  };
}

function objectFields(value: unknown, fields: string[]): unknown {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return value;
  const source = value as Record<string, unknown>;
  return Object.fromEntries(fields.filter((field) => field in source).map((field) => [field, source[field]]));
}

function storyEventProjection(entry: ChapterAnalysisEntry): ChapterStoryEventEntry {
  const analysis = entry.analysis_data;
  if (analysis === null) {
    return { ...entry, foreshadowing_counts: { planted: 0, resolved: 0, tensions: 0 } };
  }
  const source = analysis as Record<string, unknown>;
  const foreshadowing = source.foreshadowing as Record<string, unknown> | null | undefined;
  const listLength = (value: unknown) => Array.isArray(value) ? value.length : 0;
  return {
    ...entry,
    analysis_data: {
      metadata: objectFields(source.metadata, ["characters", "locations", "tone", "timeline_marker"]),
      summaries: objectFields(source.summaries, ["compressed", "summary", "short", "one_line", "keywords"]),
      key_events: source.key_events,
    } as NonNullable<ChapterAnalysisEntry["analysis_data"]>,
    foreshadowing_counts: {
      planted: listLength(foreshadowing?.planted),
      resolved: listLength(foreshadowing?.resolved),
      tensions: listLength(foreshadowing?.tensions),
    },
  };
}

function storyEventSection(entry: ChapterAnalysisEntry | ChapterStoryEventEntry, locale: Locale) {
  const t = (messageKey: Parameters<typeof translateMessage>[1], params?: Parameters<typeof translateMessage>[2]) =>
    translateMessage(locale, messageKey, params);
  return extractChapterStorySection(entry as ChapterStoryEventEntry, t);
}

let projectionState: {
  locale: Locale;
  key: number;
  sections: ReturnType<typeof storyEventSection>[];
  filterOptions: ReturnType<typeof getStoryEventFilterOptions>;
  filters: typeof DEFAULT_STORY_EVENT_FILTERS;
  chaptersCount: number;
} | null = null;

function renderProjectionState() {
  if (!projectionState) throw new Error("Story event projection is not loaded");
  const state = projectionState;
  const visible = filterStorySections(state.sections, state.filters);
  flushSync(() => root.render(
    <I18nProvider initialLocale={state.locale} key={state.key}>
      <StoryEventsListCard
        chaptersCount={state.chaptersCount}
        analyzedChapters={state.sections.filter((section) => section.hasAnalysis).length}
        sections={visible}
        filters={state.filters}
        filterOptions={state.filterOptions}
        onFiltersChange={(filters) => {
          if (projectionState) {
            projectionState.filters = filters;
            renderProjectionState();
          }
        }}
        onOpenChapter={(chapterNumber) => opened.push(chapterNumber)}
      />
    </I18nProvider>,
  ));
  return visible;
}

async function sha256(text: string) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return Array.from(new Uint8Array(digest), (value) => value.toString(16).padStart(2, "0")).join("");
}

async function measureStoryEventAnalysisProjection(
  locale: Locale,
  projection: "full" | "projected",
) {
  const garbageCollect = (globalThis as typeof globalThis & { gc?: () => void }).gc;
  const readHeapBytes = () => (performance as Performance & { memory?: { usedJSHeapSize: number } })
    .memory?.usedJSHeapSize;
  if (!garbageCollect || readHeapBytes() == null) throw new Error("Chromium heap measurement is unavailable");
  garbageCollect();
  const baselineHeapBytes = readHeapBytes()!;
  opened = [];
  let responseRows: Array<ChapterAnalysisEntry | ChapterStoryEventEntry> = Array.from(
    { length: 1_000 },
    (_, index) => storyEventEntry(index + 1),
  );
  if (projection === "projected") responseRows = responseRows.map(storyEventProjection);
  let responseJson = JSON.stringify(responseRows);
  const responseBytes = new TextEncoder().encode(responseJson).length;
  let compressed = await new Response(
    new Blob([responseJson]).stream().pipeThrough(new CompressionStream("gzip")),
  ).arrayBuffer();
  const gzipBytes = compressed.byteLength;
  compressed = new ArrayBuffer(0);
  let receivedRows = JSON.parse(responseJson) as Array<ChapterAnalysisEntry | ChapterStoryEventEntry>;
  responseRows = [];
  responseJson = "";
  const key = ++version;
  const sections = receivedRows.map((entry) => storyEventSection(entry, locale));
  receivedRows = [];
  projectionState = {
    locale,
    key,
    sections,
    filterOptions: getStoryEventFilterOptions(sections.flatMap((section) => section.events)),
    filters: { ...DEFAULT_STORY_EVENT_FILTERS },
    chaptersCount: 1_000,
  };
  const commitStart = performance.now();
  const visible = renderProjectionState();
  const commitMs = performance.now() - commitStart;
  await settle();
  garbageCollect();
  const heapBytes = readHeapBytes()!;
  const html = rootElement.innerHTML;
  const text = rootElement.textContent ?? "";
  return {
    projection,
    responseBytes,
    gzipBytes,
    heapBytes,
    baselineHeapBytes,
    heapDeltaBytes: heapBytes - baselineHeapBytes,
    chapters: rootElement.querySelectorAll(".a4n-story-event-section").length,
    analyzedChapters: sections.filter((section) => section.hasAnalysis).length,
    visibleChapters: visible.length,
    eventRows: rootElement.querySelectorAll(".a4n-story-event-section ol > li").length,
    domElements: rootElement.querySelectorAll("*").length,
    commitMs,
    htmlSha256: await sha256(html),
    textSha256: await sha256(text),
  };
}

async function filterStoryEventProjection(query: string) {
  if (!projectionState) throw new Error("Story event projection is not loaded");
  projectionState.filters = { ...projectionState.filters, query };
  const visible = renderProjectionState();
  return {
    visibleChapters: visible.length,
    eventRows: rootElement.querySelectorAll(".a4n-story-event-section ol > li").length,
    text: rootElement.textContent ?? "",
  };
}

Object.assign(window, {
  verifyStoryEvents,
  measureStoryEvents,
  measureStoryEventAnalysisProjection,
  filterStoryEventProjection,
  storyEventsOpened: () => opened,
  performanceReady: true,
});
