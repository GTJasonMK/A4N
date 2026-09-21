import type {
  GalgameAssetPlanItem,
  GalgameLine,
  GalgameLinePresentationDocumentV2,
  GalgamePipelineStartStage,
  GalgameScene,
  GalgameSpeakerProfile,
  GalgameStagePositionV2,
  GalgameVisibleCastMemberV2,
} from "../../../api/galgame";
import type { MessageKey, MessageTranslator } from "../../../i18n";

export interface PortraitExpressionCell {
  rowKey: string;
  profileId: number;
  profileName: string;
  pose: string;
  expression: string;
  item: GalgameAssetPlanItem;
}

export interface PortraitExpressionMatrixRow {
  rowKey: string;
  firstCell: PortraitExpressionCell;
}

export const LINE_TYPES: Array<{ value: GalgameLineType; labelKey: MessageKey }> = [
  { value: "narration", labelKey: "galgame.lineType.narration" },
  { value: "dialogue", labelKey: "galgame.lineType.dialogue" },
  { value: "monologue", labelKey: "galgame.lineType.monologue" },
  { value: "stage_direction", labelKey: "galgame.lineType.stageDirection" },
  { value: "choice", labelKey: "galgame.lineType.choice" },
];

export const SPEAKING_LINE_TYPES = new Set(["dialogue", "monologue"]);

export const POSITIONS: Array<{
  value: GalgameStagePositionV2 | "";
  labelKey: MessageKey;
}> = [
  { value: "", labelKey: "galgame.position.none" },
  { value: "left", labelKey: "galgame.position.left" },
  { value: "center", labelKey: "galgame.position.center" },
  { value: "right", labelKey: "galgame.position.right" },
];

export const EXPRESSIONS = [
  { value: "", labelKey: "galgame.expression.none" as MessageKey },
  { value: "neutral", labelKey: "galgame.expression.neutral" as MessageKey },
  { value: "smile", labelKey: "galgame.expression.smile" as MessageKey },
  { value: "sad", labelKey: "galgame.expression.sad" as MessageKey },
  { value: "angry", labelKey: "galgame.expression.angry" as MessageKey },
  { value: "surprised", labelKey: "galgame.expression.surprised" as MessageKey },
  { value: "shy", labelKey: "galgame.expression.shy" as MessageKey },
  { value: "serious", labelKey: "galgame.expression.serious" as MessageKey },
];

export const POSE_SUGGESTIONS = [
  { value: "natural", labelKey: "galgame.pose.natural" as MessageKey },
  { value: "standing", labelKey: "galgame.pose.standing" as MessageKey },
  { value: "sitting", labelKey: "galgame.pose.sitting" as MessageKey },
  { value: "walking", labelKey: "galgame.pose.walking" as MessageKey },
  { value: "turning", labelKey: "galgame.pose.turning" as MessageKey },
  { value: "close_up", labelKey: "galgame.pose.closeUp" as MessageKey },
];

export const CG_IMPORTANCE_OPTIONS: Array<{
  value: GalgameCgImportance;
  labelKey: MessageKey;
}> = [
  { value: "normal", labelKey: "galgame.cgImportance.normal" },
  { value: "high", labelKey: "galgame.cgImportance.high" },
  { value: "critical", labelKey: "galgame.cgImportance.critical" },
];

export const TRANSITION_OPTIONS = [
  { value: "", labelKey: "galgame.transition.none" as MessageKey },
  { value: "cut", labelKey: "galgame.transition.cut" as MessageKey },
  { value: "fade", labelKey: "galgame.transition.fade" as MessageKey },
  { value: "dissolve", labelKey: "galgame.transition.dissolve" as MessageKey },
];

export const PIPELINE_START_STAGES: Array<{
  value: GalgamePipelineStartStage | "";
  labelKey: MessageKey;
}> = [
  { value: "", labelKey: "galgame.startStage.all" },
  { value: "shooting_script_projection", labelKey: "galgame.startStage.shootingScript" },
  { value: "script_adaptation", labelKey: "galgame.startStage.adaptation" },
  { value: "character_staging", labelKey: "galgame.startStage.staging" },
  { value: "visual_planning", labelKey: "galgame.startStage.planning" },
  { value: "branch_design", labelKey: "galgame.startStage.branch" },
  { value: "persist_script", labelKey: "galgame.startStage.persist" },
  { value: "asset_generation", labelKey: "galgame.startStage.asset" },
];

