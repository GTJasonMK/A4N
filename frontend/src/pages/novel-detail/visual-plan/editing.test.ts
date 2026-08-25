import { describe, expect, it } from "vitest";
import type {
  ChapterVisualPlanV1,
  VisualEntityStateV1,
} from "../../../api/chapterVisualPlan";
import {
  collectVisualPlanIssues as collectVisualPlanIssueRecords,
  detachVisualPlanSourceSpanReferences,
  moveVisualShot,
  removeVisualPlanTimedEvent,
  resizeVisualShot,
  shouldAutoSaveVisualPlanDraft,
  translateVisualPlanIssue,
  updateVisualPlanTimedEventRange,
  visualShotAt,
} from "./editing";
import { translateMessage } from "../../../i18n";

const t = (key: Parameters<typeof translateMessage>[1], params?: Parameters<typeof translateMessage>[2]) =>
  translateMessage("zh-CN", key, params);

const collectVisualPlanIssues = (
  value: ChapterVisualPlanV1,
  availableSourceSpanIds?: readonly string[],
) => collectVisualPlanIssueRecords(value, availableSourceSpanIds)
  .map((issue) => translateVisualPlanIssue(issue, t));

const entityState = (entityId: string): VisualEntityStateV1 => ({
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

const plan: ChapterVisualPlanV1 = {
  schema_version: "chapter_visual_plan_v1",
  frame: { width: 1920, height: 1080, duration_ms: 2000, background: null },
  directing_statement: {
    central_dramatic_question: "能否发现线索？",
    protagonist_throughline: "主角持续寻找证据",
    audience_experience: "从平静过渡到警觉",
    viewpoint_strategy: "限制视角",
    visual_strategy: "用构图变化强调发现",
    spatial_strategy: "保持空间连续",
    sound_strategy: "环境音逐步收窄",
    rhythm_strategy: "前缓后快",
  },
  entities: [
    {
      id: "a",
      label: "A",
      kind: "character",
      description: "人物 A",
      source_span_ids: [],
      user_addition_ids: ["addition"],
    },
    {
      id: "b",
      label: "B",
      kind: "character",
      description: "人物 B",
      source_span_ids: [],
      user_addition_ids: ["addition"],
    },
  ],
  user_additions: [{
    id: "addition",
    kind: "story_fact",
    description: "测试方案事实",
    rationale: "测试完整结构",
    introduced_at_ms: null,
  }],
  narrative_units: [{
    id: "unit",
    sequence_index: 1,
    timeline_position: 1,
    title: "单元",
    start_ms: 0,
    end_ms: 2000,
    editing_mode: "continuity",
    intercut_group_id: null,
    narrative_purpose: "完成线索发现",
    location: null,
    time_of_day: null,
    participant_entity_ids: ["a", "b"],
    focal_entity_id: null,
    source_span_ids: [],
    user_addition_ids: ["addition"],
    shots: [
      {
        id: "a",
        title: "A",
        start_ms: 0,
        end_ms: 1000,
        source_span_ids: [],
        user_addition_ids: ["addition"],
        narrative_purpose: "建立环境",
        coverage: "must_show",
        shot_size: null,
        camera_angle: null,
        camera_height: null,
        lens_intent: null,
        composition: null,
        keyframes: [
          {
            id: "a-start",
            at_ms: 0,
            label: "A start",
            interpolation_to_next: "hold",
            camera: {
              center_x: 0.5,
              center_y: 0.5,
              zoom: 1,
              rotation_degrees: 0,
              focus_entity_id: null,
              framing_note: null,
            },
            entity_states: [],
            relations: [],
          },
          {
            id: "a-end",
            at_ms: 1000,
            label: "A end",
            interpolation_to_next: "hold",
            camera: {
              center_x: 0.5,
              center_y: 0.5,
              zoom: 1,
              rotation_degrees: 0,
              focus_entity_id: null,
              framing_note: null,
            },
            entity_states: [],
            relations: [],
          },
        ],
        entity_events: [],
        camera_events: [],
        sound_events: [],
        technique_events: [],
        continuity_notes: [],
      },
      {
        id: "b",
        title: "B",
        start_ms: 1000,
        end_ms: 2000,
        source_span_ids: [],
        user_addition_ids: ["addition"],
        narrative_purpose: "揭示线索",
        coverage: "must_show",
        shot_size: null,
        camera_angle: null,
        camera_height: null,
        lens_intent: null,
        composition: null,
        keyframes: [
          {
            id: "b-start",
            at_ms: 1000,
            label: "B start",
            interpolation_to_next: "hold",
            camera: {
              center_x: 0.5,
              center_y: 0.5,
              zoom: 1,
              rotation_degrees: 0,
              focus_entity_id: null,
              framing_note: null,
            },
            entity_states: [],
            relations: [],
          },
          {
            id: "b-end",
            at_ms: 2000,
            label: "B end",
            interpolation_to_next: "hold",
            camera: {
              center_x: 0.5,
              center_y: 0.5,
              zoom: 1,
              rotation_degrees: 0,
              focus_entity_id: null,
              framing_note: null,
            },
            entity_states: [],
            relations: [],
          },
        ],
        entity_events: [],
        camera_events: [],
        sound_events: [],
        technique_events: [],
        continuity_notes: [],
      },
    ],
  }],
  transitions: [{
    id: "transition",
    from_shot_id: "a",
    to_shot_id: "b",
    at_ms: 1000,
    duration_ms: 0,
    kind: "continuous",
    preserved_action: null,
    preserved_sound: null,
    notes: null,
  }],
  source_dispositions: [],
};

const planWithTwoUnits = () => {
  const next = structuredClone(plan);
  const original = next.narrative_units[0];
  const secondShot = original.shots[1];
  original.end_ms = 1000;
  original.shots = [original.shots[0]];
  next.narrative_units.push({
    ...structuredClone(original),
    id: "unit-b",
    sequence_index: 2,
    timeline_position: 2,
    title: "单元 B",
    start_ms: 1000,
    end_ms: 2000,
    shots: [secondShot],
  });
  return next;
};

describe("visual plan timeline editing", () => {
  it("does not autosave an initial mode selection before AI generation", () => {
    expect(shouldAutoSaveVisualPlanDraft({
      hasDraft: false,
      planDirty: false,
      modeDirty: true,
    })).toBe(false);
    expect(shouldAutoSaveVisualPlanDraft({
      hasDraft: false,
      planDirty: true,
      modeDirty: true,
    })).toBe(true);
    expect(shouldAutoSaveVisualPlanDraft({
      hasDraft: true,
      planDirty: false,
      modeDirty: true,
    })).toBe(true);
  });

  it("accepts the complete baseline fixture", () => {
    expect(collectVisualPlanIssues(plan)).toEqual([]);
  });

  it("resizes one shot and shifts everything after it", () => {
    const next = resizeVisualShot(plan, "a", 1500);
    expect(next.narrative_units[0].shots[0].end_ms).toBe(1500);
    expect(next.narrative_units[0].shots[1].start_ms).toBe(1500);
    expect(next.narrative_units[0].end_ms).toBe(2500);
    expect(next.frame.duration_ms).toBe(2500);
    expect(next.transitions[0].at_ms).toBe(1500);
  });

  it("ignores a non-finite shot duration", () => {
    const next = resizeVisualShot(plan, "a", Number.NaN);
    expect(next).toEqual(plan);
  });

  it("reorders shots and rebinds the transition to the preserved cut", () => {
    const next = moveVisualShot(plan, "b", -1);
    expect(next.narrative_units[0].shots.map((shot) => shot.id)).toEqual(["b", "a"]);
    expect(next.narrative_units[0].shots[0].start_ms).toBe(0);
    expect(next.narrative_units[0].shots[1].start_ms).toBe(1000);
    expect(next.transitions[0]).toMatchObject({
      from_shot_id: "b",
      to_shot_id: "a",
      at_ms: 1000,
      duration_ms: 0,
    });
  });

  it("moves a shot-only user addition with the shot", () => {
    const next = structuredClone(plan);
    next.user_additions[0].introduced_at_ms = 250;
    next.entities.forEach((entity) => { entity.user_addition_ids = []; });
    next.narrative_units[0].user_addition_ids = [];
    next.narrative_units[0].shots[0].user_addition_ids = ["addition"];
    next.narrative_units[0].shots[1].user_addition_ids = [];
    const reordered = moveVisualShot(next, "a", 1);
    expect(reordered.user_additions[0].introduced_at_ms).toBe(1250);
  });

  it("does not move a user addition shared by the unit", () => {
    const next = structuredClone(plan);
    next.user_additions[0].introduced_at_ms = 250;
    next.narrative_units[0].shots[0].user_addition_ids = ["addition"];
    next.narrative_units[0].shots[1].user_addition_ids = [];
    const reordered = moveVisualShot(next, "a", 1);
    expect(reordered.user_additions[0].introduced_at_ms).toBe(250);
  });

  it("selects the next shot at a shared boundary", () => {
    expect(visualShotAt(plan, 1000)?.shot.id).toBe("b");
    expect(visualShotAt(plan, 2000)?.shot.id).toBe("b");
  });

  it("reports duplicated relation ids before saving", () => {
    const next = structuredClone(plan);
    next.narrative_units[0].shots[0].keyframes[0].relations = [{
      id: "relation",
      kind: "looking_at",
      source_entity_id: "a",
      target_entity_id: "b",
      label: null,
    }];
    next.narrative_units[0].shots[0].keyframes[1].relations = [{
      ...next.narrative_units[0].shots[0].keyframes[0].relations[0],
    }];
    expect(collectVisualPlanIssues(next)).toContain("位置关系 ID 重复：relation");
  });

  it("requires relation endpoints in the current keyframe and labels custom relations", () => {
    const next = structuredClone(plan);
    next.narrative_units[0].shots[0].keyframes[0].entity_states = [entityState("a")];
    next.narrative_units[0].shots[0].keyframes[0].relations = [{
      id: "relation",
      kind: "custom",
      source_entity_id: "a",
      target_entity_id: "b",
      label: null,
    }];
    const issues = collectVisualPlanIssues(next);
    expect(issues).toContain("关键帧 A start 的位置关系目标实体必须定义当前状态");
    expect(issues).toContain("关键帧 A start 的自定义位置关系必须填写说明");

    next.narrative_units[0].shots[0].keyframes[0].relations[0] = {
      ...next.narrative_units[0].shots[0].keyframes[0].relations[0],
      target_entity_id: "a",
      label: "与内心倒影重合",
    };
    expect(collectVisualPlanIssues(next)).toContain(
      "关键帧 A start 的位置关系不能指向同一实体",
    );
  });

  it("removes a timed event and only its now-unreferenced user addition", () => {
    const next = structuredClone(plan);
    const shot = next.narrative_units[0].shots[0];
    next.user_additions.push({
      id: "event-only",
      kind: "action",
      description: "只属于事件",
      rationale: "测试删除",
      introduced_at_ms: 100,
    });
    shot.entity_events.push({
      id: "event",
      kind: "action",
      start_ms: 100,
      end_ms: 200,
      entity_id: "a",
      target_entity_id: null,
      description: "抬手",
      source_span_ids: [],
      user_addition_ids: ["addition", "event-only"],
      parameters: {},
    });
    expect(removeVisualPlanTimedEvent(next, shot.id, "event")).toBe(true);
    expect(shot.entity_events).toEqual([]);
    expect(next.user_additions.map((addition) => addition.id)).toEqual(["addition"]);
  });

  it("clamps precise event range edits to the selected shot", () => {
    const next = structuredClone(plan);
    const shot = next.narrative_units[0].shots[0];
    shot.camera_events.push({
      id: "camera",
      kind: "push_in",
      start_ms: 100,
      end_ms: 300,
      target_entity_id: null,
      description: "推近",
      source_span_ids: [],
      user_addition_ids: ["addition"],
      parameters: {},
    });
    expect(updateVisualPlanTimedEventRange(next, shot.id, "camera", "start", 999)).toBe(true);
    expect(shot.camera_events[0].start_ms).toBe(299);
    expect(updateVisualPlanTimedEventRange(next, shot.id, "camera", "end", 9_999)).toBe(true);
    expect(shot.camera_events[0].end_ms).toBe(1000);
  });

  it("keeps ids unique inside their own namespace", () => {
    expect(collectVisualPlanIssues(plan)).not.toContain("镜头 ID 重复：a");
  });

  it("reports bounds, source coverage and disposition mismatches", () => {
    const next = structuredClone(plan);
    next.narrative_units[0].shots[0].keyframes[0].entity_states = [{
      ...entityState("a"),
      bounds: { x: 0.9, y: 0, width: 0.2, height: 0.2 },
    }];
    next.source_dispositions = [{
      source_span_id: "span-1",
      disposition: "omit",
      narrative_unit_id: "unit",
      shot_ids: ["a"],
      reason: "压缩节奏",
    }];
    const issues = collectVisualPlanIssues(next, ["span-1", "span-2"]);
    expect(issues).toContain("关键帧 A start 的实体 a 位置、尺寸或透明度越界");
    expect(issues).toContain("原文片段未处理：span-2");
    expect(issues).toContain("span-1 已省略但仍保留叙事单元或镜头引用");
  });

  it("reports camera centers and focus targets outside the current keyframe", () => {
    const next = structuredClone(plan);
    const camera = next.narrative_units[0].shots[0].keyframes[0].camera;
    camera.center_x = 1.1;
    camera.focus_entity_id = "a";
    const issues = collectVisualPlanIssues(next);
    expect(issues).toContain("关键帧 A start 的相机中心必须位于画面内");
    expect(issues).toContain("关键帧 A start 的焦点实体必须定义当前状态");
  });

  it("requires complete and homogeneous intercut groups", () => {
    const missingGroup = planWithTwoUnits();
    missingGroup.narrative_units[0].editing_mode = "intercut";
    expect(collectVisualPlanIssues(missingGroup)).toContain("叙事单元 单元 必须设置交叉剪辑组");

    const singleMember = planWithTwoUnits();
    singleMember.narrative_units[0].editing_mode = "intercut";
    singleMember.narrative_units[0].intercut_group_id = "group";
    expect(collectVisualPlanIssues(singleMember)).toContain(
      "交叉剪辑组 group 至少需要两个叙事单元",
    );

    const mixedModes = planWithTwoUnits();
    mixedModes.narrative_units[0].editing_mode = "intercut";
    mixedModes.narrative_units[0].intercut_group_id = "group";
    mixedModes.narrative_units[1].editing_mode = "parallel_action";
    mixedModes.narrative_units[1].intercut_group_id = "group";
    expect(collectVisualPlanIssues(mixedModes)).toContain(
      "交叉剪辑组 group 内的剪辑模式必须一致",
    );
  });

  it("requires valid and non-duplicated provenance for every event", () => {
    const next = structuredClone(plan);
    const shot = next.narrative_units[0].shots[0];
    shot.camera_events.push({
      id: "camera",
      kind: "push_in",
      start_ms: 100,
      end_ms: 300,
      target_entity_id: null,
      description: "推近",
      source_span_ids: [],
      user_addition_ids: [],
      parameters: {},
    });
    shot.technique_events.push({
      id: "technique",
      kind: "symbolic",
      start_ms: 300,
      end_ms: 500,
      description: "象征",
      source_span_ids: [],
      user_addition_ids: ["addition", "addition"],
      parameters: {},
    });
    const issues = collectVisualPlanIssues(next);
    expect(issues).toContain("A 的时序事件必须引用原文来源或用户新增事实");
    expect(issues).toContain("A 的时序事件用户新增事实重复引用：addition");
  });

  it("keeps event provenance inside its shot scope", () => {
    const next = structuredClone(plan);
    const unit = next.narrative_units[0];
    const shot = unit.shots[0];
    unit.source_span_ids = ["span-1"];
    shot.source_span_ids = ["span-1"];
    next.source_dispositions = [
      {
        source_span_id: "span-1",
        disposition: "adapt",
        narrative_unit_id: unit.id,
        shot_ids: [shot.id],
        reason: null,
      },
      {
        source_span_id: "span-2",
        disposition: "adapt",
        narrative_unit_id: unit.id,
        shot_ids: [unit.shots[1].id],
        reason: null,
      },
    ];
    shot.camera_events.push({
      id: "outside-scope",
      kind: "push_in",
      start_ms: 100,
      end_ms: 300,
      target_entity_id: null,
      description: "越界来源",
      source_span_ids: ["span-2"],
      user_addition_ids: ["addition"],
      parameters: {},
    });

    expect(collectVisualPlanIssueRecords(next)).toContainEqual(
      expect.objectContaining({ key: "visualPlan.validation.sourceOutsideScope" }),
    );
  });

  it("detaches an omitted source span from all four event tracks", () => {
    const next = structuredClone(plan);
    const unit = next.narrative_units[0];
    const shot = unit.shots[0];
    next.entities[0].source_span_ids = ["span-1"];
    unit.source_span_ids = ["span-1"];
    shot.source_span_ids = ["span-1"];
    next.source_dispositions = [{
      source_span_id: "span-1",
      disposition: "adapt",
      narrative_unit_id: unit.id,
      shot_ids: [shot.id],
      reason: null,
    }];
    shot.entity_events = [{
      id: "entity-event",
      kind: "action",
      start_ms: 100,
      end_ms: 200,
      entity_id: "a",
      target_entity_id: null,
      description: "动作",
      source_span_ids: ["span-1"],
      user_addition_ids: ["addition"],
      parameters: {},
    }];
    shot.camera_events = [{
      id: "camera-event",
      kind: "push_in",
      start_ms: 200,
      end_ms: 300,
      target_entity_id: null,
      description: "推近",
      source_span_ids: ["span-1"],
      user_addition_ids: [],
      parameters: {},
    }];
    shot.sound_events = [{
      id: "sound-event",
      kind: "effect",
      start_ms: 300,
      end_ms: 400,
      speaker_entity_id: null,
      content: "音效",
      source_span_ids: ["span-1"],
      user_addition_ids: ["addition"],
      parameters: {},
    }];
    shot.technique_events = [{
      id: "technique-event",
      kind: "symbolic",
      start_ms: 400,
      end_ms: 500,
      description: "象征",
      source_span_ids: ["span-1"],
      user_addition_ids: [],
      parameters: {},
    }];

    detachVisualPlanSourceSpanReferences(next, "span-1");
    next.source_dispositions[0] = {
      source_span_id: "span-1",
      disposition: "omit",
      narrative_unit_id: null,
      shot_ids: [],
      reason: "压缩节奏",
    };

    expect(unit.source_span_ids).toEqual([]);
    expect(next.entities[0].source_span_ids).toEqual([]);
    expect(shot.source_span_ids).toEqual([]);
    expect(shot.entity_events[0]?.source_span_ids).toEqual([]);
    expect(shot.sound_events[0]?.source_span_ids).toEqual([]);
    expect(shot.camera_events).toEqual([]);
    expect(shot.technique_events).toEqual([]);
    expect(collectVisualPlanIssues(next, ["span-1"])).toEqual([]);
  });

  it("rejects empty and duplicated reference ids", () => {
    const next = structuredClone(plan);
    next.entities[0].source_span_ids = [""];
    next.narrative_units[0].shots[0].user_addition_ids = ["addition", "addition"];
    const issues = collectVisualPlanIssues(next);
    expect(issues).toContain("实体 a原文来源不能包含空 ID");
    expect(issues).toContain("镜头 A用户新增事实重复引用：addition");
  });

  it("requires user additions to precede the timeline end", () => {
    const next = structuredClone(plan);
    next.user_additions[0].introduced_at_ms = next.frame.duration_ms;
    expect(collectVisualPlanIssues(next)).toContain(
      "用户新增事实 addition 的出现时间必须早于画面结束",
    );
  });

  it("rejects invalid source disposition ids and duplicate shot bindings", () => {
    const next = structuredClone(plan);
    next.source_dispositions = [{
      source_span_id: "span-1",
      disposition: "adapt",
      narrative_unit_id: "unit",
      shot_ids: ["a", "a"],
      reason: null,
    }];
    next.narrative_units[0].source_span_ids = ["span-1"];
    next.narrative_units[0].shots[0].source_span_ids = ["span-1"];
    const issues = collectVisualPlanIssues(next, ["span-1"]);
    expect(issues).toContain("span-1镜头重复引用：a");

    next.source_dispositions[0].source_span_id = "";
    expect(collectVisualPlanIssues(next, ["span-1"])).toContain(
      "原文处理项缺少原文片段 ID",
    );
  });

  it("reports structures that lose all provenance when a source span is omitted", () => {
    const next = structuredClone(plan);
    next.entities.forEach((entity) => { entity.source_span_ids = []; entity.user_addition_ids = []; });
    next.narrative_units[0].source_span_ids = ["span-1"];
    next.narrative_units[0].user_addition_ids = [];
    next.narrative_units[0].shots.forEach((shot) => {
      shot.source_span_ids = ["span-1"];
      shot.user_addition_ids = [];
    });
    next.source_dispositions = [{
      source_span_id: "span-1",
      disposition: "adapt",
      narrative_unit_id: "unit",
      shot_ids: ["a", "b"],
      reason: null,
    }];
    detachVisualPlanSourceSpanReferences(next, "span-1");
    next.source_dispositions[0] = {
      source_span_id: "span-1",
      disposition: "omit",
      narrative_unit_id: null,
      shot_ids: [],
      reason: "压缩节奏",
    };
    const issues = collectVisualPlanIssues(next, ["span-1"]);
    expect(issues.filter((issue) => issue.includes("必须引用原文来源或用户新增事实"))).toHaveLength(5);
  });

  it("does not allow entities to keep omitted source spans", () => {
    const next = structuredClone(plan);
    next.entities[0].source_span_ids = ["span-1"];
    next.source_dispositions = [{
      source_span_id: "span-1",
      disposition: "omit",
      narrative_unit_id: null,
      shot_ids: [],
      reason: "压缩节奏",
    }];
    expect(collectVisualPlanIssues(next, ["span-1"])).toContain(
      "实体 a原文来源引用了不存在的 ID：span-1",
    );
  });
});
