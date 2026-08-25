import type {
  ChapterVisualPlanV1,
  VisualCameraEventV1,
  VisualEntityEventV1,
  VisualNarrativeUnitV1,
  VisualShotPlanV1,
  VisualSoundEventV1,
  VisualTechniqueEventV1,
} from "../../../api/chapterVisualPlan";
import type {
  MessageKey,
  MessageTranslator,
} from "../../../i18n";

export type VisualPlanTimedEventV1 =
  | VisualEntityEventV1
  | VisualCameraEventV1
  | VisualSoundEventV1
  | VisualTechniqueEventV1;

export interface VisualShotRef {
  unitIndex: number;
  shotIndex: number;
  unit: VisualNarrativeUnitV1;
  shot: VisualShotPlanV1;
}

export interface VisualPlanAutoSaveState {
  hasDraft: boolean;
  planDirty: boolean;
  modeDirty: boolean;
}

export interface VisualPlanIssueText {
  key: MessageKey;
  params?: Record<string, VisualPlanIssueParam>;
}

export type VisualPlanIssueParam = string | number | VisualPlanIssueText;

export interface VisualPlanIssue {
  key: MessageKey;
  params: Record<string, VisualPlanIssueParam>;
}

const issueLabel = (
  key: MessageKey,
  params?: Record<string, VisualPlanIssueParam>,
): VisualPlanIssueParam => ({ key, params });

const translateVisualPlanIssueParam = (
  value: VisualPlanIssueParam,
  t: MessageTranslator,
): string | number => typeof value === "object"
  ? t(
      value.key,
      Object.fromEntries(
        Object.entries(value.params ?? {}).map(([name, nestedValue]) => [
          name,
          translateVisualPlanIssueParam(nestedValue, t),
        ]),
      ),
    )
  : value;

export const translateVisualPlanIssue = (
  issue: VisualPlanIssue,
  t: MessageTranslator,
): string => {
  const params = Object.fromEntries(
    Object.entries(issue.params).map(([name, value]) => [
      name,
      translateVisualPlanIssueParam(value, t),
    ]),
  );
  return t(issue.key, params);
};

export const shouldAutoSaveVisualPlanDraft = ({
  hasDraft,
  planDirty,
  modeDirty,
}: VisualPlanAutoSaveState) => planDirty || (hasDraft && modeDirty);

export const clonePlan = (plan: ChapterVisualPlanV1): ChapterVisualPlanV1 =>
  structuredClone(plan);

export const visualShots = (plan: ChapterVisualPlanV1): VisualShotRef[] =>
  plan.narrative_units.flatMap((unit, unitIndex) =>
    unit.shots.map((shot, shotIndex) => ({ unitIndex, shotIndex, unit, shot })),
  );

export const visualShotAt = (
  plan: ChapterVisualPlanV1,
  atMs: number,
  shots: readonly VisualShotRef[] = visualShots(plan),
): VisualShotRef | null =>
  shots.find(({ shot }) =>
    atMs >= shot.start_ms &&
    (atMs < shot.end_ms || shot.end_ms === plan.frame.duration_ms),
  ) ?? shots.at(-1) ?? null;

const shiftShot = (shot: VisualShotPlanV1, delta: number) => {
  shot.start_ms += delta;
  shot.end_ms += delta;
  for (const keyframe of shot.keyframes) keyframe.at_ms += delta;
  for (const event of [
    ...shot.entity_events,
    ...shot.camera_events,
    ...shot.sound_events,
    ...shot.technique_events,
  ]) {
    event.start_ms += delta;
    event.end_ms += delta;
  }
};

const userAdditionIdsForShot = (shot: VisualShotPlanV1) => new Set([
  ...shot.user_addition_ids,
  ...shot.entity_events.flatMap((event) => event.user_addition_ids),
  ...shot.camera_events.flatMap((event) => event.user_addition_ids),
  ...shot.sound_events.flatMap((event) => event.user_addition_ids),
  ...shot.technique_events.flatMap((event) => event.user_addition_ids),
]);

const userAdditionIdsOutsideShot = (
  plan: ChapterVisualPlanV1,
  shotId: string,
) => {
  const ids = new Set<string>();
  const add = (values: readonly string[]) => values.forEach((value) => ids.add(value));
  for (const entity of plan.entities) add(entity.user_addition_ids);
  for (const unit of plan.narrative_units) {
    add(unit.user_addition_ids);
    for (const shot of unit.shots) {
      if (shot.id !== shotId) add([...userAdditionIdsForShot(shot)]);
    }
  }
  return ids;
};

const scaleTime = (
  value: number,
  start: number,
  oldEnd: number,
  newEnd: number,
) => {
  if (value <= start) return start;
  if (value >= oldEnd) return newEnd;
  return Math.round(start + ((value - start) * (newEnd - start)) / (oldEnd - start));
};

