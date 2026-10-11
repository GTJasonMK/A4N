import React from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { I18nProvider, type Locale } from "../../src/i18n";
import type { ChapterOutline, ChapterOutlineListEntry } from "../../src/api/outline";
import { ChapterOutlineSection } from "../../src/pages/novel-detail/outlines/ChapterOutlineSection";
import "../../src/index.css";

const rootElement = document.getElementById("root")!;
const root = createRoot(rootElement);
let opened: number[] = [];
let renderKey = 0;

function listEntry(number: number): ChapterOutlineListEntry {
  return {
    id: number,
    project_id: 1,
    chapter_number: number,
    title: `线索 ${number}`,
    summary: `第 ${number} 章推进摘要：人物追查旧档案并发现新线索。`,
    word_count_target: 1500,
    source_kind: number % 4 === 0 ? "retrospective_analysis" : null,
    updated_at: `2026-10-12 10:${String(number % 60).padStart(2, "0")}:00`,
  };
}

function fullEntry(number: number): ChapterOutline {
  return {
    ...listEntry(number),
    part_id: null,
    beats: { items: [`节拍 ${number}`, `推进 ${number}`] },
    key_scenes: { scenes: [`关键场景 ${number}`] },
    pacing_score: null,
    foreshadowing_setup: { planted: [`线索 ${number}`] },
    foreshadowing_payoff: { resolved: [] },
    characters_involved: [`人物 ${number}`],
    status: "completed",
    created_at: "2026-10-12 09:00:00",
  };
}

function render(outlines: ChapterOutlineListEntry[], locale: Locale) {
  flushSync(() => root.render(
    <I18nProvider key={`${locale}-${++renderKey}`} initialLocale={locale}>
      <ChapterOutlineSection
        outlines={outlines}
        loaded
        canEdit
        chapterCount={1}
        generating={false}
        editingId={null}
        editForm={{ title: "", summary: "" }}
        savingId={null}
        regenId={null}
        delActing={null}
        onChapterCountChange={() => undefined}
        onGenerate={() => undefined}
        onPreviewGenerate={() => undefined}
        onCreate={() => undefined}
        onOpenDetail={(outline) => opened.push(outline.id)}
        onCancelEdit={() => undefined}
        onEditFormChange={() => undefined}
        onSave={() => undefined}
        onOpenAction={() => undefined}
      />
    </I18nProvider>,
  ));
}

async function hash(text: string) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return Array.from(new Uint8Array(digest), (value) => value.toString(16).padStart(2, "0")).join("");
}

async function verifyOutlineList(locale: Locale) {
  opened = [];
  const full = Array.from({ length: 1000 }, (_, index) => fullEntry(index + 1));
  const index = Array.from({ length: 1000 }, (_, offset) => listEntry(offset + 1));
  render(full, locale);
  await new Promise<void>((done) => requestAnimationFrame(() => done()));
  await document.fonts.ready;
  const fullHtml = rootElement.innerHTML;
  const fullText = rootElement.textContent ?? "";
  const itemCount = rootElement.querySelectorAll("li").length;
  const lastItem = rootElement.querySelector("li:last-child")?.textContent ?? "";
  render(index, locale);
  await new Promise<void>((done) => requestAnimationFrame(() => done()));
  await document.fonts.ready;
  const indexHtml = rootElement.innerHTML;
  const indexText = rootElement.textContent ?? "";
  if (rootElement.querySelectorAll("li").length !== itemCount) throw new Error("Outline rows changed");
  if ((rootElement.querySelector("li:last-child")?.textContent ?? "") !== lastItem) {
    throw new Error("Last outline changed");
  }
  return {
    locale,
    outlines: itemCount,
    htmlEqual: fullHtml === indexHtml,
    textEqual: fullText === indexText,
    fullHtmlSha256: await hash(fullHtml),
    indexHtmlSha256: await hash(indexHtml),
    fullTextSha256: await hash(fullText),
    indexTextSha256: await hash(indexText),
  };
}

Object.assign(window, {
  verifyOutlineList,
  outlineListOpened: () => opened,
  performanceReady: true,
});
