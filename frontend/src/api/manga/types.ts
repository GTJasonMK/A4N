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
  characters_in_frame: unknown | null;
  layout: unknown | null;
  camera_angle: string | null;
  dialogue_position: string | null;
  generated_image_id: number | null;
  status: string;
  created_at: string | null;
  updated_at: string | null;
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
  | "event_beats"
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