export const resizeVisualShot = (
  source: ChapterVisualPlanV1,
  shotId: string,
  durationMs: number,
): ChapterVisualPlanV1 => {
  const plan = clonePlan(source);
  const shots = visualShots(plan);
  const ref = shots.find(({ shot }) => shot.id === shotId);
  if (!ref) return plan;
  if (!Number.isFinite(durationMs)) return plan;
  const nextDuration = Math.max(250, Math.round(durationMs));
  const oldEnd = ref.shot.end_ms;
  const newEnd = ref.shot.start_ms + nextDuration;
  const delta = newEnd - oldEnd;
  if (delta === 0) return plan;

  for (const keyframe of ref.shot.keyframes) {
    keyframe.at_ms = scaleTime(keyframe.at_ms, ref.shot.start_ms, oldEnd, newEnd);
  }
  for (const event of [
    ...ref.shot.entity_events,
    ...ref.shot.camera_events,
    ...ref.shot.sound_events,
    ...ref.shot.technique_events,
  ]) {
    event.start_ms = Math.min(
      newEnd - 1,
      scaleTime(event.start_ms, ref.shot.start_ms, oldEnd, newEnd),
    );
    event.end_ms = Math.max(
      event.start_ms + 1,
      Math.min(newEnd, scaleTime(event.end_ms, ref.shot.start_ms, oldEnd, newEnd)),
    );
  }
  ref.shot.end_ms = newEnd;

  const unit = plan.narrative_units[ref.unitIndex];
  for (const shot of unit.shots.slice(ref.shotIndex + 1)) shiftShot(shot, delta);
  unit.end_ms += delta;
  for (const laterUnit of plan.narrative_units.slice(ref.unitIndex + 1)) {
    laterUnit.start_ms += delta;
    laterUnit.end_ms += delta;
    for (const shot of laterUnit.shots) shiftShot(shot, delta);
  }
  plan.frame.duration_ms += delta;

  const shotsById = new Map<string, VisualShotPlanV1>();
  for (const { shot } of shots) {
    if (!shotsById.has(shot.id)) shotsById.set(shot.id, shot);
  }
  for (const transition of plan.transitions) {
    if (transition.at_ms > ref.shot.start_ms && transition.at_ms <= oldEnd) {
      transition.at_ms = scaleTime(
        transition.at_ms,
        ref.shot.start_ms,
        oldEnd,
        newEnd,
      );
    } else if (transition.at_ms > oldEnd) {
      transition.at_ms += delta;
    }
    const target = shotsById.get(transition.to_shot_id);
    if (transition.kind === "continuous") transition.duration_ms = 0;
    else if (target) transition.duration_ms = Math.min(
      transition.duration_ms,
      target.end_ms - target.start_ms,
    );
  }
  for (const addition of plan.user_additions) {
    if (addition.introduced_at_ms == null) continue;
    if (addition.introduced_at_ms > ref.shot.start_ms && addition.introduced_at_ms <= oldEnd) {
      addition.introduced_at_ms = scaleTime(
        addition.introduced_at_ms,
        ref.shot.start_ms,
        oldEnd,
        newEnd,
      );
    } else if (addition.introduced_at_ms > oldEnd) {
      addition.introduced_at_ms += delta;
    }
  }
  return plan;
};

