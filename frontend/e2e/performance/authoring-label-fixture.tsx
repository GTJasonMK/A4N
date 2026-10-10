import React, { useState } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import type { MediaAuthoringWorkspace } from "../../src/api/mediaAuthoring";
import { I18nProvider, useI18n } from "../../src/i18n";
import { authoringStageLabel } from "../../src/pages/derived/useMediaAuthoring";
import "../../src/index.css";

const chapterCount = 1000;
const workspace = {
  source_snapshot_id: 1,
  chapters: Array.from({ length: chapterCount }, (_, index) => ({
    chapter_number: index + 1,
    source: { source_chapter_id: index + 1 },
    source_spans: [],
    title: `章节 ${index + 1}`,
  })),
  documents: Array.from({ length: chapterCount }, (_, index) => ({
    record: {
      kind: "adaptation_plan",
      source_chapter_id: index + 1,
      source_snapshot_id: 1,
      status: "adopted",
    },
    stale: false,
    video_reference_status: null,
  })),
  artifacts: [],
} as unknown as MediaAuthoringWorkspace;

let bump: (() => void) | null = null;
let currentLabel = "";

function Harness() {
  const [, setRevision] = useState(0);
  const { t } = useI18n();
  bump = () => setRevision((revision) => revision + 1);
  currentLabel = authoringStageLabel(workspace, "adaptation_plan", undefined, t) ?? "";
  return <output>{currentLabel}</output>;
}

const root = createRoot(document.getElementById("root")!);
flushSync(() => root.render(<I18nProvider initialLocale="zh-CN"><Harness /></I18nProvider>));

async function measureAuthoringLabel() {
  const samples: number[] = [];
  const warmupUntil = performance.now() + 250;
  for (let sample = 0; samples.length < 9; sample += 1) {
    const started = performance.now();
    for (let update = 0; update < 80; update += 1) {
      flushSync(() => bump?.());
    }
    const elapsed = performance.now() - started;
    if (sample >= 3 && started >= warmupUntil) samples.push(elapsed);
  }
  const sorted = [...samples].sort((left, right) => left - right);
  return {
    chapterCount,
    documentCount: workspace.documents.length,
    updates: 80,
    samplesMs: samples,
    medianMs: sorted[4],
    label: currentLabel,
  };
}

Object.assign(window, { measureAuthoringLabel, performanceReady: true });
