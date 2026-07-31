import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import { idNum, type ApiId as Id } from "./shared";
import type { MediaAssetRecord } from "./canvas";
import type { ExportFileData } from "./export";
import type { ModelRequestPreview } from "./requestPreview";
import type { GalgamePipelineStatusSummary } from "./generated/host";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问 Galgame 项目");
  return id;
};

export type GalgameLineType =
  | "dialogue"
  | "narration"
  | "monologue"
  | "stage_direction"
  | "choice";

export type GalgamePipelineStartStage =
  | "shooting_script_projection"
  | "script_adaptation"
  | "character_staging"
  | "visual_planning"
  | "branch_design"
  | "persist_script"
  | "asset_generation";

interface GalgameScript {
  id: number;
  project_id: number;
  root_project_id: number;
  source_snapshot_id: number;
  title: string;
  style_config_json: unknown;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface GalgameScene {
  id: number;
  script_id: number;
  source_snapshot_chapter_id: number;
  scene_index: number;
  title: string;
  location: string | null;
  time_of_day: string | null;
  participants_json: unknown;
  background_note: string | null;
  entry_kind: string;
  next_scene_id: number | null;
  created_at: string;
  updated_at: string;
}

export interface GalgameSpeakerProfile {
  id: number;
  name: string;
}

export interface GalgameTextAnchorSummary {
  id: number;
  source_snapshot_chapter_id: number | null;
  paragraph_index: number;
  start_offset: number;
  end_offset: number;
  quote: string;
}

export interface GalgameLine {
  id: number;
  scene_id: number;
  line_index: number;
  line_type: GalgameLineType | string;
  speaker_profile_id: number | null;
  text: string;
  expression: string | null;
  pose: string | null;
  position: string | null;
  presentation_json: unknown;
  text_anchor_id: number | null;
  created_at: string;
  updated_at: string;
}

export type GalgameStagePositionV2 = "left" | "center" | "right";
type GalgameFacingV2 = "left" | "right" | "front";
type GalgameStageLayerV2 = "back" | "middle" | "front";

export interface GalgameVisibleCastMemberV2 {
  character_profile_id: number;
  expression: string;
  pose: string;
  position: GalgameStagePositionV2;
  facing: GalgameFacingV2;
  layer: GalgameStageLayerV2;
}

interface GalgameBeatRefV2 {
  scene_index: number;
  beat_index: number;
}

export interface GalgameLinePresentationDocumentV2 {
  schema_version: "galgame_line_presentation_v2";
  visible_cast: GalgameVisibleCastMemberV2[];
  enter: number[];
  exit: number[];
  focus_character_profile_id: number | null;
  hold: boolean;
  player_knowledge: string[];
  cg?: boolean;
  cg_note?: string;
  cg_importance?: "normal" | "high" | "critical";
  transition?: "cut" | "fade" | "dissolve";
  branch?: boolean;
  branch_id?: string;
  returns_to_scene_index?: number;
  after_beat_ref?: GalgameBeatRefV2;
}

export interface GalgameChoice {
  id: number;
  line_id: number;
  option_index: number;
  option_text: string;
  target_scene_id: number | null;
  created_at: string;
}

export interface GalgameSceneDetail {
  scene: GalgameScene;
  lines: GalgameLine[];
  choices: GalgameChoice[];
}

interface GalgameMediaBinding {
  id: number;
  media_asset_id: number;
  target_kind: string;
  target_id: number;
  role: string;
  created_at: string;
}

export interface GalgameMediaBindingSummary {
  binding: GalgameMediaBinding;
  asset: MediaAssetRecord;
}

export interface GalgameScriptDetail {
  script: GalgameScript;
  speaker_profiles: GalgameSpeakerProfile[];
  text_anchors: GalgameTextAnchorSummary[];
  media_bindings: GalgameMediaBindingSummary[];
  scenes: GalgameSceneDetail[];
}

export type GalgamePipelineStatus = GalgamePipelineStatusSummary;

interface GalgameAssetPlanSummary {
  background_specs: number;
  portrait_expression_specs: number;
  event_cg_specs: number;
  created_specs: number;
  updated_specs: number;
  reused_specs: number;
}

type GalgameAssetPlanOutcome = "created" | "updated" | "reused";

export interface GalgameAssetPlanItem {
  canonical_key: string;
  label: string;
  target_kind: string;
  target_entity_kind: string;
  target_entity_id: number;
  generation_spec_id: number;
  status: string;
  outcome: GalgameAssetPlanOutcome;
  params_json: unknown;
}

export interface GalgameAssetPlan {
  summary: GalgameAssetPlanSummary;
  backgrounds: GalgameAssetPlanItem[];
  portrait_expressions: GalgameAssetPlanItem[];
  event_cgs: GalgameAssetPlanItem[];
}

export interface RunGalgameAssetGenerationInput {
  project_id: number;
  generation_spec_ids?: number[];
  config_id?: number | null;
  limit?: number | null;
}

interface GalgameAssetGenerationItem {
  generation_spec_id: number;
  target_kind: string;
  target_entity_kind: string;
  target_entity_id: number;
  status: string;
  prompt_artifact_id: number | null;
  generated_image_id: number | null;
  portrait_id: number | null;
  media_asset_id: number | null;
  binding_id: number | null;
}

export interface GalgameAssetGenerationSummary {
  project_id: number;
  generated_count: number;
  skipped_completed_count: number;
  items: GalgameAssetGenerationItem[];
}

export interface BindGalgameMediaInput {
  project_id: number;
  media_asset_id: number;
  target_kind: "galgame_scene" | "galgame_line" | string;
  target_id: number;
  role: "background" | "cg" | "bgm" | "sfx" | string;
}

export interface UnbindGalgameMediaInput {
  project_id: number;
  target_kind: "galgame_scene" | "galgame_line" | string;
  target_id: number;
  role: "background" | "cg" | "bgm" | "sfx" | string;
  media_asset_id?: number | null;
}

export interface ImportGalgameAudioInput {
  project_id: number;
  target_line_id: number;
  role: "bgm" | "sfx";
  file_name: string;
  mime_type: string;
  data_base64: string;
}

export interface GalgameMediaBindingResult {
  binding: GalgameMediaBinding;
  asset: MediaAssetRecord;
}

export interface UpdateGalgameLineInput {
  line_id: number;
  line_type?: GalgameLineType | string;
  speaker_profile_id?: number | null;
  text?: string;
  presentation?: GalgameLinePresentationDocumentV2;
  text_anchor_id?: number | null;
}

export interface UpdateGalgameSceneInput {
  scene_id: number;
  title?: string;
  location?: string | null;
  time_of_day?: string | null;
  background_note?: string | null;
  next_scene_id?: number | null;
}

export interface InsertGalgameLineInput {
  scene_id: number;
  after_line_id: number | null;
  line_type: GalgameLineType | string;
  speaker_profile_id: number | null;
  text: string;
  presentation: GalgameLinePresentationDocumentV2;
  text_anchor_id: number | null;
}

export interface UpdateGalgameChoiceOptionInput {
  option_text: string;
  target_scene_id: number;
}

export interface UpdateGalgameChoicesInput {
  line_id: number;
  choices: UpdateGalgameChoiceOptionInput[];
}

export async function getGalgameScript(projectId: Id): Promise<GalgameScriptDetail> {
  return apiInvoke<GalgameScriptDetail>("get_galgame_script", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) },
  });
}

