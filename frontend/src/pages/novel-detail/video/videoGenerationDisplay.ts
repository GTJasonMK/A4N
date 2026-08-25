import type { ChapterVideoPrompt } from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import type { MessageTranslator } from "../../../i18n";

export const promptTitle = (
  prompt: ChapterVideoPrompt,
  t: MessageTranslator,
): string => {
  const title = prompt.title?.trim();
  return title ? title : t("video.chapter.shot", { number: prompt.shot_number });
};

const labelForKey = (key: string, t: MessageTranslator): string => {
  const keys: Record<string, Parameters<MessageTranslator>[0]> = {
    start: "video.chapter.director.start",
    movement: "video.chapter.director.movement",
    end: "video.chapter.director.end",
    spatial_anchors: "video.chapter.director.spatialAnchors",
    video_intent: "video.chapter.director.videoIntent",
    hook: "video.chapter.director.hook",
    audience_effect: "video.chapter.director.audienceEffect",
    rhythm: "video.chapter.director.rhythm",
    visual_priority: "video.chapter.director.visualPriority",
    emotional_arc: "video.chapter.director.emotionalArc",
    turn: "video.chapter.director.turn",
    function: "video.chapter.director.function",
    material_coverage: "video.chapter.director.materialCoverage",
    required_entities: "video.chapter.director.requiredEntities",
    missing_or_weak_references: "video.chapter.director.missingReferences",
    risk_note: "video.chapter.director.riskNote",
    do_not_change: "video.chapter.director.doNotChange",
    key_light: "video.chapter.director.keyLight",
    direction: "video.chapter.director.direction",
    shadow: "video.chapter.director.shadow",
    continuity: "video.chapter.director.continuity",
    character: "video.chapter.director.character",
    emotion: "video.chapter.director.emotion",
    visible_action: "video.chapter.director.visibleAction",
    timing: "video.chapter.director.timing",
    shot_size: "video.chapter.director.shotSize",
    camera_angle: "video.chapter.director.cameraAngle",
    camera_movement: "video.chapter.director.cameraMovement",
    axis_rule: "video.chapter.director.axisRule",
    lens_hint: "video.chapter.director.lensHint",
    start_sec: "video.chapter.director.startSecond",
    end_sec: "video.chapter.director.endSecond",
    camera: "video.chapter.director.camera",
    subject_action: "video.chapter.director.subjectAction",
    continuity_target: "video.chapter.director.continuityTarget",
    image_id: "video.chapter.director.image",
    entity: "video.chapter.director.entity",
    role: "video.chapter.director.role",
    usage: "video.chapter.director.usage",
    must_keep: "video.chapter.director.mustKeep",
    coverage_check: "video.chapter.director.coverageCheck",
    blocking: "video.chapter.director.blocking",
    lighting: "video.chapter.director.lighting",
    references: "video.chapter.director.references",
    revision: "video.chapter.director.revision",
    speaker: "video.chapter.director.speaker",
    line: "video.chapter.director.line",
    delivery: "video.chapter.director.delivery",
    listener_reaction: "video.chapter.director.listenerReaction",
    sound_design: "video.chapter.director.soundDesign",
    ambience: "video.chapter.director.ambience",
    action_sounds: "video.chapter.director.actionSounds",
    dialogue_delivery: "video.chapter.director.dialogueDelivery",
    silence_or_pause: "video.chapter.director.silenceOrPause",
  };
  return keys[key] ? t(keys[key]) : key;
};

const metadataValueText = (value: unknown, t: MessageTranslator): string => {
  if (value == null) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    return value.map((item) => metadataValueText(item, t)).filter(Boolean).join(t("common.listSeparator"));
  }
  const record = asRecord(value);
  if (!record) return "";
  return Object.entries(record)
    .map(([key, item]) => {
      const text = metadataValueText(item, t);
      return text ? `${labelForKey(key, t)}: ${text}` : "";
    })
    .filter(Boolean)
    .join(t("common.summarySeparator"));
};

const metadataFieldText = (record: Record<string, unknown>, key: string, t: MessageTranslator): string => {
  return metadataValueText(record[key], t);
};

type ClauseSpec = string;

const clausesText = (record: Record<string, unknown>, clauses: ClauseSpec[], t: MessageTranslator): string => {
  return clauses
    .map((key) => {
      const text = metadataFieldText(record, key, t);
      const displayLabel = labelForKey(key, t);
      return text ? `${displayLabel}: ${text}` : "";
    })
    .filter(Boolean)
    .join(t("common.summarySeparator"));
};

const clauseListText = (
  value: unknown,
  clauses: ClauseSpec[],
  prefix: (record: Record<string, unknown>, index: number) => string,
  t: MessageTranslator,
): string => {
  if (!Array.isArray(value)) return "";
  return value
    .map((item, index) => {
      const itemRecord = asRecord(item);
      if (!itemRecord) return metadataValueText(item, t);
      const text = clausesText(itemRecord, clauses, t);
      return text ? `${prefix(itemRecord, index)}${text}` : "";
    })
    .filter(Boolean)
    .join("\n");
};

