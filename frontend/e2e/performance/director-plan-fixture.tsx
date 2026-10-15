import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { useState } from "react";
import { useDirectorPlan } from "../../src/pages/novel-detail/visual-plan/useDirectorPlan";
import { writeFixtureAuth } from "../fixtureAuth";
import type { ChapterVisualPlanSetView, ChapterVisualPlanV3, ChapterVisualPlanView } from "../../src/api/chapterVisualPlan";

declare global {
  interface Window {
    directorPlanDirty: boolean;
    directorPlanReady: boolean;
    directorPlanSegmentCount: number;
    directorPlanValue: ChapterVisualPlanV3 | null;
    directorPlanEditorRevision: string;
    setDirectorPlanSize: (count: number) => void;
    updateDirectorPlanPrompt: (prompt: string) => void;
    verifyDirectorPlan: () => Promise<Record<string, unknown>>;
    measureDirectorPlan: (count: number, updates: number) => Promise<Record<string, unknown>>;
    performanceReady: boolean;
  }
}

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function makePlan(segmentCount: number): ChapterVisualPlanV3 {
  return {
    schema_version: "chapter_visual_plan_v3",
    frame: { width: 1920, height: 1080 },
    points: Array.from({ length: segmentCount + 1 }, (_, index) => ({
      id: `p${index}`,
      at_ms: index * 1000,
      label: `图片点 ${index}`,
      image_id: null,
      description: `描述 ${index}。${"原文细节。".repeat(8)}`,
      reference_image_ids: [],
    })),
    segments: Array.from({ length: segmentCount }, (_, index) => ({
      id: `s${index}`,
      title: `镜头 ${index}`,
      start_point_id: `p${index}`,
      end_point_id: `p${index + 1}`,
      prompt: `镜头描述 ${index}。${"旅人在雨夜走进旧书店。".repeat(12)}`,
      scene_binding: { kind: "scene", scene_profile_id: 11 },
      use_end_point_image: false,
    })),
  };
}

const recordView = (plan: ChapterVisualPlanV3, chapterId: number): ChapterVisualPlanView => ({
  edit_receipt: null,
  plan,
  record: {
    id: chapterId,
    plan_version: 1,
    revision: 1,
    status: "draft",
    authoring_mode: "collaborative",
    source_content_sha256: "a".repeat(64),
    plan_sha256: "b".repeat(64),
    confirmed_at: null,
  },
});

await writeFixtureAuth("director-plan-performance");
const originalFetch = window.fetch.bind(window);
window.fetch = async (resource, options) => {
  const url = new URL(typeof resource === "string" ? resource : resource instanceof URL ? resource.href : resource.url, location.href);
  if (url.pathname.endsWith("/visual-plan-edits")) {
    return new Response(new ReadableStream({
      start(controller) {
        options?.signal?.addEventListener("abort", () => controller.close(), { once: true });
      },
    }), { headers: { "content-type": "text/event-stream" } });
  }
  if (!url.pathname.includes("/commands/")) return originalFetch(resource, options);
  const command = decodeURIComponent(url.pathname.split("/").at(-1)!);
  const body = typeof options?.body === "string" ? JSON.parse(options.body) as { input?: Record<string, number> } : {};
  const chapterId = body.input?.source_chapter_id ?? 100;
  if (command === "get_chapter_visual_plan") {
    const plan = makePlan(chapterId);
    const draft = recordView(plan, chapterId);
    const result: ChapterVisualPlanSetView = {
      confirmed: null,
      draft,
      edit_cursor: 0,
      scene_choices: [{ id: 11, name: "雨夜街道", project_id: 1 }],
      source: { chapter_number: chapterId, title: `第 ${chapterId} 章`, content: "雨夜中，旅人走进旧书店。", content_sha256: "a".repeat(64) },
      starter_plan: plan,
    };
    return new Response(JSON.stringify(result), { status: 200, headers: { "content-type": "application/json" } });
  }
  if (command === "save_chapter_visual_plan_draft") {
    const plan = body.input?.plan as unknown as ChapterVisualPlanV3;
    return new Response(JSON.stringify(recordView(plan, chapterId)), { status: 200, headers: { "content-type": "application/json" } });
  }
  return new Response(JSON.stringify({ message: `Unknown performance fixture command: ${command}` }), { status: 501 });
};

function Probe() {
  const [chapterId, setChapterId] = useState(100);
  const model = useDirectorPlan(1, chapterId, true);
  window.directorPlanDirty = model.dirty;
  window.directorPlanReady = !model.loading && model.plan?.segments.length === chapterId;
  window.directorPlanSegmentCount = model.plan?.segments.length ?? 0;
  window.directorPlanValue = model.plan;
  window.directorPlanEditorRevision = model.editorRevision.current;
  window.setDirectorPlanSize = setChapterId;
  window.updateDirectorPlanPrompt = (prompt) => {
    const current = model.planRef.current;
    if (!current) throw new Error("Director plan is not loaded");
    const next = {
      ...current,
      segments: current.segments.map((segment, index) => index === 0 ? { ...segment, prompt } : segment),
    };
    flushSync(() => model.updateFields(next));
  };
  return <main>
    <h1>导演编排性能夹具</h1>
    <textarea aria-label="镜头描述" readOnly value={model.plan?.segments[0]?.prompt ?? ""} />
    <output>{model.dirty ? "未保存" : "已保存"}</output>
  </main>;
}

async function waitForPlan(count: number) {
  const until = Date.now() + 30_000;
  while (Date.now() < until) {
    if (window.directorPlanReady && window.directorPlanSegmentCount === count) return;
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  }
  throw new Error(`Director plan load timeout for ${count} segments`);
}

window.verifyDirectorPlan = async () => {
  await waitForPlan(100);
  const original = window.directorPlanValue!.segments[0].prompt;
  const initialRevision = window.directorPlanEditorRevision;
  window.updateDirectorPlanPrompt(`${"本地修改。".repeat(20)}`);
  check(window.directorPlanDirty, "editing a prompt must mark the plan dirty");
  check(window.directorPlanEditorRevision !== initialRevision, "editing a prompt must advance the editor revision");
  window.updateDirectorPlanPrompt(original);
  check(!window.directorPlanDirty, "reverting a prompt must clear dirty state");
  const saved = sessionStorage.getItem(`a4n:director-draft:${location.origin}:1:100`);
  check(saved === JSON.stringify(window.directorPlanValue), "the local recovery draft must match the edited plan");
  return { segments: 100, reverted: true, recoveryMatchesPlan: true };
};

window.measureDirectorPlan = async (count, updates) => {
  window.setDirectorPlanSize(count);
  await waitForPlan(count);
  const original = window.directorPlanValue!.segments[0].prompt;
  const samples = [];
  for (let sample = 0; sample < 7; sample++) {
    const started = performance.now();
    for (let update = 0; update < updates; update++) {
      const suffix = `${String(sample).padStart(2, "0")}${String(update).padStart(2, "0")}`;
      window.updateDirectorPlanPrompt(`${"x".repeat(Math.max(0, original.length - suffix.length))}${suffix}`);
    }
    const elapsed = performance.now() - started;
    if (sample >= 2) samples.push(elapsed);
  }
  const sorted = [...samples].sort((left, right) => left - right);
  const planBytes = new TextEncoder().encode(JSON.stringify(window.directorPlanValue)).length;
  check(window.directorPlanDirty, "measured edits must remain dirty");
  return { segments: count, updates, planBytes, samplesMs: samples, medianMs: sorted[2] };
};

createRoot(document.getElementById("root")!).render(<Probe />);
window.performanceReady = true;
