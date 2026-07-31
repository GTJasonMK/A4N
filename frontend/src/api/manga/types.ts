import type { DownstreamConflictStrategy } from "../dependencyConflict";
export type { DownstreamConflictStrategy } from "../dependencyConflict";

export interface MangaPanel {
  id: number;
  chapter_id: number;
  project_id: number;
  page_number: number;
  panel_number: number;
  prompt_type: string;
  image_prompt: string;
  dialogue_text: string | null;
  shooting_script_id: number;
  shooting_script_revision: number;
  page_plan: MangaPagePlan;
  drawing_prompt: MangaPageDrawingPrompt;
  reference_bindings: MangaReferenceBinding[];
  source: MangaPromptSource;
  generated_image_id: number | null;
  status: string;
  created_at: string | null;
  updated_at: string | null;
}

interface MangaPromptSource {
  root_project_id: number;
  source_chapter_id: number;
  source_snapshot_id: number | null;
  source_snapshot_chapter_id: number | null;
  source_revision_key: string;
  content_sha256: string;
  shooting_script_id: number;
  shooting_script_revision: number;
  shooting_script_sha256: string;
}

export type MangaReferenceAssetKind = "character_portrait" | "generated_image";

export interface MangaReferenceBinding {
  slot_id: string;
  target_type: string;
  target_name: string | null;
  label: string;
  purpose: string;
  required: boolean;
  asset_kind: MangaReferenceAssetKind;
  status: string;
  bound_asset: MangaBoundReferenceAsset | null;
}

export interface MangaBoundReferenceAsset {
  asset_kind: MangaReferenceAssetKind;
  asset_id: number;
  label: string;
  image_path: string;
  [key: string]: unknown;
}

interface MangaShootingBeatRef {
  scene_index: number;
  shot_index: number;
  beat_index: number;
}

type MangaTextLayerKind = "dialogue" | "thought" | "narration" | "sfx";

interface MangaTextLayer {
  layer_index: number;
  kind: MangaTextLayerKind;
  speaker: string | null;
  text: string;
  placement:
    | "top_left"
    | "top_right"
    | "upper_center"
    | "center_left"
    | "center_right"
    | "bottom_left"
    | "bottom_right"
    | "lower_center";
  avoid_covering: string[];
}

interface MangaPanelPlan {
  panel_number: number;
  beat_refs: MangaShootingBeatRef[];
  function:
    | "establish"
    | "action"
    | "dialogue"
    | "reaction"
    | "revelation"
    | "transition"
    | "hold"
    | "resolution"
    | "detail";
  size: "dominant" | "large" | "medium" | "small" | "inset";
  shot_scale: "extreme_wide" | "wide" | "medium" | "close_up" | "extreme_close_up" | "detail";
  camera_angle: "eye_level" | "high" | "low" | "top_down" | "dutch" | "over_shoulder" | "profile";
  participants: string[];
  composition: string;
  observable_action: string;
  reaction: string | null;
  spatial_relationship: string;
  continuity_from_previous: string | null;
  text_layers: MangaTextLayer[];
}

interface MangaPagePlan {
  page_number: number;
  beat_refs: MangaShootingBeatRef[];
  role: "opening" | "setup" | "rising" | "climax" | "falling" | "resolution" | "transition";
  pacing: "slow" | "medium" | "fast" | "explosive";
  importance: "critical" | "high" | "normal" | "low";
  reading_flow: "top_to_bottom_left_to_right" | "z_path" | "s_path" | "diagonal";
  page_goal: string;
  visual_thesis: string;
  entry_state: string;
  exit_state: string;
  dominant_panel_number: number;
  panels: MangaPanelPlan[];
}

interface MangaPageDrawingPrompt {
  schema_version: "manga_page_drawing_prompt_v2";
  page_number: number;
  image_prompt: string;
}

export interface PipelineStatus {
  chapter_id: number;
  pending: number;
  generating: number;
  completed: number;
  failed: number;
  cancelled: number;
  pipeline_status: string | null;
  stage: string | null;
  current: number | null;
  total: number | null;
  message: string | null;
  task_id: string | null;
  can_cancel: boolean;
  can_resume: boolean;
  has_checkpoint: boolean;
  checkpoint_valid: boolean;
}

export type MangaPipelineStartStage =
  | "character_binding"
  | "speech_selection"
  | "visual_anchor_extraction"
  | "item_extraction"
  | "comic_synthesis"
  | "page_planning"
  | "drawing_prompt";

export interface BindMangaReferenceImageInput {
  prompt_id: number;
  slot_id: string;
  asset_kind: "character_portrait" | "generated_image";
  asset_id: number;
  downstream_strategy?: DownstreamConflictStrategy;
}