const sectionText = (key: string, value: unknown, t: MessageTranslator): string => {
  const record = asRecord(value);
  if (key === "blocking_plan" && record) {
    return clausesText(record, [
      "start",
      "movement",
      "end",
      "spatial_anchors",
      "do_not_change",
    ], t);
  }
  if (key === "lighting_plan" && record) {
    return clausesText(record, [
      "key_light",
      "direction",
      "shadow",
      "continuity",
    ], t);
  }
  if (key === "cinematography_plan" && record) {
    return clausesText(record, [
      "shot_size",
      "camera_angle",
      "camera_movement",
      "axis_rule",
      "lens_hint",
    ], t);
  }
  if (key === "continuity_check" && record) {
    return clausesText(record, [
      "blocking",
      "lighting",
      "emotion",
      "references",
      "revision",
    ], t);
  }
  if (key === "emotion_action_beats" && Array.isArray(value)) {
    return clauseListText(
      value,
      [
        "character",
        "emotion",
        "visible_action",
        "timing",
      ],
      (_itemRecord, index) => `${index + 1}. `,
      t,
    );
  }
  if (key === "dialogue_beats" && Array.isArray(value)) {
    return clauseListText(
      value,
      [
        "speaker",
        "line",
        "delivery",
        "visible_action",
        "listener_reaction",
        "timing",
      ],
      (_itemRecord, index) => `${index + 1}. `,
      t,
    );
  }
  if (key === "temporal_beats" && Array.isArray(value)) {
    return clauseListText(
      value,
      [
        "camera",
        "subject_action",
        "continuity_target",
      ],
      (itemRecord, index) => {
        const start = metadataFieldText(itemRecord, "start_sec", t);
        const end = metadataFieldText(itemRecord, "end_sec", t);
        return start && end
          ? t("video.chapter.director.secondsColon", { value: `${start}-${end}` })
          : t("video.chapter.director.timeSegment", { number: index + 1 });
      },
      t,
    );
  }
  if (key === "reference_binding" && record) {
    const bindings = Array.isArray(record.bindings) ? record.bindings : [];
    const lines = bindings
      .map((item) => {
        const binding = asRecord(item);
        if (!binding) return metadataValueText(item, t);
        const id = metadataFieldText(binding, "image_id", t);
        const text = clausesText(binding, [
          "entity",
          "usage",
          "must_keep",
          "role",
        ], t);
        return `${id ? t("video.chapter.director.referenceNumber", { id }) : t("video.chapter.director.reference")}${text ? `: ${text}` : ""}`;
      })
      .filter(Boolean);
    const coverage = metadataFieldText(record, "coverage_check", t);
    if (coverage) lines.push(`${t("video.chapter.director.coverage")}: ${coverage}`);
    return lines.join("\n");
  }
  return metadataValueText(value, t);
};

const directorSectionValue = (metadata: unknown, key: string): unknown => {
  const root = asRecord(metadata);
  const shot = asRecord(root?.shot);
  const segment = asRecord(root?.segment);
  const chatVideo = asRecord(root?.chat_video);
  return root?.[key] ?? shot?.[key] ?? segment?.[key] ?? chatVideo?.[key] ?? null;
};

export const directorSections = (
  metadata: unknown,
  t: MessageTranslator,
): Array<{ key: string; label: string; text: string }> => {
  const sections = [
    "video_intent",
    "emotional_arc",
    "material_coverage",
    "self_contained_context",
    "start_state",
    "transition_in",
    "end_state",
    "blocking_plan",
    "lighting_plan",
    "emotion_action_beats",
    "dialogue_beats",
    "cinematography_plan",
    "temporal_beats",
    "reference_binding",
    "sound_design",
    "continuity_check",
  ] as const;
  const translatedLabels: Record<string, Parameters<MessageTranslator>[0]> = {
    video_intent: "video.chapter.director.videoIntent",
    emotional_arc: "video.chapter.director.emotionalArc",
    material_coverage: "video.chapter.director.materialCoverage",
    self_contained_context: "video.chapter.director.selfContainedContext",
    start_state: "video.chapter.director.startState",
    transition_in: "video.chapter.director.transitionIn",
    end_state: "video.chapter.director.endState",
    blocking_plan: "video.chapter.director.blockingPlan",
    lighting_plan: "video.chapter.director.lightingPlan",
    emotion_action_beats: "video.chapter.director.emotionActionBeats",
    dialogue_beats: "video.chapter.director.dialogueBeats",
    cinematography_plan: "video.chapter.director.cinematographyPlan",
    temporal_beats: "video.chapter.director.temporalBeats",
    reference_binding: "video.chapter.director.referenceBinding",
    sound_design: "video.chapter.director.soundDesign",
    continuity_check: "video.chapter.director.continuityCheck",
  };
  return sections
    .map((key) => ({
      key,
      label: t(translatedLabels[key]),
      text: sectionText(key, directorSectionValue(metadata, key), t),
    }))
    .filter((section) => section.text.length > 0);
};
