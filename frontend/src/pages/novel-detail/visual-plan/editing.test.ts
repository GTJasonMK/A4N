import { describe, expect, it } from "vitest";
import type {
  ChapterVisualPlanV2,
  VisualEntityStateV1,
  VisualKeyframeV2,
} from "../../../api/chapterVisualPlan";
import {
  availableVisualKeyframeTime,
  collectVisualPlanIssues,
  isVisualKeyframeTimeAvailable,
  moveVisualScene,
  moveVisualShot,
  parseVisualEventParameters,
  planDurationMs,
  removeVisualScene,
  removeVisualEntity,
  removeVisualKeyframe,
  removeVisualPlanTimedEvent,
  removeVisualShot,
  resizeVisualShot,
  updateVisualPlanTimedEventRange,
  visualSceneStartMs,
  visualShotAt,
  visualShots,
} from "./editing";
import {
  ENTITY_CONTROL_SIZE,
  ENTITY_CONTROL_LAYER_Z_INDEX,
  ENTITY_RENDER_Z_INDEX_OFFSET,
  MAX_RENDER_Z_INDEX,
  MIN_ENTITY_SIZE,
  entityControlFramePosition,
  entityControlPosition,
  entitySelectionTransform,
  measuredEntityBounds,
  normalizeEntityTransform,
  renderedEntityBounds,
  resizeEntitySelectionByScreenDelta,
  resizeEntityTransformByScreenDelta,
  rotatedBoundsSize,
} from "./geometry";

const state = (entityId: string): VisualEntityStateV1 => ({
  entity_id: entityId,
  visible: true,
  bounds: { x: 0.1, y: 0.1, width: 0.2, height: 0.2 },
  rotation_degrees: 0,
  opacity: 1,
  z_index: 0,
  facing: "front",
  expression: null,
  pose: null,
  action: null,
  gaze_target_entity_id: null,
  dialogue: null,
  notes: null,
});

const keyframe = (id: string, atMs: number, entityId = "hero"): VisualKeyframeV2 => ({
  id,
  at_ms: atMs,
  label: id,
  interpolation_to_next: "linear",
  camera: {
    center_x: 0.5,
    center_y: 0.5,
    focus_entity_id: null,
    framing_note: null,
    rotation_degrees: 0,
    zoom: 1,
  },
  entity_states: [state(entityId)],
  relations: [],
});

const shot = (id: string, durationMs: number): ChapterVisualPlanV2["scenes"][number]["shots"][number] => ({
  id,
  title: id,
  duration_ms: durationMs,
  narrative_purpose: "推进情节",
  coverage: "must_show",
  shot_size: "medium",
  camera_angle: "eye_level",
  camera_height: "eye",
  lens_intent: null,
  composition: null,
  keyframes: [keyframe(`${id}-start`, 0), keyframe(`${id}-end`, durationMs)],
  entity_events: [],
  camera_events: [],
  sound_events: [],
  technique_events: [],
  continuity_notes: [],
  transition_to_next: null,
});

const plan: ChapterVisualPlanV2 = {
  schema_version: "chapter_visual_plan_v2",
  frame: { width: 1920, height: 1080, background: null },
  entities: [{ id: "hero", label: "主角", kind: "character", description: "主角" }],
  scenes: [{
    id: "scene",
    title: "场景",
    narrative_purpose: "完成线索发现",
    editing_mode: "continuity",
    intercut_group_id: null,
    location: null,
    time_of_day: null,
    participant_entity_ids: ["hero"],
    focal_entity_id: "hero",
    shots: [shot("a", 1000), shot("b", 2000)],
  }],
};

