import type {
  ChapterVisualPlanV2,
  VisualCameraEventV2,
  VisualEntityEventV2,
  VisualKeyframeV2,
  VisualSceneV2,
  VisualShotV2,
  VisualSoundEventV2,
  VisualTechniqueEventV2,
} from "../../../api/chapterVisualPlan";
import type {
  MessageKey,
  MessageTranslator,
} from "../../../i18n";

export type VisualPlanTimedEventV2 =
  | VisualEntityEventV2
  | VisualCameraEventV2
  | VisualSoundEventV2
  | VisualTechniqueEventV2;

export interface VisualShotRef {
  sceneIndex: number;
  shotIndex: number;
  scene: VisualSceneV2;
  shot: VisualShotV2;
  start_ms: number;
  end_ms: number;
}

interface VisualPlanIssueText {
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
): string => t(
  issue.key,
  Object.fromEntries(
    Object.entries(issue.params).map(([name, value]) => [
      name,
      translateVisualPlanIssueParam(value, t),
    ]),
  ),
);

export const clonePlan = (plan: ChapterVisualPlanV2): ChapterVisualPlanV2 =>
  structuredClone(plan);

export const visualShots = (plan: ChapterVisualPlanV2): VisualShotRef[] => {
  const result: VisualShotRef[] = [];
  let cursor = 0;
  plan.scenes.forEach((scene, sceneIndex) => {
    scene.shots.forEach((shot, shotIndex) => {
      result.push({
        sceneIndex,
        shotIndex,
        scene,
        shot,
        start_ms: cursor,
        end_ms: cursor + shot.duration_ms,
      });
      cursor += shot.duration_ms;
    });
  });
  return result;
};

export const planDurationMs = (plan: ChapterVisualPlanV2): number =>
  visualShots(plan).at(-1)?.end_ms ?? 0;

export const visualSceneStartMs = (
  plan: ChapterVisualPlanV2,
  sceneId: string,
): number | null => {
  let cursor = 0;
  for (const scene of plan.scenes) {
    if (scene.id === sceneId) return cursor;
    cursor += scene.shots.reduce((total, shot) => total + shot.duration_ms, 0);
  }
  return null;
};

export const visualShotAt = (
  plan: ChapterVisualPlanV2,
  atMs: number,
  shots: readonly VisualShotRef[] = visualShots(plan),
): VisualShotRef | null => {
  const target = Math.max(0, atMs);
  const duration = shots.at(-1)?.end_ms ?? 0;
  return shots.find(({ start_ms, end_ms }) =>
    target >= start_ms && (target < end_ms || end_ms === duration),
  ) ?? shots.at(-1) ?? null;
};

const allEvents = (shot: VisualShotV2): VisualPlanTimedEventV2[] => [
  ...shot.entity_events,
  ...shot.camera_events,
  ...shot.sound_events,
  ...shot.technique_events,
];

const scaleTime = (value: number, oldDuration: number, newDuration: number) => {
  if (oldDuration <= 0) return 0;
  return Math.round((value * newDuration) / oldDuration);
};

const normalizeKeyframeTimes = (
  keyframes: VisualKeyframeV2[],
  durationMs: number,
) => {
  const ordered = keyframes
    .map((keyframe, index) => ({ keyframe, index, atMs: keyframe.at_ms }))
    .sort((left, right) => left.atMs - right.atMs || left.index - right.index);
  let previous = -1;
  ordered.forEach(({ keyframe, atMs }, index) => {
    const minimum = previous + 1;
    const maximum = durationMs - (ordered.length - index - 1);
    keyframe.at_ms = minimum <= maximum
      ? Math.max(minimum, Math.min(maximum, atMs))
      : Math.max(previous, Math.min(durationMs, atMs));
    previous = keyframe.at_ms;
  });
  keyframes.sort((left, right) => left.at_ms - right.at_ms);
};

const normalizeTransition = (
  shot: VisualShotV2 | undefined,
  nextShot: VisualShotV2 | undefined,
) => {
  if (!shot?.transition_to_next) return;
  if (!nextShot) {
    shot.transition_to_next = null;
    return;
  }
  shot.transition_to_next.duration_ms = shot.transition_to_next.kind === "continuous"
    ? 0
    : Math.min(shot.transition_to_next.duration_ms, shot.duration_ms, nextShot.duration_ms);
};