export const moveVisualShot = (
  source: ChapterVisualPlanV1,
  shotId: string,
  direction: -1 | 1,
): ChapterVisualPlanV1 => {
  const plan = clonePlan(source);
  const ref = visualShots(plan).find(({ shot }) => shot.id === shotId);
  if (!ref) return plan;
  const originalStart = ref.shot.start_ms;
  const originalEnd = ref.shot.end_ms;
  const movedAdditionIds = userAdditionIdsForShot(ref.shot);
  const sharedAdditionIds = userAdditionIdsOutsideShot(plan, shotId);
  const originalShots = visualShots(plan);
  const boundaryByFromShot = new Map<string, Map<string, number>>();
  for (let index = 0; index + 1 < originalShots.length; index += 1) {
    const fromShotId = originalShots[index].shot.id;
    const toShotId = originalShots[index + 1].shot.id;
    let boundaries = boundaryByFromShot.get(fromShotId);
    if (!boundaries) {
      boundaries = new Map();
      boundaryByFromShot.set(fromShotId, boundaries);
    }
    if (!boundaries.has(toShotId)) boundaries.set(toShotId, index);
  }
  const transitionBoundaries = new Map<string, number>();
  for (const transition of plan.transitions) {
    transitionBoundaries.set(
      transition.id,
      boundaryByFromShot.get(transition.from_shot_id)?.get(transition.to_shot_id) ?? -1,
    );
  }
  const unit = plan.narrative_units[ref.unitIndex];
  const target = ref.shotIndex + direction;
  if (target < 0 || target >= unit.shots.length) return plan;
  [unit.shots[ref.shotIndex], unit.shots[target]] = [
    unit.shots[target],
    unit.shots[ref.shotIndex],
  ];
  let cursor = unit.start_ms;
  for (const shot of unit.shots) {
    const duration = shot.end_ms - shot.start_ms;
    shiftShot(shot, cursor - shot.start_ms);
    cursor += duration;
  }
  const movedShot = unit.shots.find((shot) => shot.id === shotId);
  if (movedShot) {
    const delta = movedShot.start_ms - originalStart;
    const additionById = new Map<string, (typeof plan.user_additions)[number]>();
    for (const addition of plan.user_additions) {
      if (!additionById.has(addition.id)) additionById.set(addition.id, addition);
    }
    for (const additionId of movedAdditionIds) {
      if (sharedAdditionIds.has(additionId)) continue;
      const addition = additionById.get(additionId);
      if (
        !addition ||
        addition.introduced_at_ms == null ||
        addition.introduced_at_ms < originalStart ||
        addition.introduced_at_ms >= originalEnd
      ) continue;
      addition.introduced_at_ms = Math.max(
        0,
        Math.min(plan.frame.duration_ms - 1, addition.introduced_at_ms + delta),
      );
    }
  }
  const reorderedShots = visualShots(plan);
  for (const transition of plan.transitions) {
    const boundary = transitionBoundaries.get(transition.id) ?? -1;
    const from = reorderedShots[boundary]?.shot;
    const to = reorderedShots[boundary + 1]?.shot;
    if (!from || !to) continue;
    transition.from_shot_id = from.id;
    transition.to_shot_id = to.id;
    transition.at_ms = from.end_ms;
    if (transition.kind === "continuous") transition.duration_ms = 0;
    else transition.duration_ms = Math.min(
      transition.duration_ms,
      to.end_ms - to.start_ms,
    );
  }
  return plan;
};

const findVisualPlanTimedEvent = (
  plan: ChapterVisualPlanV1,
  shotId: string,
  eventId: string,
): { shot: VisualShotPlanV1; event: VisualPlanTimedEventV1 } | null => {
  const shot = visualShots(plan).find(({ shot: item }) => item.id === shotId)?.shot;
  if (!shot) return null;
  const event = [
    ...shot.entity_events,
    ...shot.camera_events,
    ...shot.sound_events,
    ...shot.technique_events,
  ].find((item) => item.id === eventId);
  return event ? { shot, event } : null;
};

const referencedUserAdditionIds = (plan: ChapterVisualPlanV1) => {
  const ids = new Set<string>();
  const add = (values: readonly string[]) => values.forEach((value) => ids.add(value));
  for (const entity of plan.entities) add(entity.user_addition_ids);
  for (const unit of plan.narrative_units) {
    add(unit.user_addition_ids);
    for (const shot of unit.shots) {
      add(shot.user_addition_ids);
      for (const event of [
        ...shot.entity_events,
        ...shot.camera_events,
        ...shot.sound_events,
        ...shot.technique_events,
      ]) add(event.user_addition_ids);
    }
  }
  return ids;
};

export const removeVisualPlanTimedEvent = (
  plan: ChapterVisualPlanV1,
  shotId: string,
  eventId: string,
): boolean => {
  const shot = visualShots(plan).find(({ shot: item }) => item.id === shotId)?.shot;
  if (!shot) return false;
  const removeFrom = <T extends { id: string; user_addition_ids: string[] }>(events: T[]) => {
    const index = events.findIndex((event) => event.id === eventId);
    if (index < 0) return null;
    return events.splice(index, 1)[0]?.user_addition_ids ?? [];
  };
  const additionIds = removeFrom(shot.entity_events) ??
    removeFrom(shot.camera_events) ??
    removeFrom(shot.sound_events) ??
    removeFrom(shot.technique_events);
  if (additionIds == null) return false;

  const referenced = referencedUserAdditionIds(plan);
  const removedCandidates = new Set(additionIds);
  plan.user_additions = plan.user_additions.filter(
    (addition) => !removedCandidates.has(addition.id) || referenced.has(addition.id),
  );
  return true;
};

export const updateVisualPlanTimedEventRange = (
  plan: ChapterVisualPlanV1,
  shotId: string,
  eventId: string,
  boundary: "start" | "end",
  atMs: number,
): boolean => {
  if (!Number.isFinite(atMs)) return false;
  const match = findVisualPlanTimedEvent(plan, shotId, eventId);
  if (!match) return false;
  const rounded = Math.round(atMs);
  const next = boundary === "start"
    ? Math.max(match.shot.start_ms, Math.min(match.event.end_ms - 1, rounded))
    : Math.max(match.event.start_ms + 1, Math.min(match.shot.end_ms, rounded));
  const field = boundary === "start" ? "start_ms" : "end_ms";
  if (match.event[field] === next) return false;
  match.event[field] = next;
  return true;
};

