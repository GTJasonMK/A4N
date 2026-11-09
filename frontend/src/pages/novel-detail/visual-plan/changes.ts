import type { ChapterVisualPlanV3, ChapterVisualPlanView } from "../../../api/chapterVisualPlan";
import type { MessageKey, MessageParams } from "../../../i18n/translate";
import { canonicalJson } from "../../../utils/contentHash";

type Translate = (key: MessageKey, params?: MessageParams) => string;

export interface PlanChange {
  collection: "frame" | "points" | "segments";
  id: string;
  field: string;
  before: unknown;
  after: unknown;
}

const equal = (left: unknown, right: unknown): boolean => left === right
  || (left !== undefined && right !== undefined && canonicalJson(left) === canonicalJson(right));
type Fields = Record<string, unknown>;
const fieldNameKeys: Record<string, MessageKey> = {
  label: "visualPlan.workspace.name",
  title: "visualPlan.workspace.shotName",
  description: "visualPlan.workspace.imageDescription",
  prompt: "visualPlan.workspace.changeDescription",
  at_ms: "visualPlan.workspace.timelinePosition",
  image_id: "visualPlan.workspace.boundStoryboardImage",
  reference_image_ids: "visualPlan.workspace.referenceImages",
  scene_binding: "visualPlan.workspace.sceneBinding",
  start_point_id: "visualPlan.workspace.startPoint",
  end_point_id: "visualPlan.workspace.endPoint",
  use_end_point_image: "visualPlan.workspace.useEndPointImage",
  width: "visualPlan.frameWidth",
  height: "visualPlan.frameHeight",
};

export function describePlanConflicts(plan: ChapterVisualPlanV3, changes: PlanChange[], t: Translate) {
  return changes.flatMap((change) => {
    const object = objects(plan, change.collection).get(change.id);
    const current = change.field ? object?.[change.field] : object;
    if (equal(current, change.before) || equal(current, change.after)) return [];
    const name = String(object?.label || object?.title || t(change.collection === "points"
      ? "visualPlan.workspace.conflictPoint"
      : change.collection === "segments"
        ? "visualPlan.workspace.conflictShot"
        : "visualPlan.workspace.conflictFrame"));
    return [{
      name,
      field: t(fieldNameKeys[change.field] ?? "visualPlan.workspace.conflictObject"),
      local: displayValue(current, t),
      remote: displayValue(change.after, t),
    }];
  });
}

function displayValue(value: unknown, t: Translate): string {
  if (value == null) return t("visualPlan.workspace.unsetOrDeleted");
  if (typeof value === "boolean") return value ? t("visualPlan.workspace.enabled") : t("visualPlan.workspace.disabledValue");
  if (typeof value === "object" && !Array.isArray(value)) {
    const fields = value as Fields;
    if (fields.kind === "scene") return t("visualPlan.timelineUi.sceneNumber", { id: String(fields.scene_profile_id) });
    if (fields.kind === "transition") return t("visualPlan.workspace.sceneTransitionValue", { from: String(fields.from_scene_profile_id), to: String(fields.to_scene_profile_id) });
    return String(fields.label || fields.title || t("visualPlan.workspace.retainedObject"));
  }
  return Array.isArray(value) ? value.join(t("common.listSeparator")) : String(value);
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

export function receiptChanges(saved: ChapterVisualPlanView, t: Translate): PlanChange[] {
  if (!saved.edit_receipt) throw new Error(t("visualPlan.workspace.missingEditReceipt"));
  return saved.edit_receipt.changes.map((change) => {
    if (change.collection !== "frame" && change.collection !== "points" && change.collection !== "segments") {
      throw new Error(t("visualPlan.workspace.unknownReceiptCollection"));
    }
    return { collection: change.collection, id: change.object_id, field: change.field,
      before: change.field === "" && change.before == null ? undefined : change.before,
      after: change.field === "" && change.after == null ? undefined : change.after };
  });
}

/** Reconcile an acknowledged document without replacing newer local fields. */
export function applyPlanChanges(
  plan: ChapterVisualPlanV3,
  changes: PlanChange[],
  resolution: "local" | "remote" | undefined,
  t: Translate,
): ChapterVisualPlanV3 {
  const next = structuredClone(plan);
  for (const change of changes) {
    const object = objects(next, change.collection).get(change.id);
    const current = change.field ? object?.[change.field] : object;
    if (equal(current, change.after)) continue;
    if (!equal(current, change.before) || (change.field && !object)) {
      if (resolution === "local") continue;
      if (resolution !== "remote" || (change.field && !object)) {
        const detail = describePlanConflicts(plan, [change], t)[0];
        throw new Error(t("visualPlan.workspace.concurrentConflict", {
          detail: detail ? `${detail.name} · ${detail.field}` : t("visualPlan.workspace.concurrentConflictDetail"),
        }));
      }
    }
    if (change.field) object![change.field] = structuredClone(change.after);
    else {
      if (change.collection === "frame") throw new Error(t("visualPlan.workspace.frameCannotDelete"));
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
      throw new Error(t("visualPlan.workspace.segmentReferencesDeletedPoint"));
    }
  }
  next.segments.sort((a, b) => times.get(a.start_point_id)! - times.get(b.start_point_id)!);
  return next;
}