export const resizeVisualShot = (
  source: ChapterVisualPlanV2,
  shotId: string,
  durationMs: number,
): ChapterVisualPlanV2 => {
  const plan = clonePlan(source);
  const shot = visualShots(plan).find(({ shot: item }) => item.id === shotId)?.shot;
  if (!shot || !Number.isFinite(durationMs)) return plan;

  const oldDuration = shot.duration_ms;
  const nextDuration = Math.max(250, Math.round(durationMs));
  if (oldDuration === nextDuration) return plan;
  shot.duration_ms = nextDuration;
  shot.keyframes.forEach((keyframe) => {
    keyframe.at_ms = scaleTime(keyframe.at_ms, oldDuration, nextDuration);
  });
  normalizeKeyframeTimes(shot.keyframes, nextDuration);
  for (const event of allEvents(shot)) {
    event.start_ms = Math.min(
      Math.max(0, nextDuration - 1),
      scaleTime(event.start_ms, oldDuration, nextDuration),
    );
    event.end_ms = Math.max(
      event.start_ms + 1,
      Math.min(nextDuration, scaleTime(event.end_ms, oldDuration, nextDuration)),
    );
  }

  const ref = visualShots(plan).find(({ shot: item }) => item.id === shotId);
  const scene = ref ? plan.scenes[ref.sceneIndex] : undefined;
  normalizeTransition(scene?.shots[ref ? ref.shotIndex - 1 : -1], shot);
  normalizeTransition(shot, scene?.shots[ref ? ref.shotIndex + 1 : -1]);
  return plan;
};

export const moveVisualShot = (
  source: ChapterVisualPlanV2,
  shotId: string,
  direction: -1 | 1,
): ChapterVisualPlanV2 => {
  const plan = clonePlan(source);
  const ref = visualShots(plan).find(({ shot }) => shot.id === shotId);
  if (!ref) return plan;
  const scene = plan.scenes[ref.sceneIndex];
  const targetIndex = ref.shotIndex + direction;
  if (!scene || targetIndex < 0 || targetIndex >= scene.shots.length) return plan;

  // 转场属于时间线边界，镜头换位时保留边界上的配置，而不是让最后一个镜头携带转场。
  const transitions = scene.shots.map((shot) => shot.transition_to_next);
  [scene.shots[ref.shotIndex], scene.shots[targetIndex]] = [
    scene.shots[targetIndex],
    scene.shots[ref.shotIndex],
  ];
  scene.shots.forEach((shot, index) => {
    shot.transition_to_next = index < scene.shots.length - 1
      ? transitions[index] ?? null
      : null;
    normalizeTransition(shot, scene.shots[index + 1]);
  });
  return plan;
};

export const moveVisualScene = (
  source: ChapterVisualPlanV2,
  sceneId: string,
  direction: -1 | 1,
): ChapterVisualPlanV2 => {
  const plan = clonePlan(source);
  const index = plan.scenes.findIndex((scene) => scene.id === sceneId);
  const targetIndex = index + direction;
  if (index < 0 || targetIndex < 0 || targetIndex >= plan.scenes.length) return plan;
  [plan.scenes[index], plan.scenes[targetIndex]] = [plan.scenes[targetIndex], plan.scenes[index]];
  return plan;
};

export const removeVisualScene = (
  source: ChapterVisualPlanV2,
  sceneId: string,
): ChapterVisualPlanV2 => {
  const plan = clonePlan(source);
  const index = plan.scenes.findIndex((scene) => scene.id === sceneId);
  if (index >= 0) plan.scenes.splice(index, 1);
  return plan;
};

export const removeVisualShot = (
  source: ChapterVisualPlanV2,
  shotId: string,
): ChapterVisualPlanV2 => {
  const plan = clonePlan(source);
  const ref = visualShots(plan).find(({ shot }) => shot.id === shotId);
  if (ref) {
    const scene = plan.scenes[ref.sceneIndex];
    scene?.shots.splice(ref.shotIndex, 1);
    normalizeTransition(scene?.shots[ref.shotIndex - 1], scene?.shots[ref.shotIndex]);
  }
  return plan;
};

