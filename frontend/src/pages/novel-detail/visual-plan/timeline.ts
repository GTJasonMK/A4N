import type { ChapterVisualPlanV3, SceneBindingV3, VisualPointV3, VisualSegmentV3 } from "../../../api/chapterVisualPlan";

export type DirectorSelection = { kind: "point" | "segment"; id: string };

export const durationMs = (plan: ChapterVisualPlanV3): number => plan.points.at(-1)?.at_ms ?? 0;

export function pointAt(plan: ChapterVisualPlanV3, time: number): VisualPointV3 {
  const index = plan.points.findIndex((point) => point.at_ms > time);
  return plan.points[index < 0 ? plan.points.length - 1 : Math.max(0, index - 1)];
}

export function segmentTimes(plan: ChapterVisualPlanV3, segment: VisualSegmentV3): [number, number] {
  const start = plan.points.find((point) => point.id === segment.start_point_id);
  const end = plan.points.find((point) => point.id === segment.end_point_id);
  if (!start || !end) throw new Error("镜头段引用的时间点不存在");
  return [start.at_ms, end.at_ms];
}

export function segmentAt(plan: ChapterVisualPlanV3, time: number): VisualSegmentV3 | undefined {
  return plan.segments.find((segment) => {
    const [start, end] = segmentTimes(plan, segment);
    return time >= start && time < end;
  }) ?? (time === durationMs(plan) ? plan.segments.at(-1) : undefined);
}

export function formatTime(time: number): string {
  const seconds = time / 1000;
  if (seconds < 60) return `${Number(seconds.toFixed(3))}s`;
  return `${Math.floor(seconds / 60)}:${(seconds % 60).toFixed(2).padStart(5, "0")}`;
}

export function sceneColor(sceneId: number): string {
  return `hsl(${(sceneId * 137.508) % 360} 48% 49%)`;
}

export function segmentBackground(binding: SceneBindingV3 | null): string {
  if (!binding) return "var(--director-empty-segment)";
  if (binding.kind === "scene") return sceneColor(binding.scene_profile_id);
  return `linear-gradient(90deg, ${sceneColor(binding.from_scene_profile_id)}, ${sceneColor(binding.to_scene_profile_id)})`;
}

export function sceneLabel(binding: SceneBindingV3 | null, names: ReadonlyMap<number, string>): string {
  if (!binding) return "未绑定场景";
  const label = (id: number) => names.get(id) ?? `场景 #${id}`;
  return binding.kind === "scene" ? label(binding.scene_profile_id)
    : `${label(binding.from_scene_profile_id)} → ${label(binding.to_scene_profile_id)}`;
}

/** Display-only gesture projection. The domain applies and validates the committed edit. */
export function scaledPreview(plan: ChapterVisualPlanV3, target: number): ChapterVisualPlanV3 {
  const total = durationMs(plan);
  return { ...plan, points: plan.points.map((point) => ({ ...point, at_ms: Math.round(point.at_ms * target / total) })) };
}