export const lineBusyTarget = (lineId: number): string => `line:${lineId}`;
export const sceneBusyTarget = (sceneId: number): string => `scene:${sceneId}`;
export const audioBusyTarget = (lineId: number, role: "bgm" | "sfx"): string =>
  `audio:${lineId}:${role}`;
export const bindMediaBusyTarget = (
  targetKind: "galgame_scene" | "galgame_line",
  targetId: number,
  role: "background" | "cg" | "bgm" | "sfx",
): string => `bind-media:${targetKind}:${targetId}:${role}`;
export const insertBusyTarget = (
  sceneId: number,
  afterLineId: number | null,
): string => `insert:${sceneId}:${afterLineId ?? "start"}`;


export function pipelineStartStageLabel(
  stage: GalgamePipelineStartStage | "",
  t: MessageTranslator,
): string {
  return (
    PIPELINE_START_STAGES.find((item) => item.value === stage)
      ? t(PIPELINE_START_STAGES.find((item) => item.value === stage)!.labelKey)
      : t("galgame.startStage.all")
  );
}

export function sceneEntryKindLabel(entryKind: string, t: MessageTranslator): string {
  switch (entryKind) {
    case "main":
      return t("galgame.sceneEntry.main");
    case "branch":
      return t("galgame.sceneEntry.branch");
    default:
      return t("galgame.sceneEntry.other");
  }
}

export function sceneSelectLabel(scene: GalgameScene, t: MessageTranslator): string {
  const entryKind = scene.entry_kind === "branch"
    ? t("galgame.sceneEntry.branch")
    : t("galgame.sceneEntry.main");
  return `${t("galgame.scenes")} ${scene.scene_index} · ${scene.title} · ${entryKind}`;
}

export function assetPlanOutcomeLabel(outcome: string, t: MessageTranslator): string {
  switch (outcome) {
    case "created":
      return t("galgame.assetOutcome.created");
    case "updated":
      return t("galgame.assetOutcome.updated");
    case "reused":
      return t("galgame.assetOutcome.reused");
    default:
      return t("galgame.assetOutcome.other");
  }
}

export function assetSpecStatusLabel(status: string, t: MessageTranslator): string {
  switch (status) {
    case "draft":
      return t("galgame.assetStatus.draft");
    case "completed":
      return t("galgame.assetStatus.completed");
    case "failed":
      return t("galgame.assetStatus.failed");
    case "running":
      return t("galgame.assetStatus.running");
    default:
      throw new Error(`Galgame 资产规划状态无效: ${status}`);
  }
}

export function buildPortraitExpressionCells(
  items: GalgameAssetPlanItem[],
  speakerProfiles: GalgameSpeakerProfile[],
): PortraitExpressionCell[] {
  const speakerNameById = new Map(
    speakerProfiles.map((profile) => [profile.id, profile.name]),
  );
  return items.map((item) => {
    const params = requireRecordValue(
      item.params_json,
      `Galgame 资产规划 ${item.generation_spec_id} params_json`,
    );
    const profileId = item.target_entity_id;
    const profileName = speakerNameById.get(profileId);
    if (!profileName) {
      throw new Error(`Galgame 资产规划引用不存在的角色 ${profileId}`);
    }
    const expression = requireStringValue(
      params.expression,
      `Galgame 资产规划 ${item.generation_spec_id} expression`,
    );
    const pose = requireStringValue(
      params.pose,
      `Galgame 资产规划 ${item.generation_spec_id} pose`,
    );
    return {
      rowKey: `${profileId}:${pose}`,
      profileId,
      profileName,
      pose,
      expression,
      item,
    };
  });
}

export function buildPortraitExpressionMatrixModel(cells: PortraitExpressionCell[]): {
  rows: PortraitExpressionMatrixRow[];
  expressionValues: string[];
  cellsByKey: Map<string, PortraitExpressionCell>;
} {
  const rows: PortraitExpressionMatrixRow[] = [];
  const seenRowKeys = new Set<string>();
  const presentExpressions = new Set<string>();
  const cellsByKey = new Map<string, PortraitExpressionCell>();
  for (const cell of cells) {
    if (!seenRowKeys.has(cell.rowKey)) {
      seenRowKeys.add(cell.rowKey);
      rows.push({ rowKey: cell.rowKey, firstCell: cell });
    }
    presentExpressions.add(cell.expression);
    cellsByKey.set(`${cell.rowKey}:${cell.expression}`, cell);
  }
  return {
    rows,
    expressionValues: portraitExpressionColumns(presentExpressions),
    cellsByKey,
  };
}

