import React from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import type { ChapterVideoPrompt, VideoSegmentPlanV2 } from "../../src/api/video";
import { I18nProvider } from "../../src/i18n";
import { DEFAULT_VIDEO_OPTIONS } from "../../src/pages/novel-detail/video/videoTabConstants";
import { videoDurationLimitsFromConfig } from "../../src/pages/novel-detail/video/videoOptionUtils";
import { VideoPlanningWorkspace } from "../../src/pages/novel-detail/video/VideoPlanningWorkspace";
import "../../src/index.css";

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function makeSegment(index: number) {
  return {
    schema_version: "video_segment_plan_v2",
    segment_number: index + 1,
    segment_index_in_shot: 1,
    shot_segment_count: 1,
    source_shot_id: `shot-${index + 1}`,
    source_span_ids: [`span-${index + 1}`],
    covered_beat_ids: [`beat-${index + 1}`],
    title: `片段标题 ${index + 1}`,
    scene_summary: `场景摘要 ${index + 1}。`.repeat(4),
    blocking_summary: `调度说明 ${index + 1}。`.repeat(3),
    duration_secs: 6,
    aspect_ratio: "16:9",
    reference_image_ids: [],
    spatial_anchors: [],
    emotion_action_goals: [],
    video_intent: {
      hook: "开场动作",
      audience_effect: "引起注意",
      rhythm: "逐渐加快",
      visual_priority: "人物动作",
    },
    emotional_arc: {
      start: "平静",
      turn: "发现线索",
      end: "决定行动",
      function: "推进情节",
    },
    performance_plan: {
      movement_path: "由左向右",
      body_performance: "保持克制",
      gaze_and_hands: "视线落在道具上",
      pause_or_reaction: "短暂停顿",
      camera_relationship: "近景跟随",
      time_blocks: [],
      action_script: [],
    },
    material_coverage: {
      required_entities: [],
      missing_or_weak_references: [],
      risk_note: "",
    },
    transition_to_next: null,
    director_constraints: null,
  } satisfies VideoSegmentPlanV2;
}

function makePrompt(segments: VideoSegmentPlanV2[], id: number): ChapterVideoPrompt {
  return {
    id,
    project_id: 19,
    chapter_id: 3,
    style: null,
    prompt_type: "video_segment_plan",
    workflow_mode: "shared_segment_plan",
    shot_number: 0,
    title: "片段规划",
    scene_summary: null,
    visual_prompt: "",
    motion_prompt: null,
    negative_prompt: null,
    duration_secs: 6,
    aspect_ratio: "16:9",
    reference_image_ids_json: null,
    status: "completed",
    generated_video_id: null,
    metadata_json: { segments },
    created_at: "2026-07-29T00:00:00Z",
    updated_at: "2026-07-29T00:00:00Z",
  };
}

const rootElement = document.getElementById("root");
check(rootElement, "Missing video planning root");
const root = createRoot(rootElement);
const durationLimits = videoDurationLimitsFromConfig(null);
let dirty = false;

async function settle() {
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
}

