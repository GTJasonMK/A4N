import React, { useState } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import type {
  ChapterShootingScriptV2,
  ShootingSceneV2,
  ShootingShotV2,
} from "../../src/api/shootingScript";
import { I18nProvider } from "../../src/i18n";
import { StructuredScriptEditor } from "../../src/pages/novel-detail/shooting-script/StructuredScriptEditor";
import "../../src/index.css";

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function makeShot(index: number): ShootingShotV2 {
  return {
    shot_id: `shot-${index + 1}`,
    shot_index: index + 1,
    title: `镜头标题 ${index + 1}`,
    narrative_purpose: `叙事目的 ${index + 1}。`.repeat(3),
    subject: `主体 ${index + 1}`,
    composition: `构图 ${index + 1}。`.repeat(3),
    lens_intent: `镜头意图 ${index + 1}`,
    shot_size: "medium",
    camera_angle: "eye_level",
    camera_height: "eye",
    camera_movement: "locked",
    coverage: "must_show",
    planned_start_ms: index * 6000,
    planned_end_ms: index * 6000 + 5000,
    blocking: { start: "起始站位", movement: "向前一步", end: "停在桌边" },
    spatial_anchors: ["窗边", "桌面"],
    continuity_requirements: ["保持视线方向"],
    camera_start_state: "中景",
    camera_end_state: "近景",
    start_state: "人物站立",
    end_state: "人物坐下",
    transition_in: "接上一个镜头的动作",
    transition_out: "留出反应时间",
    sound: { dialogue: [], environment: ["室内底噪"], effects: [], music: [], silence: [] },
    performance_beats: [],
    dialogue_cues: [],
    source_span_ids: [`span-${index + 1}`],
    user_addition_ids: [],
    visual_shot_id: null,
    visual_constraints: null,
  };
}

function makeScene(sceneIndex: number, shots: ShootingShotV2[]): ShootingSceneV2 {
  return {
    scene_identity: `scene-${sceneIndex + 1}`,
    scene_index: sceneIndex + 1,
    sequence_index: sceneIndex + 1,
    narrative_unit_index: sceneIndex + 1,
    timeline_position: sceneIndex + 1,
    title: `场景标题 ${sceneIndex + 1}`,
    slugline: `内景 · 房间 ${sceneIndex + 1} · 日`,
    dramatic_question: "人物会不会说出真相？",
    scene_objective: "找到关键线索",
    scene_obstacle: "同伴隐瞒信息",
    location: `房间 ${sceneIndex + 1}`,
    time_of_day: "日",
    participants: ["主角", "同伴"],
    focal_character: "主角",
    editing_mode: "continuity",
    viewpoint: {
      mode: "limited_third_person",
      anchor_character: "主角",
      information_strategy: "只呈现主角能观察到的信息",
    },
    relationship_before: "互相试探",
    relationship_after: "形成信任",
    entry_state: "主角进入房间",
    exit_state: "主角拿到线索",
    audience_knowledge_before: [],
    audience_knowledge_after: [`发现线索 ${sceneIndex + 1}`],
    withheld_information: [],
    spatial_continuity: {
      continuity_intent: "transparent",
      primary_axis: "桌子与门之间",
      screen_direction: "主角由左向右移动",
      reorientation_strategy: null,
    },
    sound_strategy: "保留环境声",
    transition_contracts: [],
    shots,
    source_span_ids: [`scene-span-${sceneIndex + 1}`],
    user_addition_ids: [],
    intercut_group_id: null,
    visual_narrative_unit_id: null,
  };
}

function makeScript(shotCount: number): ChapterShootingScriptV2 {
  const shotsPerScene = 5;
  const scenes: ShootingSceneV2[] = [];
  let nextShotIndex = 0;
  while (nextShotIndex < shotCount) {
    const sceneShots = Array.from(
      { length: Math.min(shotsPerScene, shotCount - nextShotIndex) },
      () => makeShot(nextShotIndex++),
    );
    scenes.push(makeScene(scenes.length, sceneShots));
  }
  return {
    schema_version: "chapter_shooting_script_v2",
    directing_statement: null,
    scenes,
    user_additions: [],
    visual_plan_source: null,
  };
}