export const removeVisualEntity = (
  source: ChapterVisualPlanV2,
  entityId: string,
): ChapterVisualPlanV2 => {
  const plan = clonePlan(source);
  if (!plan.entities.some((entity) => entity.id === entityId)) return plan;
  plan.entities = plan.entities.filter((entity) => entity.id !== entityId);
  plan.scenes.forEach((scene) => {
    scene.participant_entity_ids = scene.participant_entity_ids.filter((id) => id !== entityId);
    if (scene.focal_entity_id === entityId) scene.focal_entity_id = null;
    scene.shots.forEach((shot) => {
      shot.entity_events = shot.entity_events.filter((event) => event.entity_id !== entityId && event.target_entity_id !== entityId);
      shot.camera_events = shot.camera_events.filter((event) => event.target_entity_id !== entityId);
      shot.sound_events = shot.sound_events.filter((event) => event.speaker_entity_id !== entityId);
      shot.keyframes.forEach((keyframe) => {
        keyframe.entity_states = keyframe.entity_states.filter((state) => state.entity_id !== entityId);
        keyframe.relations = keyframe.relations.filter((relation) => relation.source_entity_id !== entityId && relation.target_entity_id !== entityId);
        if (keyframe.camera.focus_entity_id === entityId) keyframe.camera.focus_entity_id = null;
        keyframe.entity_states.forEach((state) => {
          if (state.gaze_target_entity_id === entityId) state.gaze_target_entity_id = null;
        });
      });
    });
  });
  return plan;
};

export const removeVisualKeyframe = (
  source: ChapterVisualPlanV2,
  shotId: string,
  keyframeId: string,
): ChapterVisualPlanV2 => {
  const plan = clonePlan(source);
  const shot = visualShots(plan).find(({ shot: item }) => item.id === shotId)?.shot;
  if (!shot || shot.keyframes.length <= 2) return plan;
  const index = shot.keyframes.findIndex((keyframe) => keyframe.id === keyframeId);
  if (index > 0 && index < shot.keyframes.length - 1) shot.keyframes.splice(index, 1);
  return plan;
};

export const isVisualKeyframeTimeAvailable = (
  shot: VisualShotV2,
  keyframeId: string | null,
  atMs: number,
): boolean => {
  if (!Number.isFinite(atMs)) return false;
  const rounded = Math.round(atMs);
  const currentIndex = shot.keyframes.findIndex((keyframe) => keyframe.id === keyframeId);
  if (currentIndex === 0 && rounded !== 0) return false;
  if (currentIndex === shot.keyframes.length - 1 && rounded !== shot.duration_ms) return false;
  return rounded >= 0
    && rounded <= shot.duration_ms
    && !shot.keyframes.some((keyframe) => keyframe.id !== keyframeId && keyframe.at_ms === rounded);
};

/** 返回最接近偏好时间的空闲整数毫秒，避免新增关键帧被当前播放头卡住。 */
export const availableVisualKeyframeTime = (
  shot: VisualShotV2,
  preferredAtMs: number,
): number | null => {
  if (!Number.isFinite(preferredAtMs) || shot.duration_ms < 0) return null;
  const preferred = Math.max(0, Math.min(shot.duration_ms, Math.round(preferredAtMs)));
  const occupied = [...new Set(
    shot.keyframes
      .map((keyframe) => keyframe.at_ms)
      .filter((atMs) => Number.isFinite(atMs) && atMs >= 0 && atMs <= shot.duration_ms),
  )].sort((left, right) => left - right);
  const ranges: Array<[number, number]> = [];
  let lower = 0;
  for (const atMs of occupied) {
    if (lower <= atMs - 1) ranges.push([lower, atMs - 1]);
    lower = atMs + 1;
  }
  if (lower <= shot.duration_ms) ranges.push([lower, shot.duration_ms]);
  if (ranges.length === 0) return null;
  return ranges.reduce((best, range) => {
    const candidate = Math.max(range[0], Math.min(range[1], preferred));
    return Math.abs(candidate - preferred) < Math.abs(best - preferred) ? candidate : best;
  }, Math.max(ranges[0][0], Math.min(ranges[0][1], preferred)));
};