async function editSample(segmentCount: number, run: number, updates: number) {
  const source = Array.from({ length: segmentCount }, (_, index) => makeSegment(index));
  const originalJson = JSON.stringify(source);
  let saved: VideoSegmentPlanV2[] | null = null;
  const onSavePlan = async (segments: VideoSegmentPlanV2[]) => {
    saved = structuredClone(segments);
    return true;
  };

  flushSync(() => root.render(
    <I18nProvider key={run} initialLocale="zh-CN">
      <VideoPlanningWorkspace
        prompt={makePrompt(source, run + 1)}
        previewSegments={[]}
        draft={null}
        options={DEFAULT_VIDEO_OPTIONS}
        durationLimits={durationLimits}
        loading={false}
        loadError={null}
        dataReady
        acting={null}
        shootingScenes={[]}
        onGenerate={() => {}}
        onOptionsChange={() => {}}
        onRetryLoad={() => {}}
        onDelete={() => {}}
        onSavePlan={onSavePlan}
        onDirtyChange={(value) => { dirty = value; }}
      />
    </I18nProvider>,
  ));
  await settle();

  const editButton = rootElement.querySelector("button");
  check(editButton, "Missing edit plan button");
  flushSync(() => editButton.click());
  const valueSetter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
  check(valueSetter, "Missing textarea value setter");
  const dispatchTextareaChange = (index: number, value: string) => {
    const field = rootElement.querySelectorAll<HTMLTextAreaElement>("fieldset textarea")[index];
    check(field, `Missing segment textarea ${index}`);
    valueSetter.call(field, value);
    flushSync(() => field.dispatchEvent(new InputEvent("input", { bubbles: true })));
  };

  const samples: number[] = [];
  let finalTitle = "";
  for (let update = 0; update < updates; update++) {
    finalTitle = `编辑后的片段标题 ${run}-${update}`;
    const started = performance.now();
    dispatchTextareaChange(0, finalTitle);
    samples.push(performance.now() - started);
  }

  await settle();
  check(dirty, "Edited plan did not become dirty");
  if (run === 1001) {
    dispatchTextareaChange(0, source[0].title);
    await settle();
    check(!dirty, "Restoring the original value should clear the dirty state");
    const saveButton = [...rootElement.querySelectorAll("section button")][1];
    check(saveButton?.disabled, "Restoring the original value should disable Save");
    dispatchTextareaChange(0, finalTitle);
    dispatchTextareaChange(3, "更新后的开场钩子");
    await settle();
    check(dirty, "Changing the restored plan should mark it dirty again");
  }
  check(JSON.stringify(source) === originalJson, "Editing mutated the source segment plan");
  const editorRows = rootElement.querySelectorAll("fieldset > details").length;
  check(editorRows === segmentCount, `Expected ${segmentCount} editor rows, found ${editorRows}`);
  const buttons = [...rootElement.querySelectorAll("section button")];
  check(buttons[1] && !buttons[1].disabled, "Save button should be enabled after editing");
  flushSync(() => buttons[1].click());
  await settle();

  const expected = structuredClone(source);
  expected[0].title = finalTitle;
  if (run === 1001) expected[0].video_intent.hook = "更新后的开场钩子";
  check(saved != null && JSON.stringify(saved) === JSON.stringify(expected), "Saved segment plan changed unexpectedly");
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(JSON.stringify(saved)),
  );
  const savedSha256 = Array.from(new Uint8Array(digest), (value) => value.toString(16).padStart(2, "0")).join("");
  return {
    segmentCount,
    planBytes: new TextEncoder().encode(originalJson).byteLength,
    editorRows,
    updates,
    samplesMs: samples,
    savedSha256,
    dirty,
  };
}

async function verifyVideoPlanning() {
  const result = await editSample(6, 1001, 1);
  return { savedSha256: result.savedSha256, dirty: result.dirty, editorRows: result.editorRows };
}

async function measureVideoPlanning(segmentCount: number, updates: number) {
  const samples: number[] = [];
  let final: Awaited<ReturnType<typeof editSample>> | null = null;
  for (let run = 0; samples.length < 5; run++) {
    final = await editSample(segmentCount, 2000 + run, updates);
    const elapsed = final.samplesMs.reduce((sum, value) => sum + value, 0);
    if (run >= 2) samples.push(elapsed);
  }
  check(final, "Video planning benchmark produced no sample");
  const sorted = [...samples].sort((left, right) => left - right);
  return {
    segmentCount,
    planBytes: final.planBytes,
    updates,
    samplesMs: samples,
    medianMs: sorted[Math.floor(sorted.length / 2)],
    perInputMedianMs: sorted[Math.floor(sorted.length / 2)] / updates,
    savedSha256: final.savedSha256,
  };
}

Object.assign(window, { verifyVideoPlanning, measureVideoPlanning, performanceReady: true });