export async function startGalgamePipeline(
  projectId: Id,
  options?: {
    taskId?: string | null;
    startFromStage?: GalgamePipelineStartStage | null;
    resumeFromCheckpoint?: boolean;
  },
): Promise<GalgameScriptDetail> {
  return apiInvoke<GalgameScriptDetail>("start_galgame_pipeline", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      task_id: options?.taskId ?? null,
      start_from_stage: options?.startFromStage ?? null,
      resume_from_checkpoint: options?.resumeFromCheckpoint ?? false,
    },
  });
}

export async function cancelGalgamePipeline(projectId: Id): Promise<GalgamePipelineStatus> {
  return apiInvoke<GalgamePipelineStatus>("cancel_galgame_pipeline", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) },
  });
}

export async function previewGalgamePipelineRequest(
  projectId: Id,
  options?: {
    startFromStage?: GalgamePipelineStartStage | null;
    resumeFromCheckpoint?: boolean;
  },
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_galgame_pipeline_request", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      start_from_stage: options?.startFromStage ?? null,
      resume_from_checkpoint: options?.resumeFromCheckpoint ?? false,
    },
  });
}

export async function getGalgamePipelineStatus(projectId: Id): Promise<GalgamePipelineStatus> {
  return apiInvoke<GalgamePipelineStatus>("get_galgame_pipeline_status", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) },
  });
}