describe("V2 visual plan editing", () => {
  it("flattens scene shots to absolute ranges and selects the next shot at a cut", () => {
    const shots = visualShots(plan);
    expect(shots.map((ref) => [ref.start_ms, ref.end_ms])).toEqual([[0, 1000], [1000, 3000]]);
    expect(planDurationMs(plan)).toBe(3000);
    expect(visualShotAt(plan, 1000, shots)?.shot.id).toBe("b");
    expect(visualShotAt(plan, 3000, shots)?.shot.id).toBe("b");
  });

  it("resizes only relative shot content while later shot ranges follow the derived timeline", () => {
    const next = structuredClone(plan);
    const first = next.scenes[0].shots[0];
    first.keyframes[1].at_ms = 500;
    first.camera_events.push({ id: "camera", kind: "pan", start_ms: 250, end_ms: 750, target_entity_id: null, description: "移动", parameters: {} });
    first.transition_to_next = { kind: "dissolve", duration_ms: 800, preserved_action: null, preserved_sound: null, notes: null };
    const resized = resizeVisualShot(next, "a", 1500);
    expect(resized.scenes[0].shots[0].duration_ms).toBe(1500);
    expect(resized.scenes[0].shots[0].keyframes[1].at_ms).toBe(750);
    expect(resized.scenes[0].shots[0].camera_events[0].start_ms).toBe(375);
    expect(visualShots(resized)[1].start_ms).toBe(1500);
    expect(planDurationMs(resized)).toBe(3500);

    const shorterNextShot = resizeVisualShot(resized, "b", 400);
    expect(shorterNextShot.scenes[0].shots[0].transition_to_next?.duration_ms).toBe(400);
  });

  it("reorders shots and preserves transitions at the same timeline boundary", () => {
    const next = structuredClone(plan);
    next.scenes[0].shots[0].transition_to_next = { kind: "dissolve", duration_ms: 200, preserved_action: null, preserved_sound: null, notes: "保留边界" };
    const reordered = moveVisualShot(next, "b", -1);
    expect(reordered.scenes[0].shots.map((value) => value.id)).toEqual(["b", "a"]);
    expect(reordered.scenes[0].shots[0].transition_to_next?.kind).toBe("dissolve");
    expect(reordered.scenes[0].shots[1].transition_to_next).toBeNull();
  });

  it("reorders scenes and keeps empty scenes addressable by timeline position", () => {
    const next = structuredClone(plan);
    const second = structuredClone(next.scenes[0]);
    second.id = "scene-2";
    second.title = "第二场景";
    second.shots = [shot("c", 500)];
    const empty = structuredClone(second);
    empty.id = "scene-empty";
    empty.title = "空场景";
    empty.shots = [];
    next.scenes.push(empty, second);

    expect(visualSceneStartMs(next, "scene-empty")).toBe(3000);
    expect(visualSceneStartMs(next, "scene-2")).toBe(3000);
    const reordered = moveVisualScene(next, "scene-empty", -1);
    expect(reordered.scenes.map((scene) => scene.id)).toEqual(["scene-empty", "scene", "scene-2"]);
    expect(visualSceneStartMs(reordered, "scene-2")).toBe(3000);
  });

  it("removes scenes and allows a shot to be deleted until its scene is empty", () => {
    const oneShot = structuredClone(plan);
    oneShot.scenes[0].shots = [oneShot.scenes[0].shots[0]];
    const emptyScene = removeVisualShot(oneShot, "a");
    expect(emptyScene.scenes[0].shots).toHaveLength(0);

    const withTransition = structuredClone(plan);
    withTransition.scenes[0].shots[0].transition_to_next = { kind: "dissolve", duration_ms: 200, preserved_action: null, preserved_sound: null, notes: null };
    const withoutLastShot = removeVisualShot(withTransition, "b");
    expect(withoutLastShot.scenes[0].shots[0].transition_to_next).toBeNull();

    const withoutScene = removeVisualScene(plan, "scene");
    expect(withoutScene.scenes).toHaveLength(0);
  });

  it("keeps the transition before a deleted middle shot and normalizes its new boundary", () => {
    const next = structuredClone(plan);
    const middle = shot("middle", 1500);
    next.scenes[0].shots[0].transition_to_next = { kind: "dissolve", duration_ms: 1800, preserved_action: null, preserved_sound: null, notes: null };
    middle.transition_to_next = { kind: "fade", duration_ms: 1200, preserved_action: null, preserved_sound: null, notes: null };
    next.scenes[0].shots.splice(1, 0, middle);

    const withoutMiddle = removeVisualShot(next, "middle");
    expect(withoutMiddle.scenes[0].shots.map((value) => value.id)).toEqual(["a", "b"]);
    expect(withoutMiddle.scenes[0].shots[0].transition_to_next?.kind).toBe("dissolve");
    expect(withoutMiddle.scenes[0].shots[0].transition_to_next?.duration_ms).toBe(1000);
    expect(withoutMiddle.scenes[0].shots[1].transition_to_next).toBeNull();
  });

  it("removes an entity and every authored reference to it", () => {
    const next = structuredClone(plan);
    const currentShot = next.scenes[0].shots[0];
    const currentKeyframe = currentShot.keyframes[0];
    currentKeyframe.camera.focus_entity_id = "hero";
    currentKeyframe.entity_states[0].gaze_target_entity_id = "hero";
    currentKeyframe.relations.push({ id: "relation", source_entity_id: "hero", target_entity_id: "hero", kind: "looking_at", label: null });
    currentShot.entity_events.push({ id: "entity-event", kind: "action", start_ms: 0, end_ms: 100, entity_id: "hero", target_entity_id: "hero", description: "动作", parameters: {} });
    currentShot.camera_events.push({ id: "camera-event", kind: "reframe", start_ms: 0, end_ms: 100, target_entity_id: "hero", description: "构图", parameters: {} });
    currentShot.sound_events.push({ id: "sound-event", kind: "dialogue", start_ms: 0, end_ms: 100, speaker_entity_id: "hero", content: "对白", parameters: {} });

    const withoutEntity = removeVisualEntity(next, "hero");
    expect(withoutEntity.entities).toHaveLength(0);
    expect(withoutEntity.scenes[0].participant_entity_ids).toEqual([]);
    expect(withoutEntity.scenes[0].focal_entity_id).toBeNull();
    expect(withoutEntity.scenes[0].shots[0].keyframes[0].entity_states).toEqual([]);
    expect(withoutEntity.scenes[0].shots[0].keyframes[0].relations).toEqual([]);
    expect(withoutEntity.scenes[0].shots[0].keyframes[0].camera.focus_entity_id).toBeNull();
    expect(withoutEntity.scenes[0].shots[0].entity_events).toEqual([]);
    expect(withoutEntity.scenes[0].shots[0].camera_events).toEqual([]);
    expect(withoutEntity.scenes[0].shots[0].sound_events).toEqual([]);
  });

  it("keeps the two shot boundary keyframes", () => {
    const withoutStart = removeVisualKeyframe(plan, "a", "a-start");
    expect(withoutStart.scenes[0].shots[0].keyframes.map((value) => value.id)).toEqual(["a-start", "a-end"]);

    const withExtra = structuredClone(plan);
    withExtra.scenes[0].shots[0].keyframes.splice(1, 0, keyframe("middle", 500));
    const withoutMiddle = removeVisualKeyframe(withExtra, "a", "middle");
    expect(withoutMiddle.scenes[0].shots[0].keyframes.map((value) => value.id)).toEqual(["a-start", "a-end"]);
  });

  it("keeps keyframe times finite, inside the shot, and unique", () => {
    const currentShot = plan.scenes[0].shots[0];
    expect(isVisualKeyframeTimeAvailable(currentShot, null, 500)).toBe(true);
    expect(isVisualKeyframeTimeAvailable(currentShot, null, 0)).toBe(false);
    expect(isVisualKeyframeTimeAvailable(currentShot, "a-start", 0)).toBe(true);
    expect(isVisualKeyframeTimeAvailable(currentShot, "a-start", 1)).toBe(false);
    expect(isVisualKeyframeTimeAvailable(currentShot, "a-end", currentShot.duration_ms)).toBe(true);
    expect(isVisualKeyframeTimeAvailable(currentShot, "a-end", currentShot.duration_ms - 1)).toBe(false);
    expect(isVisualKeyframeTimeAvailable(currentShot, null, 1001)).toBe(false);
    expect(isVisualKeyframeTimeAvailable(currentShot, null, Number.NaN)).toBe(false);
  });

  it("finds an available keyframe time when the playhead is already occupied", () => {
    const currentShot = plan.scenes[0].shots[0];
    expect(availableVisualKeyframeTime(currentShot, 0)).toBe(1);
    expect(availableVisualKeyframeTime(currentShot, 500)).toBe(500);

    const fullShot = { ...currentShot, duration_ms: 2, keyframes: [keyframe("zero", 0), keyframe("one", 1), keyframe("two", 2)] };
    expect(availableVisualKeyframeTime(fullShot, 1)).toBeNull();
  });

  it("keeps resized keyframes ordered when rounding would collide", () => {
    const next = structuredClone(plan);
    const currentShot = next.scenes[0].shots[0];
    currentShot.keyframes.splice(1, 0, keyframe("near-start", 1));
    const resized = resizeVisualShot(next, "a", 250);
    expect(resized.scenes[0].shots[0].keyframes.map(({ at_ms }) => at_ms)).toEqual([0, 1, 250]);
  });

  it("keeps malformed and rotated entity bounds inside the frame", () => {
    const transform = normalizeEntityTransform(
      { x: 0.88, y: 0.82, width: 0.3, height: 0.001 },
      47,
      undefined,
      { width: 1920, height: 1080 },
    );
    const rendered = renderedEntityBounds(transform, { width: 1920, height: 1080 });

    expect(transform.bounds.width).toBeGreaterThanOrEqual(MIN_ENTITY_SIZE);
    expect(transform.bounds.height).toBeGreaterThanOrEqual(MIN_ENTITY_SIZE);
    expect(rendered.x).toBeGreaterThanOrEqual(0);
    expect(rendered.y).toBeGreaterThanOrEqual(0);
    expect(rendered.x + rendered.width).toBeLessThanOrEqual(1);
    expect(rendered.y + rendered.height).toBeLessThanOrEqual(1);

    const rotated = rotatedBoundsSize(transform.bounds, transform.rotation, { width: 1920, height: 1080 });
    const centerX = transform.bounds.x + transform.bounds.width / 2;
    const centerY = transform.bounds.y + transform.bounds.height / 2;
    expect(centerX - rotated.width / 2).toBeGreaterThanOrEqual(-Number.EPSILON);
    expect(centerY - rotated.height / 2).toBeGreaterThanOrEqual(-Number.EPSILON);
    expect(centerX + rotated.width / 2).toBeLessThanOrEqual(1 + Number.EPSILON);
    expect(centerY + rotated.height / 2).toBeLessThanOrEqual(1 + Number.EPSILON);
  });

  it("keeps rotated bounds inside the measured canvas ratio", () => {
    const frame = { width: 342, height: 192 };
    const transform = normalizeEntityTransform(
      { x: 0, y: 0, width: 0.2, height: 0.2 },
      45,
      undefined,
      frame,
    );
    const rendered = renderedEntityBounds(transform, frame);

    expect(rendered.x).toBeGreaterThanOrEqual(0);
    expect(rendered.y).toBeGreaterThanOrEqual(0);
    expect(rendered.x + rendered.width).toBeLessThanOrEqual(1);
    expect(rendered.y + rendered.height).toBeLessThanOrEqual(1);
  });

  it("keeps the control layer above every render z-index", () => {
    expect(ENTITY_CONTROL_LAYER_Z_INDEX).toBeGreaterThan(
      ENTITY_RENDER_Z_INDEX_OFFSET + MAX_RENDER_Z_INDEX,
    );
  });

  it("keeps every rotated corner inside the frame at each edge", () => {
    const angles = [0, 15, 45, 90, 135, 225, 315];
    const positions = [
      { x: 0, y: 0 },
      { x: 0.8, y: 0 },
      { x: 0, y: 0.8 },
      { x: 0.8, y: 0.8 },
    ];

    angles.forEach((rotation) => positions.forEach(({ x, y }) => {
      const transform = normalizeEntityTransform(
        { x, y, width: 0.2, height: 0.2 },
        rotation,
        undefined,
        { width: 1920, height: 1080 },
      );
      const rendered = renderedEntityBounds(transform, { width: 1920, height: 1080 });

      expect(rendered.x).toBeGreaterThanOrEqual(0);
      expect(rendered.y).toBeGreaterThanOrEqual(0);
      expect(rendered.x + rendered.width).toBeLessThanOrEqual(1);
      expect(rendered.y + rendered.height).toBeLessThanOrEqual(1);
      const rotated = rotatedBoundsSize(transform.bounds, transform.rotation, { width: 1920, height: 1080 });
      const centerX = transform.bounds.x + transform.bounds.width / 2;
      const centerY = transform.bounds.y + transform.bounds.height / 2;
      expect(centerX - rotated.width / 2).toBeGreaterThanOrEqual(-Number.EPSILON);
      expect(centerY - rotated.height / 2).toBeGreaterThanOrEqual(-Number.EPSILON);
      expect(centerX + rotated.width / 2).toBeLessThanOrEqual(1 + Number.EPSILON);
      expect(centerY + rotated.height / 2).toBeLessThanOrEqual(1 + Number.EPSILON);
    }));
  });

  it("keeps rotated control hit areas inside the real frame", () => {
    const frame = { width: 344, height: 194 };
    const transform = normalizeEntityTransform(
      { x: 0, y: 0, width: 0.2, height: 0.24 },
      45,
      undefined,
      { width: 1920, height: 1080 },
    );
    const half = ENTITY_CONTROL_SIZE / 2;
    const extent = half * (Math.abs(Math.cos(Math.PI / 4)) + Math.abs(Math.sin(Math.PI / 4)));
    const centerX = (transform.bounds.x + transform.bounds.width / 2) * frame.width;
    const centerY = (transform.bounds.y + transform.bounds.height / 2) * frame.height;

    (["nw", "ne", "se", "sw", "rotate", "center"] as const).forEach((anchor) => {
      const position = entityControlPosition(transform, anchor, frame);
      const localX = position.left - transform.bounds.width * frame.width / 2;
      const localY = position.top - transform.bounds.height * frame.height / 2;
      const screenX = centerX + localX * Math.cos(Math.PI / 4) - localY * Math.sin(Math.PI / 4);
      const screenY = centerY + localX * Math.sin(Math.PI / 4) + localY * Math.cos(Math.PI / 4);
      expect(screenX).toBeGreaterThanOrEqual(extent - Number.EPSILON);
      expect(screenX).toBeLessThanOrEqual(frame.width - extent + Number.EPSILON);
      expect(screenY).toBeGreaterThanOrEqual(extent - Number.EPSILON);
      expect(screenY).toBeLessThanOrEqual(frame.height - extent + Number.EPSILON);
    });
  });

  it("uses the normalized entity bounds for its selection", () => {
    const transform = normalizeEntityTransform(
      { x: 0.2, y: 0.25, width: 0.4, height: 0.3 },
      0,
      undefined,
      { width: 1920, height: 1080 },
    );

    expect(entitySelectionTransform(transform)).toEqual(transform);
  });

  it("keeps a measured text label tighter than the editable entity minimum", () => {
    const transform = normalizeEntityTransform(
      { x: 0.6, y: 0.4, width: 0.12, height: 0.13 },
      0,
      undefined,
      { width: 1920, height: 1080 },
    );
    const measured = measuredEntityBounds(
      transform,
      { left: 57, top: 25, width: 18, height: 35 },
      { width: 1140, height: 641 },
    );

    expect(measured.width).toBeCloseTo(18 / 1140);
    expect(measured.height).toBeCloseTo(35 / 641);
    expect(entitySelectionTransform({ bounds: measured, rotation: 0 }).bounds).toEqual(measured);
  });

  it("resizes the actual entity when its tight selection frame is resized", () => {
    const transform = normalizeEntityTransform(
      { x: 0.2, y: 0.25, width: 0.3, height: 0.25 },
      12,
      undefined,
      { width: 1920, height: 1080 },
    );
    const resized = resizeEntitySelectionByScreenDelta(
      transform,
      "se",
      120,
      48,
      { width: 600, height: 338 },
    );

    expect(resized.bounds.width).toBeGreaterThan(transform.bounds.width);
    expect(resized.bounds.height).toBeGreaterThan(transform.bounds.height);
    expect(resized.rotation).toBe(transform.rotation);
  });

  it("keeps selection controls inside the measured canvas", () => {
    const frame = { width: 344, height: 194 };
    const transform = normalizeEntityTransform(
      { x: 0, y: 0, width: 0.2, height: 0.24 },
      45,
      undefined,
      { width: 1920, height: 1080 },
    );
    const selection = entitySelectionTransform(transform);
    const half = ENTITY_CONTROL_SIZE / 2;

    (["nw", "ne", "se", "sw"] as const).forEach((anchor) => {
      const position = entityControlFramePosition(selection, anchor, frame);
      expect(position.left).toBeGreaterThanOrEqual(half);
      expect(position.left).toBeLessThanOrEqual(frame.width - half);
      expect(position.top).toBeGreaterThanOrEqual(half);
      expect(position.top).toBeLessThanOrEqual(frame.height - half);
    });
  });

  it("keeps the rotation hit area above a tight selection", () => {
    const frame = { width: 344, height: 194 };
    const transform = normalizeEntityTransform(
      { x: 0.68, y: 0.42, width: 0.12, height: 0.12 },
      0,
      undefined,
      { width: 1920, height: 1080 },
    );
    const selection = entitySelectionTransform(transform);
    const position = entityControlFramePosition(selection, "rotate", frame);
    const selectionTop = selection.bounds.y * frame.height;

    expect(position.top).toBeLessThan(selectionTop);
    expect(position.top).toBeGreaterThanOrEqual(ENTITY_CONTROL_SIZE / 2);
  });

  it("maps selected controls to frame coordinates with one rotation", () => {
    const frame = { width: 344, height: 194 };
    const transform = normalizeEntityTransform(
      { x: 0.4, y: 0.34, width: 0.2, height: 0.24 },
      32,
      undefined,
      { width: 1920, height: 1080 },
    );
    const radians = transform.rotation * Math.PI / 180;
    const centerX = (transform.bounds.x + transform.bounds.width / 2) * frame.width;
    const centerY = (transform.bounds.y + transform.bounds.height / 2) * frame.height;
    const localX = transform.bounds.width * frame.width / 2;
    const localY = -transform.bounds.height * frame.height / 2;
    const expected = {
      left: centerX + localX * Math.cos(radians) - localY * Math.sin(radians),
      top: centerY + localX * Math.sin(radians) + localY * Math.cos(radians),
    };

    expect(entityControlFramePosition(transform, "ne", frame)).toEqual({
      left: expect.closeTo(expected.left, 8),
      top: expect.closeTo(expected.top, 8),
    });
  });

  it("resizes with the same local coordinate system used by pointer and keyboard controls", () => {
    const transform = normalizeEntityTransform(
      { x: 0.2, y: 0.25, width: 0.3, height: 0.25 },
      -18,
      undefined,
      { width: 1920, height: 1080 },
    );
    const resized = resizeEntityTransformByScreenDelta(transform, "se", 120, -40, { width: 1920, height: 1080 });
    const rotated = rotatedBoundsSize(resized.bounds, resized.rotation, { width: 1920, height: 1080 });
    const centerX = resized.bounds.x + resized.bounds.width / 2;
    const centerY = resized.bounds.y + resized.bounds.height / 2;
    expect(centerX - rotated.width / 2).toBeGreaterThanOrEqual(-Number.EPSILON);
    expect(centerY - rotated.height / 2).toBeGreaterThanOrEqual(-Number.EPSILON);
    expect(centerX + rotated.width / 2).toBeLessThanOrEqual(1 + Number.EPSILON);
    expect(centerY + rotated.height / 2).toBeLessThanOrEqual(1 + Number.EPSILON);
  });

  it("updates and removes relative timed events", () => {
    const next = structuredClone(plan);
    next.scenes[0].shots[0].sound_events.push({ id: "sound", kind: "effect", start_ms: 100, end_ms: 400, speaker_entity_id: null, content: "门响", parameters: {} });
    expect(updateVisualPlanTimedEventRange(next, "a", "sound", "start", 200)).toBe(true);
    expect(next.scenes[0].shots[0].sound_events[0].start_ms).toBe(200);
    expect(removeVisualPlanTimedEvent(next, "a", "sound")).toBe(true);
    expect(next.scenes[0].shots[0].sound_events).toHaveLength(0);
  });

  it("accepts only JSON objects whose event parameter values are strings", () => {
    expect(parseVisualEventParameters('{"direction":"left"}')).toEqual({ direction: "left" });
    expect(parseVisualEventParameters("[]")).toBeNull();
    expect(parseVisualEventParameters('{"amount":2}')).toBeNull();
    expect(parseVisualEventParameters("invalid")).toBeNull();
  });

  it("reports strict confirmation gaps while allowing an empty draft", () => {
    const empty: ChapterVisualPlanV2 = { ...structuredClone(plan), scenes: [] };
    expect(collectVisualPlanIssues(empty)).toEqual([]);
    expect(collectVisualPlanIssues(empty, true).map((issue) => issue.key)).toContain("visualPlan.validation.scenesRequired");
  });

  it("requires identity IDs to be unique across entity and scene kinds", () => {
    const next = structuredClone(plan);
    next.scenes[0].id = "hero";
    expect(collectVisualPlanIssues(next).map((issue) => issue.key)).toContain("visualPlan.validation.duplicateId");
  });

  it("reports invalid keyframe references before save", () => {
    const next = structuredClone(plan);
    next.scenes[0].shots[0].keyframes[0].entity_states[0].entity_id = "missing";
    expect(collectVisualPlanIssues(next).map((issue) => issue.key)).toContain("visualPlan.validation.missingReferenceId");
  });

  it("matches backend validation for empty optional references and intercut groups", () => {
    const next = structuredClone(plan);
    next.frame.background = "  ";
    next.scenes[0].participant_entity_ids.push("hero");
    next.scenes[0].intercut_group_id = "group-1";
    next.scenes[0].shots[0].keyframes[0].camera.focus_entity_id = "";
    next.scenes[0].shots[0].keyframes[0].relations = [{
      id: "",
      kind: "looking_at",
      source_entity_id: "hero",
      target_entity_id: "hero",
      label: null,
    }];
    next.scenes[0].shots[0].camera_events.push({
      id: "",
      kind: "pan",
      start_ms: 0,
      end_ms: 100,
      target_entity_id: "missing",
      description: "推镜",
      parameters: {},
    });
    const issues = collectVisualPlanIssues(next).map((issue) => issue.key);
    expect(issues).toContain("visualPlan.validation.backgroundNotEmpty");
    expect(issues).toContain("visualPlan.validation.duplicateId");
    expect(issues).toContain("visualPlan.validation.missingId");
    expect(issues).toContain("visualPlan.validation.intercutGroupNotAllowed");
    expect(issues.filter((key) => key === "visualPlan.validation.missingReferenceId").length).toBeGreaterThan(0);

    const grouped = structuredClone(plan);
    grouped.scenes[0].editing_mode = "intercut";
    grouped.scenes[0].intercut_group_id = "group-1";
    expect(collectVisualPlanIssues(grouped).map((issue) => issue.key)).toContain("visualPlan.validation.intercutGroupTooSmall");
  });

  it("requires visible entity states for focus and relations", () => {
    const next = structuredClone(plan);
    next.entities.push({ id: "other", label: "其他", kind: "character", description: "其他实体" });
    const hidden = state("hero");
    hidden.visible = false;
    const frame = next.scenes[0].shots[0].keyframes[0];
    frame.entity_states = [hidden, state("other")];
    frame.camera.focus_entity_id = "hero";
    frame.relations = [{
      id: "relation",
      kind: "left_of",
      source_entity_id: "hero",
      target_entity_id: "other",
      label: null,
    }];
    const issues = collectVisualPlanIssues(next).map((issue) => issue.key);
    expect(issues).toContain("visualPlan.validation.focusEntityStateMissing");
    expect(issues).toContain("visualPlan.validation.relationSourceStateMissing");
  });
});