export const removeVisualPlanTimedEvent = (
  plan: ChapterVisualPlanV2,
  shotId: string,
  eventId: string,
): boolean => {
  const shot = visualShots(plan).find(({ shot: item }) => item.id === shotId)?.shot;
  if (!shot) return false;
  for (const events of [
    shot.entity_events,
    shot.camera_events,
    shot.sound_events,
    shot.technique_events,
  ]) {
    const index = events.findIndex((event) => event.id === eventId);
    if (index < 0) continue;
    events.splice(index, 1);
    return true;
  }
  return false;
};

export const updateVisualPlanTimedEventRange = (
  plan: ChapterVisualPlanV2,
  shotId: string,
  eventId: string,
  boundary: "start" | "end",
  atMs: number,
): boolean => {
  if (!Number.isFinite(atMs)) return false;
  const shot = visualShots(plan).find(({ shot: item }) => item.id === shotId)?.shot;
  if (!shot) return false;
  const event = allEvents(shot).find((item) => item.id === eventId);
  if (!event) return false;
  const rounded = Math.round(atMs);
  const next = boundary === "start"
    ? Math.max(0, Math.min(event.end_ms - 1, rounded))
    : Math.max(event.start_ms + 1, Math.min(shot.duration_ms, rounded));
  const field = boundary === "start" ? "start_ms" : "end_ms";
  if (event[field] === next) return false;
  event[field] = next;
  return true;
};

const requireText = (
  value: unknown,
  label: VisualPlanIssueParam,
  push: (key: MessageKey, params?: Record<string, VisualPlanIssueParam>) => void,
) => {
  if (typeof value !== "string" || !value.trim()) push("visualPlan.validation.textRequired", { label });
};

const requireReference = (
  value: string,
  known: ReadonlySet<string>,
  label: VisualPlanIssueParam,
  push: (key: MessageKey, params?: Record<string, VisualPlanIssueParam>) => void,
) => {
  if (!value.trim()) push("visualPlan.validation.missingId", { label });
  else if (!known.has(value)) push("visualPlan.validation.missingReferenceId", { label, value });
};