export async function updateGalgameLine(
  projectId: Id,
  input: UpdateGalgameLineInput,
): Promise<GalgameLine> {
  return apiInvoke<GalgameLine>("update_galgame_line", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input,
  });
}

export async function insertGalgameLine(
  projectId: Id,
  input: InsertGalgameLineInput,
): Promise<GalgameLine> {
  return apiInvoke<GalgameLine>("insert_galgame_line", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input,
  });
}

export async function updateGalgameScene(
  projectId: Id,
  input: UpdateGalgameSceneInput,
): Promise<GalgameScene> {
  return apiInvoke<GalgameScene>("update_galgame_scene", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input,
  });
}

export async function deleteGalgameLine(projectId: Id, lineId: Id): Promise<void> {
  return apiInvoke<void>("delete_galgame_line", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input: { line_id: idNum(lineId) },
  });
}

export async function reorderGalgameLines(
  projectId: Id,
  sceneId: Id,
  lineIds: number[],
): Promise<GalgameLine[]> {
  return apiInvoke<GalgameLine[]>("reorder_galgame_lines", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input: { scene_id: idNum(sceneId), line_ids: lineIds },
  });
}

export async function updateGalgameChoices(
  projectId: Id,
  input: UpdateGalgameChoicesInput,
): Promise<GalgameChoice[]> {
  return apiInvoke<GalgameChoice[]>("update_galgame_choices", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input,
  });
}

export async function planGalgameAssets(projectId: Id): Promise<GalgameAssetPlan> {
  return apiInvoke<GalgameAssetPlan>("plan_galgame_assets", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) },
  });
}

export async function runGalgameAssetGeneration(
  projectId: Id,
  options: Omit<RunGalgameAssetGenerationInput, "project_id"> = {},
): Promise<GalgameAssetGenerationSummary> {
  return apiInvoke<GalgameAssetGenerationSummary>("run_galgame_asset_generation", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId), ...options },
  });
}

export async function bindGalgameMedia(
  input: BindGalgameMediaInput,
): Promise<GalgameMediaBindingResult> {
  return apiInvoke<GalgameMediaBindingResult>("bind_galgame_media", {
    userId: currentUserId(),
    input,
  });
}

export async function unbindGalgameMedia(input: UnbindGalgameMediaInput): Promise<number> {
  return apiInvoke<number>("unbind_galgame_media", {
    userId: currentUserId(),
    input,
  });
}

export async function importGalgameAudio(
  input: ImportGalgameAudioInput,
): Promise<GalgameMediaBindingResult> {
  return apiInvoke<GalgameMediaBindingResult>("import_galgame_audio", {
    userId: currentUserId(),
    input,
  });
}

export async function exportGalgameWebgal(projectId: Id): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>("export_galgame_webgal", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) },
  });
}
