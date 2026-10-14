import React, { useState } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import type { GalgameBranchDesign, MediaAuthoringWorkspace } from "../../src/api/mediaAuthoring";
import { I18nProvider } from "../../src/i18n";
import { GalgameBranchPlanEditor } from "../../src/pages/derived/MediaPlanEditors";
import "../../src/index.css";

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function makeWorkspace(chapterCount: number): MediaAuthoringWorkspace {
  const chapters = Array.from({ length: chapterCount }, (_, index) => ({
    chapter_number: index + 1,
    source: { source_chapter_id: index + 1 },
  }));
  const makeDocument = (chapterId: number, unitId: string, status: string, stale: boolean) => ({
    stale,
    record: {
      kind: "adaptation_plan",
      status,
      source_chapter_id: chapterId,
      document: {
        units: [{
          id: unitId,
          title: `叙事单元 ${unitId}`,
          beats: [{ id: `beat-${unitId}`, action: `动作 ${unitId}` }],
        }],
      },
    },
  });
  const documents = Array.from({ length: chapterCount }, (_, index) => {
    const chapterId = index + 1;
    return [
      makeDocument(chapterId, `stale-${chapterId}`, "adopted", true),
      makeDocument(chapterId, `unit-${chapterId}`, "adopted", false),
      makeDocument(chapterId, `draft-${chapterId}`, "draft", false),
      makeDocument(chapterId, `duplicate-${chapterId}`, "adopted", false),
    ];
  }).flat();
  return { chapters, documents } as unknown as MediaAuthoringWorkspace;
}

function makePlan(): GalgameBranchDesign {
  return {
    choices: [{
      after_beat_ref: { source_chapter_id: 1, unit_id: "unit-1", beat_id: "beat-unit-1" },
      question: "主角会怎么做？",
      options: [0, 1].map((index) => ({
        branch_id: `branch-${index + 1}`,
        option_text: `选择 ${index + 1}`,
        branch_title: `分支 ${index + 1}`,
        branch_objective: `目标 ${index + 1}`,
        dramatic_action: `行动 ${index + 1}`,
        reaction: `反应 ${index + 1}`,
        player_knowledge: [],
        returns_to: { source_chapter_id: index + 2, unit_id: `unit-${index + 2}` },
      })),
    }],
  };
}

function GalgameEditorHarness({
  source,
  workspace,
  run,
}: {
  source: GalgameBranchDesign;
  workspace: MediaAuthoringWorkspace;
  run: number;
}) {
  const [draft, setDraft] = useState(() => structuredClone(source));
  const dirty = JSON.stringify(draft) !== JSON.stringify(source);
  Object.assign(window, { galgameAuthoringDraft: draft, galgameAuthoringDirty: dirty });
  return (
    <I18nProvider key={run} initialLocale="zh-CN">
      <GalgameBranchPlanEditor value={draft} workspace={workspace} onChange={setDraft} />
    </I18nProvider>
  );
}

const rootElement = document.getElementById("root");
check(rootElement, "Missing Galgame authoring root");
const root = createRoot(rootElement);

async function settle() {
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
}

async function editSample(chapterCount: number, run: number, updates: number) {
  const source = makePlan();
  const workspace = makeWorkspace(chapterCount);
  const originalJson = JSON.stringify(source);
  flushSync(() => root.render(
    <GalgameEditorHarness key={run} source={source} workspace={workspace} run={run} />,
  ));
  await settle();

  const field = rootElement.querySelector<HTMLTextAreaElement>("textarea");
  check(field, "Missing Galgame branch question field");
  const valueSetter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
  check(valueSetter, "Missing textarea value setter");
  const samplesMs: number[] = [];
  let finalQuestion = "";
  for (let update = 0; update < updates; update++) {
    finalQuestion = `编辑后的分支问题 ${run}-${update}`;
    valueSetter.call(field, finalQuestion);
    const started = performance.now();
    flushSync(() => field.dispatchEvent(new InputEvent("input", { bubbles: true })));
    samplesMs.push(performance.now() - started);
  }
  await settle();
  check(window.galgameAuthoringDirty === true, "Editing did not mark the branch plan dirty");
  check(JSON.stringify(source) === originalJson, "Editing mutated the source branch plan");
  const expected = structuredClone(source);
  expected.choices[0].question = finalQuestion;
  const draft = window.galgameAuthoringDraft as GalgameBranchDesign;
  check(JSON.stringify(draft) === JSON.stringify(expected), "Saved branch plan changed unexpectedly");
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(JSON.stringify(draft)));
  const savedSha256 = Array.from(new Uint8Array(digest), (value) => value.toString(16).padStart(2, "0")).join("");
  return {
    chapterCount,
    workspaceBytes: new TextEncoder().encode(JSON.stringify(workspace)).byteLength,
    updates,
    samplesMs,
    savedSha256,
  };
}

async function verifyGalgameAuthoring() {
  const result = await editSample(5, 9001, 1);
  const selects = [...rootElement.querySelectorAll<HTMLButtonElement>('[role="combobox"]')];
  check(selects.length === 3, "Expected one beat selector and two return selectors");
  check(
    selects[0].textContent?.includes("动作 unit-1"),
    `The first adopted chapter plan should retain the beat selection (trigger=${selects[0].textContent})`,
  );
  check(selects[1].textContent?.includes("叙事单元 unit-2"), "The second branch should retain its return target");
  check(selects[2].textContent?.includes("叙事单元 unit-3"), "The third branch should retain its return target");
  return { chapterCount: result.chapterCount, savedSha256: result.savedSha256 };
}

async function measureGalgameAuthoring(chapterCount: number, updates: number) {
  const samples: number[] = [];
  let final: Awaited<ReturnType<typeof editSample>> | null = null;
  for (let run = 0; samples.length < 5; run++) {
    final = await editSample(chapterCount, 10000 + run, updates);
    const elapsed = final.samplesMs.reduce((sum, value) => sum + value, 0);
    if (run >= 2) samples.push(elapsed);
  }
  check(final, "Galgame authoring benchmark produced no sample");
  const sorted = [...samples].sort((left, right) => left - right);
  return {
    chapterCount,
    workspaceBytes: final.workspaceBytes,
    updates,
    samplesMs: samples,
    medianMs: sorted[Math.floor(sorted.length / 2)],
    perInputMedianMs: sorted[Math.floor(sorted.length / 2)] / updates,
    savedSha256: final.savedSha256,
  };
}

Object.assign(window, {
  verifyGalgameAuthoring,
  measureGalgameAuthoring,
  performanceReady: true,
});