const validateKeyframe = (
  keyframe: VisualKeyframeV2,
  shot: VisualShotV2,
  entityIds: ReadonlySet<string>,
  identityIds: Set<string>,
  push: (key: MessageKey, params?: Record<string, VisualPlanIssueParam>) => void,
) => {
  const label = issueLabel("visualPlan.validation.labels.keyframeWithName", { name: keyframe.label || keyframe.id });
  if (!keyframe.id.trim()) push("visualPlan.validation.missingId", { label: issueLabel("visualPlan.validation.labels.keyframe") });
  else if (identityIds.has(keyframe.id)) push("visualPlan.validation.duplicateId", { label: issueLabel("visualPlan.validation.labels.keyframe"), value: keyframe.id });
  else identityIds.add(keyframe.id);
  requireText(keyframe.label, issueLabel("visualPlan.validation.labels.keyframeDescription", { id: keyframe.id }), push);
  if (!Number.isFinite(keyframe.at_ms) || keyframe.at_ms < 0 || keyframe.at_ms > shot.duration_ms) push("visualPlan.validation.keyframeTimeInvalid", { label });
  const stateIds = new Set<string>();
  const visibleStateIds = new Set<string>();
  for (const state of keyframe.entity_states) {
    requireReference(state.entity_id, entityIds, issueLabel("visualPlan.validation.labels.keyframeEntityState", { name: keyframe.label }), push);
    if (stateIds.has(state.entity_id)) push("visualPlan.validation.duplicateEntityState", { label, entityId: state.entity_id });
    stateIds.add(state.entity_id);
    if (state.visible) visibleStateIds.add(state.entity_id);
    const { x, y, width, height } = state.bounds;
    if (![x, y, width, height, state.rotation_degrees, state.opacity].every(Number.isFinite) ||
      x < 0 || y < 0 || width <= 0 || height <= 0 || x + width > 1 || y + height > 1 ||
      state.opacity < 0 || state.opacity > 1) push("visualPlan.validation.entityBoundsOutOfBounds", { label, entityId: state.entity_id });
    if (state.gaze_target_entity_id != null) requireReference(state.gaze_target_entity_id, entityIds, issueLabel("visualPlan.validation.labels.gazeTarget", { id: state.entity_id }), push);
  }
  if (keyframe.camera.focus_entity_id != null) {
    requireReference(keyframe.camera.focus_entity_id, entityIds, issueLabel("visualPlan.validation.labels.keyframeFocusEntity", { name: keyframe.label }), push);
    if (!visibleStateIds.has(keyframe.camera.focus_entity_id)) push("visualPlan.validation.focusEntityStateMissing", { label });
  }
  if (!Number.isFinite(keyframe.camera.center_x) || !Number.isFinite(keyframe.camera.center_y) ||
    !Number.isFinite(keyframe.camera.rotation_degrees) || !Number.isFinite(keyframe.camera.zoom) ||
    keyframe.camera.zoom <= 0 || keyframe.camera.center_x < 0 || keyframe.camera.center_x > 1 ||
    keyframe.camera.center_y < 0 || keyframe.camera.center_y > 1) push("visualPlan.validation.cameraParamsInvalid", { label });
  for (const relation of keyframe.relations) {
    if (!relation.id.trim()) push("visualPlan.validation.missingId", { label: issueLabel("visualPlan.validation.labels.relation") });
    else if (identityIds.has(relation.id)) push("visualPlan.validation.duplicateId", { label: issueLabel("visualPlan.validation.labels.relation"), value: relation.id });
    else identityIds.add(relation.id);
    requireReference(relation.source_entity_id, entityIds, issueLabel("visualPlan.validation.labels.relationSourceEntity"), push);
    requireReference(relation.target_entity_id, entityIds, issueLabel("visualPlan.validation.labels.relationTargetEntity"), push);
    if (relation.source_entity_id === relation.target_entity_id) push("visualPlan.validation.selfRelation", { label });
    if (!visibleStateIds.has(relation.source_entity_id)) push("visualPlan.validation.relationSourceStateMissing", { label });
    if (!visibleStateIds.has(relation.target_entity_id)) push("visualPlan.validation.relationTargetStateMissing", { label });
    if (relation.kind === "custom" && !relation.label?.trim()) push("visualPlan.validation.customRelationLabelRequired", { label });
  }
};

const validateEvent = (
  event: VisualPlanTimedEventV2,
  shot: VisualShotV2,
  entityIds: ReadonlySet<string>,
  identityIds: Set<string>,
  push: (key: MessageKey, params?: Record<string, VisualPlanIssueParam>) => void,
) => {
  const shotName = shot.title || shot.id;
  const label = issueLabel("visualPlan.validation.labels.timedEventInShot", { shot: shotName });
  if (!event.id.trim()) push("visualPlan.validation.missingId", { label: issueLabel("visualPlan.validation.labels.timedEvent") });
  else if (identityIds.has(event.id)) push("visualPlan.validation.duplicateId", { label: issueLabel("visualPlan.validation.labels.timedEvent"), value: event.id });
  else identityIds.add(event.id);
  const text = "content" in event ? event.content : event.description;
  requireText(text, issueLabel("visualPlan.validation.labels.timedEventDescription", { shot: shotName }), push);
  if (!Number.isFinite(event.start_ms) || !Number.isFinite(event.end_ms) || event.start_ms < 0 || event.start_ms >= event.end_ms || event.end_ms > shot.duration_ms) push("visualPlan.validation.eventOutOfShot", { label });
  if ("entity_id" in event) requireReference(event.entity_id, entityIds, issueLabel("visualPlan.validation.labels.eventEntity", { shot: shotName }), push);
  if ("target_entity_id" in event && event.target_entity_id != null) requireReference(event.target_entity_id, entityIds, issueLabel("visualPlan.validation.labels.eventTarget", { shot: shotName }), push);
  if ("speaker_entity_id" in event && event.speaker_entity_id != null) requireReference(event.speaker_entity_id, entityIds, issueLabel("visualPlan.validation.labels.eventSpeaker", { shot: shotName }), push);
};

