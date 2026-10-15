import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { I18nProvider } from "../../src/i18n";
import type { MangaPipelineStatusSummary } from "../../src/api/manga";
import { MangaChapterList } from "../../src/pages/novel-detail/manga/MangaChapterList";
import type { ChapterRow } from "../../src/pages/novel-detail/shared";
import "../../src/index.css";

const chapterCount = 1000;
const activeChapterId = 500;
const chapters: ChapterRow[] = Array.from({ length: chapterCount }, (_, index) => ({
  id: index + 1,
  chapter_number: index + 1,
  title: `性能章节 ${index + 1}`,
  status: "completed",
}));
const activeStatus = (completed: number): MangaPipelineStatusSummary => ({
  can_cancel: true,
  can_resume: false,
  cancelled: 0,
  chapter_id: activeChapterId,
  checkpoint_valid: true,
  completed,
  current: completed + 1,
  failed: 0,
  generating: 0,
  has_checkpoint: true,
  message: `运行中 ${completed}`,
  pending: 100 - completed,
  pipeline_status: "running",
  stage: "preprocessing",
  task_handle: null,
  total: 100,
});
const selection: Array<{ chapterId: number; revision: number }> = [];
let setStatuses: React.Dispatch<
  React.SetStateAction<Record<number, MangaPipelineStatusSummary | null>>
>;

function Fixture() {
  const [statuses, updateStatuses] = useState<Record<number, MangaPipelineStatusSummary | null>>({
    [activeChapterId]: activeStatus(0),
  });
  setStatuses = updateStatuses;
  return (
    <I18nProvider initialLocale="zh-CN">
      <MangaChapterList
        chapters={chapters}
        statuses={statuses}
        workspaceStage="preprocessing"
        selectedChapter={chapters[activeChapterId - 1]}
        onSelect={(chapter) => selection.push({
          chapterId: chapter.id,
          revision: statuses[activeChapterId]?.completed ?? 0,
        })}
      />
    </I18nProvider>
  );
}

const root = createRoot(document.getElementById("root")!);
flushSync(() => root.render(<Fixture />));

function percentile(values: number[], fraction: number): number {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1)];
}

async function measureMangaChapterList() {
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  const roundP50Ms: number[] = [];
  const roundP95Ms: number[] = [];
  let completed = 0;
  for (let round = 0; round < 3; round += 1) {
    for (let warmup = 0; warmup < 2; warmup += 1) {
      completed += 1;
      flushSync(() => setStatuses((current) => ({
        ...current,
        [activeChapterId]: activeStatus(completed),
      })));
    }
    const samples: number[] = [];
    for (let sample = 0; sample < 15; sample += 1) {
      completed += 1;
      const startedAt = performance.now();
      flushSync(() => setStatuses((current) => ({
        ...current,
        [activeChapterId]: activeStatus(completed),
      })));
      samples.push(performance.now() - startedAt);
    }
    roundP50Ms.push(percentile(samples, 0.5));
    roundP95Ms.push(percentile(samples, 0.95));
  }
  const activeChapterText = document.querySelectorAll("#root li button")[activeChapterId - 1]?.textContent ?? "";
  const markup = document.querySelector("#root ul")?.innerHTML ?? "";
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(markup));
  const resultSha256 = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
  return {
    chapterCount: document.querySelectorAll("#root li").length,
    activeChapterText: activeChapterText.replace(/\s+/g, " ").trim(),
    roundP50Ms,
    roundP95Ms,
    medianP50Ms: percentile(roundP50Ms, 0.5),
    medianP95Ms: percentile(roundP95Ms, 0.5),
    resultSha256,
  };
}

Object.assign(window, {
  getMangaChapterSelection: () => selection,
  measureMangaChapterList,
  performanceReady: true,
});