function ScriptHarness({ source, run }: { source: ChapterShootingScriptV2; run: number }) {
  const [draft, setDraft] = useState(() => structuredClone(source));
  const dirty = JSON.stringify(source) !== JSON.stringify(draft);
  Object.assign(window, { shootingScriptDraft: draft, shootingScriptDirty: dirty });
  return (
    <I18nProvider key={run} initialLocale="zh-CN">
      <StructuredScriptEditor script={draft} onChange={setDraft} />
    </I18nProvider>
  );
}

const rootElement = document.getElementById("root");
check(rootElement, "Missing shooting script root");
const root = createRoot(rootElement);

async function settle() {
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
}

async function editSample(shotCount: number, run: number, updates: number) {
  const source = makeScript(shotCount);
  const originalJson = JSON.stringify(source);
  flushSync(() => root.render(<ScriptHarness key={run} source={source} run={run} />));
  await settle();

  const shot = rootElement.querySelector<HTMLDetailsElement>('[data-shot-id="shot-1"]');
  check(shot, "Missing first shot editor");
  for (let element: HTMLElement | null = shot; element; element = element.parentElement) {
    if (element instanceof HTMLDetailsElement) element.open = true;
  }
  const valueSetter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
  check(valueSetter, "Missing textarea value setter");
  const changeTextarea = (index: number, value: string) => {
    const field = shot.querySelectorAll<HTMLTextAreaElement>("textarea")[index];
    check(field, `Missing first shot textarea ${index}`);
    valueSetter.call(field, value);
    flushSync(() => field.dispatchEvent(new InputEvent("input", { bubbles: true })));
  };

  const samplesMs: number[] = [];
  let finalTitle = "";
  for (let update = 0; update < updates; update++) {
    finalTitle = `编辑后的镜头标题 ${run}-${update}`;
    const started = performance.now();
    changeTextarea(0, finalTitle);
    samplesMs.push(performance.now() - started);
  }
  await settle();
  if (run === 5001) {
    check(window.shootingScriptDirty === true, "Editing did not mark the script dirty");
    changeTextarea(3, "更新后的构图说明");
    await settle();
  }
  check(JSON.stringify(source) === originalJson, "Editing mutated the source script");
  check(window.shootingScriptDirty === true, "Updated script should be dirty");

  const expected = structuredClone(source);
  expected.scenes[0].shots[0].title = finalTitle;
  if (run === 5001) expected.scenes[0].shots[0].composition = "更新后的构图说明";
  const draft = window.shootingScriptDraft as ChapterShootingScriptV2;
  check(JSON.stringify(draft) === JSON.stringify(expected), "Edited shooting script changed unexpectedly");
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(JSON.stringify(draft)));
  const savedSha256 = Array.from(new Uint8Array(digest), (value) => value.toString(16).padStart(2, "0")).join("");
  return {
    shotCount,
    planBytes: new TextEncoder().encode(originalJson).byteLength,
    updates,
    samplesMs,
    savedSha256,
  };
}

async function verifyShootingScriptEditor() {
  const result = await editSample(5, 5001, 1);
  return { shotCount: result.shotCount, savedSha256: result.savedSha256 };
}

async function measureShootingScriptEditor(shotCount: number, updates: number) {
  const samples: number[] = [];
  let final: Awaited<ReturnType<typeof editSample>> | null = null;
  for (let run = 0; samples.length < 5; run++) {
    final = await editSample(shotCount, 6000 + run, updates);
    const elapsed = final.samplesMs.reduce((sum, value) => sum + value, 0);
    if (run >= 2) samples.push(elapsed);
  }
  check(final, "Shooting script benchmark produced no sample");
  const sorted = [...samples].sort((left, right) => left - right);
  return {
    shotCount,
    planBytes: final.planBytes,
    updates,
    samplesMs: samples,
    medianMs: sorted[Math.floor(sorted.length / 2)],
    perInputMedianMs: sorted[Math.floor(sorted.length / 2)] / updates,
    savedSha256: final.savedSha256,
  };
}

Object.assign(window, {
  verifyShootingScriptEditor,
  measureShootingScriptEditor,
  performanceReady: true,
});