/** Detach a source span from every adaptation-scoped reference. */
export const detachVisualPlanSourceSpanReferences = (
  plan: ChapterVisualPlanV1,
  spanId: string,
) => {
  const removeFromEvents = <T extends {
    source_span_ids: string[];
    user_addition_ids: string[];
  }>(events: T[]) => events
    .map((event) => ({
      ...event,
      source_span_ids: event.source_span_ids.filter((id) => id !== spanId),
    }))
    .filter((event) => event.source_span_ids.length > 0 || event.user_addition_ids.length > 0);

  for (const entity of plan.entities) {
    entity.source_span_ids = entity.source_span_ids.filter((id) => id !== spanId);
  }
  for (const unit of plan.narrative_units) {
    unit.source_span_ids = unit.source_span_ids.filter((id) => id !== spanId);
    for (const shot of unit.shots) {
      shot.source_span_ids = shot.source_span_ids.filter((id) => id !== spanId);
      shot.entity_events = removeFromEvents(shot.entity_events);
      shot.camera_events = removeFromEvents(shot.camera_events);
      shot.sound_events = removeFromEvents(shot.sound_events);
      shot.technique_events = removeFromEvents(shot.technique_events);
    }
  }
};

export const collectVisualPlanIssues = (
  plan: ChapterVisualPlanV1,
  availableSourceSpanIds?: readonly string[],
): VisualPlanIssue[] => {
  const issues: VisualPlanIssue[] = [];
  const push = (
    key: MessageKey,
    params: Record<string, VisualPlanIssueParam> = {},
  ) => issues.push({ key, params });
  const labels = {
    entity: issueLabel("visualPlan.validation.labels.entity"),
    userAddition: issueLabel("visualPlan.validation.labels.userAddition"),
    narrativeUnit: issueLabel("visualPlan.validation.labels.narrativeUnit"),
    shot: issueLabel("visualPlan.validation.labels.shot"),
    keyframe: issueLabel("visualPlan.validation.labels.keyframe"),
    relation: issueLabel("visualPlan.validation.labels.relation"),
    timedEvent: issueLabel("visualPlan.validation.labels.timedEvent"),
    transition: issueLabel("visualPlan.validation.labels.transition"),
  };
  const entityLabel = (id: string): VisualPlanIssueParam => issueLabel(
    "visualPlan.validation.labels.entityWithId",
    { id },
  );
  const unitLabel = (name: string): VisualPlanIssueParam => issueLabel(
    "visualPlan.validation.labels.unitWithName",
    { name },
  );
  const shotLabel = (name: string): VisualPlanIssueParam => issueLabel(
    "visualPlan.validation.labels.shotWithName",
    { name },
  );
  const keyframeLabel = (name: string): VisualPlanIssueParam => issueLabel(
    "visualPlan.validation.labels.keyframeWithName",
    { name },
  );
  const entityIds = new Set<string>();
  const additionIds = new Set<string>();
  const unitIds = new Set<string>();
  const shotIds = new Set<string>();
  const keyframeIds = new Set<string>();
  const eventIds = new Set<string>();
  const transitionIds = new Set<string>();
  const addId = (
    value: string,
    label: VisualPlanIssueParam,
    namespace: Set<string>,
  ) => {
    if (!value.trim()) push("visualPlan.validation.missingId", { label });
    else if (namespace.has(value)) push("visualPlan.validation.duplicateId", { label, value });
    else namespace.add(value);
  };
  const requireId = (
    value: string,
    namespace: Set<string>,
    label: VisualPlanIssueParam,
  ) => {
    if (!value.trim()) push("visualPlan.validation.missingId", { label });
    else if (!namespace.has(value)) push("visualPlan.validation.missingReferenceId", { label, value });
  };
  const requireIds = (
    values: readonly string[],
    namespace: Set<string>,
    label: VisualPlanIssueParam,
  ) => {
    const seen = new Set<string>();
    for (const value of values) {
      if (!value.trim()) push("visualPlan.validation.emptyReferenceId", { label });
      else if (seen.has(value)) push("visualPlan.validation.duplicateReference", { label, value });
      else {
        seen.add(value);
        requireId(value, namespace, label);
      }
    }
  };
  const requireProvenance = (
    sourceSpanIds: readonly string[],
    userAdditionIds: readonly string[],
    sourceNamespace: Set<string>,
    additionNamespace: Set<string>,
    label: VisualPlanIssueParam,
    allowedSourceSpanIds?: ReadonlySet<string>,
  ) => {
    requireIds(sourceSpanIds, sourceNamespace, issueLabel("visualPlan.validation.labels.sourceProvenance", { label }));
    requireIds(userAdditionIds, additionNamespace, issueLabel("visualPlan.validation.labels.userAdditionProvenance", { label }));
    if (allowedSourceSpanIds && sourceSpanIds.some((spanId) => !allowedSourceSpanIds.has(spanId))) {
      push("visualPlan.validation.sourceOutsideScope", { label });
    }
    if (sourceSpanIds.length === 0 && userAdditionIds.length === 0) {
      push("visualPlan.validation.provenanceRequired", { label });
    }
  };
  const timeLabel = (title: string, fallback: string) => title.trim() || fallback;
  const requireText = (value: unknown, label: VisualPlanIssueParam) => {
    if (typeof value !== "string" || !value.trim()) push("visualPlan.validation.textRequired", { label });
  };
  const requireFiniteInteger = (value: unknown, label: VisualPlanIssueParam) => {
    if (!Number.isInteger(value) || !Number.isFinite(value)) push("visualPlan.validation.finiteIntegerRequired", { label });
  };

  if (plan.schema_version !== "chapter_visual_plan_v1") push("visualPlan.validation.unsupportedSchema");
  for (const [field, value] of Object.entries(plan.directing_statement)) {
    requireText(value, issueLabel("visualPlan.validation.labels.directingIntentField", { field }));
  }
  requireFiniteInteger(plan.frame.width, issueLabel("visualPlan.validation.labels.frameWidth"));
  requireFiniteInteger(plan.frame.height, issueLabel("visualPlan.validation.labels.frameHeight"));
  requireFiniteInteger(plan.frame.duration_ms, issueLabel("visualPlan.validation.labels.frameDuration"));
  if (plan.frame.width <= 0 || plan.frame.height <= 0) push("visualPlan.validation.frameDimensionsPositive");
  if (plan.frame.duration_ms <= 0) push("visualPlan.validation.durationPositive");
  for (const entity of plan.entities) addId(entity.id, labels.entity, entityIds);
  for (const addition of plan.user_additions) {
    addId(addition.id, labels.userAddition, additionIds);
    requireText(addition.description, issueLabel("visualPlan.validation.labels.userAdditionDescription", { id: addition.id }));
    requireText(addition.rationale, issueLabel("visualPlan.validation.labels.userAdditionRationale", { id: addition.id }));
    if (addition.introduced_at_ms != null && addition.introduced_at_ms >= plan.frame.duration_ms) {
      push("visualPlan.validation.additionBeforeEnd", { id: addition.id });
    }
    if (addition.introduced_at_ms != null) {
      requireFiniteInteger(addition.introduced_at_ms, issueLabel("visualPlan.validation.labels.userAdditionAppearance", { id: addition.id }));
    }
  }

  const dispositionBySpan = new Map(
    plan.source_dispositions.map((item) => [item.source_span_id, item]),
  );
  const adaptedSpanIds = new Set(
    plan.source_dispositions
      .filter((item) => item.disposition === "adapt")
      .map((item) => item.source_span_id),
  );
  for (const entity of plan.entities) {
    requireText(entity.label, issueLabel("visualPlan.validation.labels.entityName", { id: entity.id }));
    requireText(entity.description, issueLabel("visualPlan.validation.labels.entityDescription", { id: entity.id }));
    requireProvenance(
      entity.source_span_ids,
      entity.user_addition_ids,
      adaptedSpanIds,
      additionIds,
      entityLabel(entity.id),
    );
  }
  if (availableSourceSpanIds) {
    const available = new Set(availableSourceSpanIds);
    for (const spanId of available) {
      if (!dispositionBySpan.has(spanId)) push("visualPlan.validation.sourceSpanUnprocessed", { id: spanId });
    }
    for (const spanId of dispositionBySpan.keys()) {
      if (!available.has(spanId)) push("visualPlan.validation.dispositionSourceMissing", { id: spanId });
    }
  }

  let chapterCursor = 0;
  const sequenceIndices = new Set<number>();
  const timelinePositions = new Set<number>();
  const intercutGroups = new Map<string, typeof plan.narrative_units>();
  for (const unit of plan.narrative_units) {
    addId(unit.id, labels.narrativeUnit, unitIds);
    const unitName = timeLabel(unit.title, unit.id);
    const currentUnitLabel = unitLabel(unitName);
    requireText(unit.title, issueLabel("visualPlan.validation.labels.unitTitle", { id: unit.id }));
    requireText(unit.narrative_purpose, issueLabel("visualPlan.validation.labels.unitNarrativePurpose", { name: unitName }));
    requireFiniteInteger(unit.sequence_index, issueLabel("visualPlan.validation.labels.unitSequence", { name: unitName }));
    requireFiniteInteger(unit.timeline_position, issueLabel("visualPlan.validation.labels.unitTimelinePosition", { name: unitName }));
    if (sequenceIndices.has(unit.sequence_index)) push("visualPlan.validation.duplicateSequence", { value: unit.sequence_index });
    if (timelinePositions.has(unit.timeline_position)) push("visualPlan.validation.duplicateTimelinePosition", { value: unit.timeline_position });
    sequenceIndices.add(unit.sequence_index);
    timelinePositions.add(unit.timeline_position);
    if (unit.start_ms !== chapterCursor) push("visualPlan.validation.rangeGapOverlap", { label: currentUnitLabel });
    if (unit.end_ms <= unit.start_ms || unit.end_ms > plan.frame.duration_ms) push("visualPlan.validation.rangeInvalid", { label: currentUnitLabel });
    chapterCursor = unit.end_ms;
    const requiresIntercutGroup = unit.editing_mode === "intercut" || unit.editing_mode === "parallel_action";
    if (requiresIntercutGroup && !unit.intercut_group_id?.trim()) {
      push("visualPlan.validation.intercutGroupRequired", { label: currentUnitLabel });
    } else if (!requiresIntercutGroup && unit.intercut_group_id != null) {
      push("visualPlan.validation.intercutGroupNotAllowed", { label: currentUnitLabel });
    } else if (unit.intercut_group_id?.trim()) {
      const group = intercutGroups.get(unit.intercut_group_id) ?? [];
      group.push(unit);
      intercutGroups.set(unit.intercut_group_id, group);
    }
    requireProvenance(
      unit.source_span_ids,
      unit.user_addition_ids,
      adaptedSpanIds,
      additionIds,
      currentUnitLabel,
    );
    const unitSourceSpanIds = new Set(unit.source_span_ids);
    requireIds(unit.participant_entity_ids, entityIds, issueLabel("visualPlan.validation.labels.unitParticipants", { name: unitName }));
    if (unit.focal_entity_id) {
      requireId(unit.focal_entity_id, entityIds, issueLabel("visualPlan.validation.labels.unitFocalEntity", { name: unitName }));
      if (!unit.participant_entity_ids.includes(unit.focal_entity_id)) {
        push("visualPlan.validation.focalEntityNotParticipant", { label: currentUnitLabel });
      }
    }

    let shotCursor = unit.start_ms;
    for (const shot of unit.shots) {
      addId(shot.id, labels.shot, shotIds);
      const shotName = timeLabel(shot.title, shot.id);
      const currentShotLabel = shotLabel(shotName);
      requireText(shot.title, issueLabel("visualPlan.validation.labels.shotTitle", { id: shot.id }));
      requireText(shot.narrative_purpose, issueLabel("visualPlan.validation.labels.shotNarrativePurpose", { name: shotName }));
      if (shot.start_ms !== shotCursor) push("visualPlan.validation.rangeGapOverlap", { label: currentShotLabel });
      if (shot.end_ms <= shot.start_ms || shot.end_ms > plan.frame.duration_ms) push("visualPlan.validation.rangeInvalid", { label: currentShotLabel });
      if (shot.keyframes.length < 2) push("visualPlan.validation.keyframesRequired", { label: currentShotLabel });
      if (shot.keyframes[0]?.at_ms !== shot.start_ms) push("visualPlan.validation.firstKeyframeMissing", { label: currentShotLabel });
      if (shot.keyframes.at(-1)?.at_ms !== shot.end_ms) push("visualPlan.validation.lastKeyframeMissing", { label: currentShotLabel });
      requireProvenance(
        shot.source_span_ids,
        shot.user_addition_ids,
        adaptedSpanIds,
        additionIds,
        currentShotLabel,
        unitSourceSpanIds,
      );
      const shotSourceSpanIds = new Set(shot.source_span_ids);

      let keyframeCursor = shot.start_ms;
      for (const keyframe of shot.keyframes) {
        addId(keyframe.id, labels.keyframe, keyframeIds);
        const currentKeyframeLabel = keyframeLabel(keyframe.label || keyframe.id);
        requireText(keyframe.label, issueLabel("visualPlan.validation.labels.keyframeDescription", { id: keyframe.id }));
        if (keyframe.at_ms < shot.start_ms || keyframe.at_ms > shot.end_ms || keyframe.at_ms < keyframeCursor) {
          push("visualPlan.validation.keyframeTimeInvalid", { label: currentShotLabel });
        }
        keyframeCursor = keyframe.at_ms;
        if (
          !Number.isFinite(keyframe.camera.center_x) ||
          !Number.isFinite(keyframe.camera.center_y) ||
          !Number.isFinite(keyframe.camera.rotation_degrees) ||
          !Number.isFinite(keyframe.camera.zoom) ||
          keyframe.camera.zoom <= 0
        ) {
          push("visualPlan.validation.cameraParamsInvalid", { label: currentKeyframeLabel });
        }
        if (keyframe.camera.center_x < 0 || keyframe.camera.center_x > 1 ||
          keyframe.camera.center_y < 0 || keyframe.camera.center_y > 1) {
          push("visualPlan.validation.cameraCenterOutOfBounds", { label: currentKeyframeLabel });
        }
        const stateEntityIds = new Set<string>();
        for (const state of keyframe.entity_states) {
          requireId(state.entity_id, entityIds, issueLabel("visualPlan.validation.labels.keyframeEntityState", { name: keyframe.label }));
          if (stateEntityIds.has(state.entity_id)) push("visualPlan.validation.duplicateEntityState", { label: currentKeyframeLabel, entityId: state.entity_id });
          stateEntityIds.add(state.entity_id);
          const { x, y, width, height } = state.bounds;
          if (
            ![x, y, width, height, state.rotation_degrees, state.opacity].every(Number.isFinite) ||
            x < 0 || y < 0 || width <= 0 || height <= 0 ||
            x + width > 1 || y + height > 1 || state.opacity < 0 || state.opacity > 1
          ) {
            push("visualPlan.validation.entityBoundsOutOfBounds", { label: currentKeyframeLabel, entityId: state.entity_id });
          }
          if (state.gaze_target_entity_id) requireId(state.gaze_target_entity_id, entityIds, issueLabel("visualPlan.validation.labels.gazeTarget", { id: state.entity_id }));
        }
        if (keyframe.camera.focus_entity_id) {
          requireId(keyframe.camera.focus_entity_id, entityIds, issueLabel("visualPlan.validation.labels.keyframeFocusEntity", { name: keyframe.label }));
          if (!stateEntityIds.has(keyframe.camera.focus_entity_id)) {
            push("visualPlan.validation.focusEntityStateMissing", { label: currentKeyframeLabel });
          }
        }
        for (const relation of keyframe.relations) {
          addId(relation.id, labels.relation, eventIds);
          requireId(relation.source_entity_id, entityIds, issueLabel("visualPlan.validation.labels.relationSourceEntity"));
          requireId(relation.target_entity_id, entityIds, issueLabel("visualPlan.validation.labels.relationTargetEntity"));
          if (relation.source_entity_id === relation.target_entity_id) {
            push("visualPlan.validation.selfRelation", { label: currentKeyframeLabel });
          }
          if (!stateEntityIds.has(relation.source_entity_id)) {
            push("visualPlan.validation.relationSourceStateMissing", { label: currentKeyframeLabel });
          }
          if (!stateEntityIds.has(relation.target_entity_id)) {
            push("visualPlan.validation.relationTargetStateMissing", { label: currentKeyframeLabel });
          }
          if (relation.kind === "custom" && !relation.label?.trim()) {
            push("visualPlan.validation.customRelationLabelRequired", { label: currentKeyframeLabel });
          }
        }
      }
      for (const event of [
        ...shot.entity_events,
        ...shot.camera_events,
        ...shot.sound_events,
        ...shot.technique_events,
      ]) {
        addId(event.id, labels.timedEvent, eventIds);
        requireText(
          "content" in event ? event.content : event.description,
          issueLabel("visualPlan.validation.labels.timedEventDescription", { shot: shotName }),
        );
        if (event.start_ms < shot.start_ms || event.start_ms >= event.end_ms || event.end_ms > shot.end_ms) {
          push("visualPlan.validation.eventOutOfShot", { label: currentShotLabel });
        }
        requireProvenance(
          event.source_span_ids,
          event.user_addition_ids,
          adaptedSpanIds,
          additionIds,
          issueLabel("visualPlan.validation.labels.timedEventInShot", { shot: shotName }),
          shotSourceSpanIds,
        );
        if ("entity_id" in event) requireId(event.entity_id, entityIds, issueLabel("visualPlan.validation.labels.eventEntity", { shot: shotName }));
        if ("target_entity_id" in event && event.target_entity_id) requireId(event.target_entity_id, entityIds, issueLabel("visualPlan.validation.labels.eventTarget", { shot: shotName }));
        if ("speaker_entity_id" in event && event.speaker_entity_id) requireId(event.speaker_entity_id, entityIds, issueLabel("visualPlan.validation.labels.eventSpeaker", { shot: shotName }));
      }
      shotCursor = shot.end_ms;
    }
    if (shotCursor !== unit.end_ms) push("visualPlan.validation.unitShotsIncomplete", { label: currentUnitLabel });
  }
  if (chapterCursor !== plan.frame.duration_ms) push("visualPlan.validation.timelineCoverageIncomplete");
  for (let position = 1; position <= plan.narrative_units.length; position += 1) {
    if (!sequenceIndices.has(position)) push("visualPlan.validation.sequenceNotContinuous");
    if (!timelinePositions.has(position)) push("visualPlan.validation.timelinePositionNotContinuous");
  }
  for (const [groupId, units] of intercutGroups) {
    if (units.length < 2) push("visualPlan.validation.intercutGroupTooSmall", { groupId });
    else if (units.some((unit) => unit.editing_mode !== units[0].editing_mode)) {
      push("visualPlan.validation.intercutGroupModesMismatch", { groupId });
    }
  }

  const shots = visualShots(plan);
  const shotPositions = new Map(shots.map((item, index) => [item.shot.id, index]));
  for (const transition of plan.transitions) {
    addId(transition.id, labels.transition, transitionIds);
    const fromIndex = shotPositions.get(transition.from_shot_id);
    const toIndex = shotPositions.get(transition.to_shot_id);
    if (fromIndex == null || toIndex == null) {
      push("visualPlan.validation.transitionMissingShot", { id: transition.id });
      continue;
    }
    const from = shots[fromIndex].shot;
    const to = shots[toIndex].shot;
    if (toIndex !== fromIndex + 1) push("visualPlan.validation.transitionNonAdjacent", { id: transition.id });
    if (transition.at_ms !== from.end_ms || transition.at_ms !== to.start_ms) push("visualPlan.validation.transitionNotAtCut", { id: transition.id });
    if (transition.duration_ms > plan.frame.duration_ms - transition.at_ms || transition.duration_ms > to.end_ms - to.start_ms) {
      push("visualPlan.validation.transitionDurationOutOfRange", { id: transition.id });
    }
    if (transition.kind === "continuous" && transition.duration_ms !== 0) push("visualPlan.validation.continuousTransitionDuration", { id: transition.id });
  }

  const unitById = new Map<string, VisualNarrativeUnitV1>();
  const shotsByUnitId = new Map<string, Map<string, VisualShotPlanV1>>();
  for (const unit of plan.narrative_units) {
    if (unitById.has(unit.id)) continue;
    unitById.set(unit.id, unit);
    const shotById = new Map<string, VisualShotPlanV1>();
    for (const shot of unit.shots) {
      if (!shotById.has(shot.id)) shotById.set(shot.id, shot);
    }
    shotsByUnitId.set(unit.id, shotById);
  }
  const dispositionIds = new Set<string>();
  for (const item of plan.source_dispositions) {
    if (!item.source_span_id.trim()) push("visualPlan.validation.sourceDispositionIdMissing");
    if (dispositionIds.has(item.source_span_id)) push("visualPlan.validation.duplicateSourceDisposition", { id: item.source_span_id });
    dispositionIds.add(item.source_span_id);
    requireIds(item.shot_ids, shotIds, issueLabel("visualPlan.validation.labels.dispositionShots", { id: item.source_span_id }));
    if (item.disposition === "omit") {
      if (!item.reason?.trim()) push("visualPlan.validation.omittedReasonMissing", { id: item.source_span_id });
      if (item.narrative_unit_id || item.shot_ids.length > 0) push("visualPlan.validation.omittedBindingsPresent", { id: item.source_span_id });
      continue;
    }
    if (!item.narrative_unit_id) push("visualPlan.validation.adaptedUnitMissing", { id: item.source_span_id });
    if (item.shot_ids.length === 0) push("visualPlan.validation.adaptedShotMissing", { id: item.source_span_id });
    const unit = item.narrative_unit_id
      ? unitById.get(item.narrative_unit_id)
      : undefined;
    if (!unit) {
      push("visualPlan.validation.boundUnitMissing", { id: item.source_span_id });
      continue;
    }
    if (!unit.source_span_ids.includes(item.source_span_id)) push("visualPlan.validation.unitSourceMismatch", { id: item.source_span_id });
    for (const shotId of item.shot_ids) {
      const shot = shotsByUnitId.get(unit.id)?.get(shotId);
      if (!shot) push("visualPlan.validation.shotNotInUnit", { spanId: item.source_span_id, shotId });
      else if (!shot.source_span_ids.includes(item.source_span_id)) push("visualPlan.validation.shotSourceMismatch", { spanId: item.source_span_id, shotId: shot.id });
    }
  }
  for (const unit of plan.narrative_units) {
    for (const spanId of unit.source_span_ids) {
      const disposition = dispositionBySpan.get(spanId);
      if (!disposition || disposition.disposition !== "adapt" || disposition.narrative_unit_id !== unit.id) {
        push("visualPlan.validation.unitReferenceMismatch", { spanId });
      }
    }
    for (const shot of unit.shots) {
      for (const spanId of shot.source_span_ids) {
        const disposition = dispositionBySpan.get(spanId);
        if (!disposition || disposition.disposition !== "adapt" || !disposition.shot_ids.includes(shot.id)) {
          push("visualPlan.validation.shotReferenceMismatch", { spanId, shotId: shot.id });
        }
      }
    }
  }
  return issues;
};

export const formatPlanTime = (milliseconds: number) =>
  `${(milliseconds / 1000).toFixed(milliseconds % 1000 === 0 ? 0 : 1)}s`;
