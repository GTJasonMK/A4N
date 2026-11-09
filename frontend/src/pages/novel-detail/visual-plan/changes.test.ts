import { describe, expect, it } from "vitest";
import type { ChapterVisualPlanV3 } from "../../../api/chapterVisualPlan";
import { translateMessage, type MessageKey, type MessageParams } from "../../../i18n/translate";
import { applyPlanChanges, inverseChanges, planChanges } from "./changes";

const t = (key: MessageKey, params?: MessageParams) => translateMessage("zh-CN", key, params);

const fixture = (): ChapterVisualPlanV3 => ({
  schema_version: "chapter_visual_plan_v3", frame: { width: 1920, height: 1080 },
  points: [
    { id: "start", at_ms: 0, label: "", description: "", image_id: null, reference_image_ids: [] },
    { id: "end", at_ms: 10000, label: "", description: "", image_id: null, reference_image_ids: [] },
  ],
  segments: [{ id: "shot", title: "", start_point_id: "start", end_point_id: "end", prompt: "", scene_binding: null, use_end_point_image: false }],
});

describe("native director reconciliation", () => {
  it("ignores object field order in transport round trips", () => {
    const plan = fixture();
    plan.segments[0].scene_binding = { kind: "transition", from_scene_profile_id: 11, to_scene_profile_id: 22 };
    const reordered = structuredClone(plan);
    reordered.frame = { height: plan.frame.height, width: plan.frame.width };
    reordered.segments[0].scene_binding = { to_scene_profile_id: 22, from_scene_profile_id: 11, kind: "transition" };
    expect(JSON.stringify(reordered)).not.toBe(JSON.stringify(plan));
    expect(planChanges(plan, reordered)).toEqual([]);
  });

  it("merges a real binding change when the local binding uses a different field order", () => {
    const base = fixture();
    base.segments[0].scene_binding = { kind: "transition", from_scene_profile_id: 11, to_scene_profile_id: 22 };
    const remote = structuredClone(base);
    remote.segments[0].scene_binding = { kind: "scene", scene_profile_id: 22 };
    const local = structuredClone(base);
    local.segments[0].scene_binding = { to_scene_profile_id: 22, from_scene_profile_id: 11, kind: "transition" };
    expect(applyPlanChanges(local, planChanges(base, remote), undefined, t)).toEqual(remote);
  });

  it("keeps absent points distinct from existing points during insertion and removal", () => {
    const base = fixture();
    const next = structuredClone(base);
    next.points.splice(1, 0, { ...next.points[0], id: "middle", at_ms: 5000 });
    const changes = planChanges(base, next);
    expect(changes).toHaveLength(1);
    expect(changes[0].before).toBeUndefined();
    expect(applyPlanChanges(base, changes, undefined, t)).toEqual(next);
    expect(applyPlanChanges(next, inverseChanges(changes), undefined, t)).toEqual(base);
  });

  it("merges disjoint fields, deduplicates receipts and selectively undoes an Agent batch", () => {
    const base = fixture();
    const agent = structuredClone(base);
    agent.points[0].description = "Agent frame";
    const batch = planChanges(base, agent);
    const manual = structuredClone(base);
    manual.points[0].label = "Manual label";
    const applied = applyPlanChanges(manual, batch, undefined, t);
    expect(applied.points[0]).toMatchObject({ label: "Manual label", description: "Agent frame" });
    expect(applyPlanChanges(applied, batch, undefined, t)).toEqual(applied);
    expect(applyPlanChanges(applied, inverseChanges(batch), undefined, t)).toEqual(manual);
  });
  it("keeps both inputs intact when the same field changed", () => {
    const base = fixture();
    const desired = structuredClone(base);
    desired.segments[0].prompt = "Agent";
    const manual = structuredClone(base);
    manual.segments[0].prompt = "Manual";
    expect(() => applyPlanChanges(manual, planChanges(base, desired), undefined, t)).toThrow("并发修改冲突");
    expect(manual.segments[0].prompt).toBe("Manual");
    expect(desired.segments[0].prompt).toBe("Agent");
  });
});
