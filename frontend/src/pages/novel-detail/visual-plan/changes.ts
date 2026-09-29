import type { ChapterVisualPlanV3, ChapterVisualPlanView } from "../../../api/chapterVisualPlan";

export interface PlanChange {
  collection: "frame" | "points" | "segments";
  id: string;
  field: string;
  before: unknown;
  after: unknown;
}

const equal = (left: unknown, right: unknown): boolean => JSON.stringify(left) === JSON.stringify(right);
type Fields = Record<string, unknown>;
const fieldNames: Record<string, string> = {
  label: "名称", title: "名称", description: "画面描述", prompt: "镜头描述", at_ms: "时间位置",
  image_id: "绑定图片", reference_image_ids: "参考图片", scene_binding: "场景",
  start_point_id: "起点", end_point_id: "终点", use_end_point_image: "结束点图片", width: "宽度", height: "高度",
};

export function describePlanConflicts(plan: ChapterVisualPlanV3, changes: PlanChange[]) {
  return changes.flatMap((change) => {
    const object = objects(plan, change.collection).get(change.id);
    const current = change.field ? object?.[change.field] : object;
    if (equal(current, change.before) || equal(current, change.after)) return [];
    const name = String(object?.label || object?.title || (change.collection === "points" ? "图片点" : change.collection === "segments" ? "镜头" : "画幅"));
    return [{ name, field: fieldNames[change.field] ?? "对象", local: displayValue(current), remote: displayValue(change.after) }];
  });
}

function displayValue(value: unknown): string {
  if (value == null) return "未设置或已删除";
  if (typeof value === "boolean") return value ? "启用" : "关闭";
  if (typeof value === "object" && !Array.isArray(value)) {
    const fields = value as Fields;
    if (fields.kind === "scene") return `场景 #${fields.scene_profile_id}`;
    if (fields.kind === "transition") return `场景 #${fields.from_scene_profile_id} → #${fields.to_scene_profile_id}`;
    return String(fields.label || fields.title || "保留对象");
  }
  return Array.isArray(value) ? value.join("、") : String(value);
}
const objects = (plan: ChapterVisualPlanV3, collection: PlanChange["collection"]): Map<string, Fields> =>
  collection === "frame"
    ? new Map([["frame", plan.frame as unknown as Fields]])
    : new Map(plan[collection].map((item) => [item.id, item as unknown as Fields]));

export function planChanges(before: ChapterVisualPlanV3, after: ChapterVisualPlanV3): PlanChange[] {
  const changes: PlanChange[] = [];
  for (const collection of ["frame", "points", "segments"] as const) {
    const left = objects(before, collection);
    const right = objects(after, collection);
    for (const id of new Set([...left.keys(), ...right.keys()])) {
      const old = left.get(id);
      const next = right.get(id);
      if (equal(old, next)) continue;
      if (!old || !next) changes.push({ collection, id, field: "", before: old, after: next });
      else for (const field of Object.keys(old)) {
        if (!equal(old[field], next[field])) changes.push({ collection, id, field, before: old[field], after: next[field] });
      }
    }
  }
  return changes;
}

export const inverseChanges = (changes: PlanChange[]): PlanChange[] =>
  changes.slice().reverse().map((change) => ({ ...change, before: change.after, after: change.before }));

export function receiptChanges(saved: ChapterVisualPlanView): PlanChange[] {
  if (!saved.edit_receipt) throw new Error("保存结果缺少编辑回执");
  return saved.edit_receipt.changes.map((change) => {
    if (change.collection !== "frame" && change.collection !== "points" && change.collection !== "segments") {
      throw new Error("编辑回执包含未知对象集合");
    }
    return { collection: change.collection, id: change.object_id, field: change.field,
      before: change.field === "" && change.before == null ? undefined : change.before,
      after: change.field === "" && change.after == null ? undefined : change.after };
  });
}

/** Reconcile an acknowledged document without replacing newer local fields. */
export function applyPlanChanges(plan: ChapterVisualPlanV3, changes: PlanChange[], resolution?: "local" | "remote"): ChapterVisualPlanV3 {
  const next = structuredClone(plan);
  for (const change of changes) {
    const object = objects(next, change.collection).get(change.id);
    const current = change.field ? object?.[change.field] : object;
    if (equal(current, change.after)) continue;
    if (!equal(current, change.before) || (change.field && !object)) {
      if (resolution === "local") continue;
      if (resolution !== "remote" || (change.field && !object)) {
        const detail = describePlanConflicts(plan, [change])[0];
        throw new Error(`并发修改冲突：${detail ? `${detail.name}的${detail.field}` : "时间线结构"}`);
      }
    }
    if (change.field) object![change.field] = structuredClone(change.after);
    else {
      if (change.collection === "frame") throw new Error("不能删除画幅");
      const items = next[change.collection] as unknown as Fields[];
      const index = items.findIndex((item) => item.id === change.id);
      if (index >= 0) items.splice(index, 1);
      if (change.after != null) items.push(structuredClone(change.after) as Fields);
    }
  }
  next.points.sort((a, b) => a.at_ms - b.at_ms);
  const times = new Map(next.points.map((point) => [point.id, point.at_ms]));
  for (const segment of next.segments) {
    if (!times.has(segment.start_point_id) || !times.has(segment.end_point_id)) {
      throw new Error("并发修改冲突：镜头引用的图片点已删除");
    }
  }
  next.segments.sort((a, b) => times.get(a.start_point_id)! - times.get(b.start_point_id)!);
  return next;
}
