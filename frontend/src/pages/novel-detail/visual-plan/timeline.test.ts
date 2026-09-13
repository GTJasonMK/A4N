import { describe, expect, it } from "vitest";
import type { ChapterVisualPlanV3 } from "../../../api/chapterVisualPlan";
import { pointAt, scaledPreview, sceneColor, segmentAt, segmentBackground } from "./timeline";

const plan: ChapterVisualPlanV3 = {
  schema_version: "chapter_visual_plan_v3", frame: { width: 1920, height: 1080 },
  points: [0, 2000, 4000, 6000, 8000, 10000, 12000].map((at_ms) => ({ id: String(at_ms), at_ms, label: "", description: "", image_id: at_ms || null, reference_image_ids: [] })),
  segments: [
    { id: "a", title: "A", start_point_id: "0", end_point_id: "6000", prompt: "动作", scene_binding: { kind: "scene", scene_profile_id: 1 }, use_end_point_image: false },
    { id: "transition", title: "转场", start_point_id: "6000", end_point_id: "8000", prompt: "换景", scene_binding: { kind: "transition", from_scene_profile_id: 1, to_scene_profile_id: 2 }, use_end_point_image: false },
    { id: "b", title: "B", start_point_id: "8000", end_point_id: "12000", prompt: "动作", scene_binding: { kind: "scene", scene_profile_id: 2 }, use_end_point_image: false },
  ],
};

describe("director timeline projections", () => {
  it("switches images at points independently of shot segments", () => {
    expect(pointAt(plan, 3000).at_ms).toBe(2000);
    expect(pointAt(plan, 5000).at_ms).toBe(4000);
    expect(segmentAt(plan, 3000)?.id).toBe("a");
    expect(segmentAt(plan, 5000)?.id).toBe("a");
    expect(pointAt(plan, 6000).at_ms).toBe(6000);
    expect(segmentAt(plan, 6000)?.id).toBe("transition");
    expect(pointAt(plan, 12000).at_ms).toBe(12000);
    expect(pointAt(plan, 0).image_id).toBeNull();
  });

  it("retimes every point but preserves bindings and segment identity", () => {
    const scaled = scaledPreview(plan, 6000);
    expect(scaled.points.map((point) => point.at_ms)).toEqual([0, 1000, 2000, 3000, 4000, 5000, 6000]);
    expect(scaled.segments).toBe(plan.segments);
    expect(plan.points.at(-1)?.at_ms).toBe(12000);
    expect(scaled.points[3].image_id).toBe(6000);
  });

  it("uses the same scene colors for solid segments and gradient endpoints", () => {
    expect(segmentBackground(plan.segments[0].scene_binding)).toBe(sceneColor(1));
    const gradient = segmentBackground(plan.segments[1].scene_binding);
    expect(gradient).toContain(sceneColor(1));
    expect(gradient).toContain(sceneColor(2));
  });
});
