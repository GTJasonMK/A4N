import React from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { ChapterSidebar } from "../../src/pages/writing-desk/ChapterSidebar";
import { countChapterBodyCharacters } from "../../src/pages/writing-desk/wordCount";
import { chapterIndexEntry } from "../../src/pages/writing-desk/chapterIndex";
import { I18nProvider, formatNumber, translateMessage, type Locale } from "../../src/i18n";
import { writeCurrentTokens } from "../../src/api/authStorage";
import type { ChapterSummary } from "../../src/api/chapter";
import "../../src/index.css";

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

const requests: string[] = [];
const unexpectedRequests: string[] = [];
window.fetch = async (input) => {
  const url = new URL(input instanceof Request ? input.url : String(input), location.href);
  const command = url.pathname.split("/").pop()!;
  requests.push(command);
  if (command !== "get_blueprint" && command !== "list_profiles") {
    unexpectedRequests.push(command);
    throw new Error(`Unexpected sidebar fixture request: ${command}`);
  }
  return new Response(JSON.stringify(command === "get_blueprint" ? null : []), {
    headers: { "Content-Type": "application/json" },
  });
};

function chapter(number: number, content: string): ChapterSummary {
  return {
    id: number, project_id: 1, chapter_number: number, title: `章节 ${number} 🙂`, content,
    // A stale stored count must not replace the existing code-point count of the body.
    word_count: 0, status: "draft", outline_id: null, edit_revision: `fixture-${number}`,
    summary: null, style_tags: null, analysis_data: null,
    analysis_sync_status: null, analysis_sync_can_resume: false,
    analysis_sync_has_saved_analysis: false, analysis_sync_resume_block_reason: null, analysis_sync_error: null,
    derived_sync_status: null, derived_sync_workflow_run_id: null, derived_sync_can_resume: false, derived_sync_error: null,
    material_sync: { content_index: "completed", outline_and_foreshadowing: "not_analyzed",
      objects: "not_analyzed", characters: "not_analyzed", scenes: "not_analyzed" },
    material_updates: [], created_at: null, updated_at: null,
  };
}

const element = document.getElementById("root")!;
const root = createRoot(element);
let sessionNumber = 0;
let selectionLog: number[] = [];

function session(chapters: ChapterSummary[], locale: Locale) {
  const key = ++sessionNumber;
  selectionLog = [];
  let props: React.ComponentProps<typeof ChapterSidebar> = {
    projectId: 1, chapters: chapters.map((chapter) => ({ ...chapter, ...chapterIndexEntry(chapter) })), outlines: [], chapterNumbers: chapters.map((item) => item.chapter_number),
    filteredChapterNumbers: chapters.map((item) => item.chapter_number), activeNumber: 1,
    activeEditorContent: chapters[0]?.content ?? "", activeOutline: null, chapterQuery: "", draftSet: new Set(),
    profilesRefreshKey: 0, onChapterQueryChange() {}, onClearChapterQuery() {}, onOpenProfiles() {},
    onRefreshChapters() {}, onCreateChapter() {},
    onSelectChapter(number) {
      selectionLog.push(number);
      render({ activeNumber: number, activeEditorContent: chapters.find((item) => item.chapter_number === number)?.content ?? "" });
    },
  };
  function render(changes: Partial<typeof props> = {}) {
    props = { ...props, ...changes };
    flushSync(() => root.render(<I18nProvider key={key} initialLocale={locale}><ChapterSidebar {...props} /></I18nProvider>));
  }
  render();
  return { render };
}

const settle = async () => {
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  check(unexpectedRequests.length === 0, `Unexpected requests: ${unexpectedRequests.join(", ")}`);
};

function cardCount(index: number, count: number | null, locale: Locale) {
  const card = element.querySelectorAll<HTMLButtonElement>(".a4n-chapter-card")[index];
  check(card, `Missing chapter card ${index}`);
  const meta = card.querySelector(".text-small.text-book-text-muted")?.textContent?.split(" · ")[0];
  const expected = count == null ? undefined : translateMessage(locale, "writing.chapterWordCount", { count: formatNumber(count, locale) });
  check(meta === expected, `Chapter ${index} count: ${meta} != ${expected}`);
}

async function verifySidebar(locale: Locale) {
  let chapters = [chapter(1, "猫🙂e\u0301"), chapter(2, ""), chapter(3, "正文\r\n𠮷")];
  const probe = session(chapters, locale);
  await settle();
  const trace: string[] = [];
  const capture = () => trace.push(element.textContent!);
  cardCount(0, 4, locale);
  cardCount(1, 0, locale);
  cardCount(2, 5, locale);
  capture();
  probe.render({ activeEditorContent: "新🙂" });
  cardCount(0, 2, locale);
  cardCount(2, 5, locale);
  capture();
  chapters = chapters.map((item) => item.id === 2 ? { ...item, content: "更新𠮷" } : item);
  probe.render({ chapters: chapters.map((chapter) => ({ ...chapter, ...chapterIndexEntry(chapter) })) });
  cardCount(1, 3, locale);
  capture();
  probe.render({ chapterNumbers: [1, 2, 3, 4], filteredChapterNumbers: [2, 4], activeNumber: 4,
    activeEditorContent: "🚀末", draftSet: new Set([4]) });
  cardCount(0, 3, locale);
  cardCount(1, 2, locale);
  capture();
  probe.render({ activeNumber: 2, activeEditorContent: "" });
  cardCount(0, 0, locale);
  cardCount(1, null, locale);
  capture();
  // Keyboard activation must use a replaced callback even when row data is unchanged.
  probe.render({ onSelectChapter: (number) => { selectionLog.push(number + 1000); } });
  await settle();
  return trace;
}

async function measureSidebar(chapterCount: number, characters: number) {
  const body = Array.from("正文🙂e\u0301".repeat(Math.ceil(characters / 5))).slice(0, characters).join("");
  const chapters = Array.from({ length: chapterCount }, (_, index) => chapter(index + 1, body + String(index)));
  const probe = session(chapters, "zh-CN");
  await settle();
  await document.fonts.ready;
  const updates = 25;
  const samples: number[] = [];
  const warmupUntil = performance.now() + 250;
  for (let run = 0; samples.length < 5; run++) {
    const started = performance.now();
    for (let update = 0; update < updates; update++) {
      probe.render({ activeEditorContent: chapters[0].content + "补".repeat(update) });
    }
    const elapsed = performance.now() - started;
    if (run >= 2 && started >= warmupUntil) samples.push(elapsed);
  }
  cardCount(0, countChapterBodyCharacters(chapters[0].content) + updates - 1, "zh-CN");
  cardCount(chapterCount - 1, countChapterBodyCharacters(chapters[chapterCount - 1].content), "zh-CN");
  check(element.querySelectorAll(".a4n-chapter-card").length === chapterCount, "Chapter rows disappeared");
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(element.innerHTML));
  const htmlSha256 = Array.from(new Uint8Array(digest), (value) => value.toString(16).padStart(2, "0")).join("");
  return { chapterCount, characters, updates, samplesMs: samples,
    medianMs: [...samples].sort((a, b) => a - b)[2], htmlSha256 };
}

void (async () => {
  await writeCurrentTokens({ session_id: "sidebar-fixture", access_token: "synthetic-token", refresh_token: "synthetic-refresh",
    token_type: "Bearer", expires_in: 3600 });
  Object.assign(window, { verifySidebar, measureSidebar, sidebarSelection: () => [...selectionLog], performanceReady: true });
})();