export function portraitExpressionColumns(present: Set<string>): string[] {
  const known = EXPRESSIONS.map((item) => item.value);
  const columns = known.filter((value) => present.has(value));
  for (const value of present) {
    if (!columns.includes(value)) columns.push(value);
  }
  return columns;
}

export function portraitExpressionLabel(
  expression: string,
  t: MessageTranslator,
): string {
  const option = EXPRESSIONS.find((item) => item.value === expression);
  if (!option) throw new Error(`Galgame 立绘表情无效: ${expression}`);
  return t(option.labelKey);
}

export function poseDisplayValue(pose: string, t: MessageTranslator): string {
  const option = POSE_SUGGESTIONS.find((item) => item.value === pose);
  return option ? t(option.labelKey) : pose;
}

export function poseStoredValue(pose: string, t: MessageTranslator): string {
  const normalized = pose.trim();
  return (
    POSE_SUGGESTIONS.find((item) => t(item.labelKey) === normalized)?.value ??
    normalized
  );
}

export function requireRecordValue(
  value: unknown,
  fieldName: string,
): Record<string, unknown> {
  if (value != null && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  throw new Error(`${fieldName} 必须是 JSON 对象`);
}

export function requireStringValue(value: unknown, fieldName: string): string {
  if (typeof value === "string" && value.trim()) return value.trim();
  throw new Error(`${fieldName} 必须是非空字符串`);
}

export function emptyGalgamePresentation(): GalgameLinePresentationDocumentV2 {
  return {
    after_beat_ref: null,
    branch: null,
    branch_id: null,
    cg: null,
    cg_importance: null,
    cg_note: null,
    schema_version: "galgame_line_presentation_v2",
    visible_cast: [],
    enter: [],
    exit: [],
    focus_character_profile_id: null,
    hold: false,
    player_knowledge: [],
    returns_to_scene_index: null,
    transition: null,
  };
}

export function presentationForLinePatch(
  line: GalgameLine,
  patch: Partial<GalgameLine>,
): GalgameLinePresentationDocumentV2 {
  const direct = patch.presentation_json ?? line.presentation_json;
  const presentation: GalgameLinePresentationDocumentV2 = {
    ...direct,
    visible_cast: direct.visible_cast.map((member) => ({ ...member })),
    enter: [...direct.enter],
    exit: [...direct.exit],
    player_knowledge: [...direct.player_knowledge],
  };
  const nextType = patch.line_type ?? line.line_type;
  if (!SPEAKING_LINE_TYPES.has(nextType)) return presentation;
  const speakerProfileId =
    "speaker_profile_id" in patch
      ? (patch.speaker_profile_id ?? null)
      : line.speaker_profile_id;
  if (speakerProfileId == null)
    throw new Error(`${nextType} 行必须绑定 speaker_profile_id`);
  const existing = presentation.visible_cast.find(
    (member) => member.character_profile_id === speakerProfileId,
  );
  const expression =
    ("expression" in patch
      ? patch.expression
      : (existing?.expression ?? line.expression)) || "neutral";
  const pose =
    ("pose" in patch ? patch.pose : (existing?.pose ?? line.pose)) ||
    "standing";
  const position = (("position" in patch
    ? patch.position
    : (existing?.position ?? line.position)) ||
    "center") as GalgameStagePositionV2;
  const speaker: GalgameVisibleCastMemberV2 = {
    character_profile_id: speakerProfileId,
    expression,
    pose,
    position,
    facing: existing?.facing ?? "front",
    layer: existing?.layer ?? "front",
  };
  presentation.visible_cast = [
    ...presentation.visible_cast.filter(
      (member) => member.character_profile_id !== speakerProfileId,
    ),
    speaker,
  ];
  if (presentation.focus_character_profile_id == null) {
    presentation.focus_character_profile_id = speakerProfileId;
  }
  return presentation;
}

export function stringValue(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

export function selectValue(
  value: unknown,
  options: Array<{ value: string }>,
): string {
  const normalized = stringValue(value);
  return options.some((item) => item.value === normalized) ? normalized : "";
}
