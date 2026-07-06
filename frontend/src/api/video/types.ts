export type VideoDurationMode = "range" | "fixed";
export type PromptOutputLanguage = "zh_cn" | "en";
export type VideoWorkflowMode =
  | "boundary_frames"
  | "first_frame_chain"
  | "storyboard"
  | "reference_media"
  | "chat_video"
  | "canvas_video"
  | "canvas_reference_media"
  | "canvas_trim"
  | "canvas_assembly";
export type { DownstreamConflictStrategy } from "../dependencyConflict";

export interface VideoGenConfig {
  id: number;
  name: string;
  backend: string;
  base_url: string | null;
  api_key_encrypted?: string | null;
  has_api_key?: boolean;
  model: string;
  max_concurrent: number;
  default_width: number;
  default_height: number;
  default_duration_secs: number;
  min_duration_secs: number;
  max_duration_secs: number;
  duration_mode: VideoDurationMode;
  supported_durations_json: number[] | null;
  default_fps: number;
  provider_options_json?: unknown | null;
  is_default: boolean;
  created_at: string | null;
  updated_at: string | null;
}

export interface ChapterVideoPrompt {
  id: number;
  project_id: number;
  chapter_id: number;
  style: string | null;
  prompt_type: string;
  workflow_mode: VideoWorkflowMode;
  shot_number: number;
  title: string | null;
  scene_summary: string | null;
  visual_prompt: string;
  motion_prompt: string | null;
  negative_prompt: string | null;
  duration_secs: number;
  aspect_ratio: string | null;
  reference_image_ids_json: unknown | null;
  status: string;
  generated_video_id: number | null;
  metadata_json: unknown | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface GeneratedVideo {
  id: number;
  project_id: number;
  chapter_id: number | null;
  video_type: string;
  workflow_mode: VideoWorkflowMode | null;
  file_path: string | null;
  thumbnail_path: string | null;
  prompt: string;
  negative_prompt: string | null;
  model: string;
  width: number;
  height: number;
  duration_secs: number;
  fps: number;
  file_size: number | null;
  source_video_id: number | null;
  source_image_id: number | null;
  status: string;
  provider_job_id: string | null;
  error_message: string | null;
  metadata_json: unknown | null;
  is_favorite: boolean;
  created_at: string | null;
  updated_at: string | null;
}

export interface VideoPlaybackData {
  mime_type: string;
  data_base64: string;
}

interface AssembledChapterVideoClip {
  video_id: number;
  video_type: string;
  shot_number: number;
  sort_order: number;
  insert_after_video_id: number | null;
  duration_secs: number;
  file_path: string;
}

export interface AssembledChapterVideoPreview {
  mime_type: string;
  data_base64: string;
  file_path: string;
  video_ids: number[];
  clips: AssembledChapterVideoClip[];
  duration_secs: number;
  file_size: number;
  updated_at: string | null;
  cached: boolean;
}

export interface AssemblyPushResult {
  video: GeneratedVideo;
  source_video_ids: number[];
  push_index: number;
  status: string;
}

export interface CreateVideoConfigInput {
  name: string;
  backend: string;
  base_url: string;
  api_key_encrypted?: string | null;
  model: string;
  max_concurrent: number;
  default_width: number;
  default_height: number;
  default_duration_secs: number;
  min_duration_secs: number;
  max_duration_secs: number;
  duration_mode: VideoDurationMode;
  supported_durations_json?: number[] | null;
  default_fps: number;
  provider_options_json?: unknown | null;
}

export interface UpdateVideoConfigInput extends Omit<CreateVideoConfigInput, "api_key_encrypted"> {
  api_key_encrypted?: string | null;
}

export interface ReferenceMediaVideoInput {
  prompt: string;
  image_files: string[];
  video_files: string[];
  audio_files: string[];
  duration_secs: number;
  aspect_ratio: string;
  config_id?: number | null;
}

export interface ChapterVideosBatchInput {
  prompt_ids: number[];
  config_id?: number | null;
}

export interface ChapterVideosBatchResult {
  videos: GeneratedVideo[];
  success_count: number;
  failed_count: number;
  pending_count: number;
}

export interface PreviewChapterVideoPromptOverrideInput {
  visual_prompt?: string | null;
  motion_prompt?: string | null;
  negative_prompt?: string | null;
  duration_secs?: number | null;
  reference_image_ids_json?: unknown | null;
}