export const collectVisualPlanIssues = (
  plan: ChapterVisualPlanV2,
  strict = false,
): VisualPlanIssue[] => {
  const issues: VisualPlanIssue[] = [];
  const push = (key: MessageKey, params: Record<string, VisualPlanIssueParam> = {}) => issues.push({ key, params });
  if (plan.schema_version !== "chapter_visual_plan_v2") push("visualPlan.validation.unsupportedSchema");
  if (!Number.isInteger(plan.frame.width) || !Number.isInteger(plan.frame.height) || plan.frame.width <= 0 || plan.frame.height <= 0) push("visualPlan.validation.frameDimensionsPositive");
  if (plan.frame.background != null && !plan.frame.background.trim()) push("visualPlan.validation.backgroundNotEmpty");

  const entityIds = new Set<string>();
  const identityIds = new Set<string>();
  for (const entity of plan.entities) {
    if (!entity.id.trim()) push("visualPlan.validation.missingId", { label: issueLabel("visualPlan.validation.labels.entity") });
    else if (identityIds.has(entity.id)) push("visualPlan.validation.duplicateId", { label: issueLabel("visualPlan.validation.labels.entity"), value: entity.id });
    else {
      identityIds.add(entity.id);
      entityIds.add(entity.id);
    }
    requireText(entity.label, issueLabel("visualPlan.validation.labels.entityName", { id: entity.id }), push);
    requireText(entity.description, issueLabel("visualPlan.validation.labels.entityDescription", { id: entity.id }), push);
  }
  if (strict && plan.scenes.length === 0) push("visualPlan.validation.scenesRequired");
  const intercutGroups = new Map<string, { modes: VisualSceneV2["editing_mode"][] }>();
  plan.scenes.forEach((scene) => {
    const sceneLabel = issueLabel("visualPlan.validation.labels.sceneWithName", { name: scene.title || scene.id });
    if (!scene.id.trim()) push("visualPlan.validation.missingId", { label: issueLabel("visualPlan.validation.labels.scene") });
    else if (identityIds.has(scene.id)) push("visualPlan.validation.duplicateId", { label: issueLabel("visualPlan.validation.labels.scene"), value: scene.id });
    else identityIds.add(scene.id);
    requireText(scene.title, issueLabel("visualPlan.validation.labels.sceneTitle", { id: scene.id }), push);
    requireText(scene.narrative_purpose, issueLabel("visualPlan.validation.labels.sceneNarrativePurpose", { name: scene.title || scene.id }), push);
    const participantIds = new Set<string>();
    for (const entityId of scene.participant_entity_ids) {
      if (participantIds.has(entityId)) push("visualPlan.validation.duplicateId", { label: issueLabel("visualPlan.validation.labels.sceneParticipants", { name: scene.title || scene.id }), value: entityId });
      participantIds.add(entityId);
      requireReference(entityId, entityIds, issueLabel("visualPlan.validation.labels.sceneParticipants", { name: scene.title || scene.id }), push);
    }
    if (scene.focal_entity_id != null) {
      requireReference(scene.focal_entity_id, entityIds, issueLabel("visualPlan.validation.labels.sceneFocalEntity", { name: scene.title || scene.id }), push);
      if (!scene.participant_entity_ids.includes(scene.focal_entity_id)) push("visualPlan.validation.focalEntityNotParticipant", { label: sceneLabel });
    }
    if (scene.intercut_group_id != null) {
      if (!scene.intercut_group_id.trim()) push("visualPlan.validation.textRequired", { label: issueLabel("visualPlan.intercutGroup") });
      else if (scene.editing_mode !== "intercut" && scene.editing_mode !== "parallel_action") push("visualPlan.validation.intercutGroupNotAllowed", { label: issueLabel("visualPlan.intercutGroup") });
      else {
        const group = intercutGroups.get(scene.intercut_group_id) ?? { modes: [] };
        group.modes.push(scene.editing_mode);
        intercutGroups.set(scene.intercut_group_id, group);
      }
    } else if (scene.editing_mode === "intercut" || scene.editing_mode === "parallel_action") {
      push("visualPlan.validation.intercutGroupRequired", { label: issueLabel("visualPlan.intercutGroup") });
    }
    if (strict && scene.shots.length === 0) push("visualPlan.validation.shotsRequired", { label: sceneLabel });
    scene.shots.forEach((shot, shotIndex) => {
      const shotLabel = issueLabel("visualPlan.validation.labels.shotWithName", { name: shot.title || shot.id });
      if (!shot.id.trim()) push("visualPlan.validation.missingId", { label: issueLabel("visualPlan.validation.labels.shot") });
      else if (identityIds.has(shot.id)) push("visualPlan.validation.duplicateId", { label: issueLabel("visualPlan.validation.labels.shot"), value: shot.id });
      else identityIds.add(shot.id);
      requireText(shot.title, issueLabel("visualPlan.validation.labels.shotTitle", { id: shot.id }), push);
      requireText(shot.narrative_purpose, issueLabel("visualPlan.validation.labels.shotNarrativePurpose", { name: shot.title || shot.id }), push);
      if (!Number.isFinite(shot.duration_ms) || shot.duration_ms <= 0) push("visualPlan.validation.durationPositive", { label: shotLabel });
      if (strict && shot.keyframes.length < 2) push("visualPlan.validation.keyframesRequired", { label: shotLabel });
      let previousAt = -1;
      shot.keyframes.forEach((keyframe) => {
        validateKeyframe(keyframe, shot, entityIds, identityIds, push);
        if (keyframe.at_ms < previousAt) push("visualPlan.validation.keyframeTimeInvalid", { label: shotLabel });
        previousAt = keyframe.at_ms;
      });
      if (strict && shot.keyframes[0]?.at_ms !== 0) push("visualPlan.validation.firstKeyframeMissing", { label: shotLabel });
      if (strict && shot.keyframes.at(-1)?.at_ms !== shot.duration_ms) push("visualPlan.validation.lastKeyframeMissing", { label: shotLabel });
      allEvents(shot).forEach((event) => validateEvent(event, shot, entityIds, identityIds, push));
      if (shot.transition_to_next && shotIndex === scene.shots.length - 1) push("visualPlan.validation.transitionNotAtCut", { id: shot.id });
      if (shot.transition_to_next && shot.transition_to_next.kind === "continuous" && shot.transition_to_next.duration_ms !== 0) push("visualPlan.validation.continuousTransitionDuration", { id: shot.id });
      const nextShot = scene.shots[shotIndex + 1];
      if (shot.transition_to_next && nextShot && shot.transition_to_next.duration_ms > Math.min(shot.duration_ms, nextShot.duration_ms)) push("visualPlan.validation.transitionDurationOutOfRange", { id: shot.id });
    });
  });
  for (const [groupId, group] of intercutGroups) {
    if (group.modes.length < 2) push("visualPlan.validation.intercutGroupTooSmall", { groupId });
    if (group.modes.some((mode) => mode !== group.modes[0])) push("visualPlan.validation.intercutGroupModesMismatch", { groupId });
  }
  return issues;
};

export const formatPlanTime = (milliseconds: number) =>
  `${(milliseconds / 1000).toFixed(milliseconds % 1000 === 0 ? 0 : 1)}s`;

export const nearestKeyframe = (shot: VisualShotV2, atMs: number): VisualKeyframeV2 | null => {
  for (let index = shot.keyframes.length - 1; index >= 0; index -= 1) {
    const keyframe = shot.keyframes[index];
    if (keyframe.at_ms <= atMs) return keyframe;
  }
  return shot.keyframes[0] ?? null;
};

export const eventText = (event: VisualPlanTimedEventV2): string =>
  "content" in event ? event.content : event.description;

export const parseVisualEventParameters = (
  value: string,
): Record<string, string> | null => {
  try {
    const parsed: unknown = JSON.parse(value);
    if (parsed == null || typeof parsed !== "object" || Array.isArray(parsed)) return null;
    const entries = Object.entries(parsed);
    if (entries.some(([, entry]) => typeof entry !== "string")) return null;
    return Object.fromEntries(entries);
  } catch {
    return null;
  }
};
