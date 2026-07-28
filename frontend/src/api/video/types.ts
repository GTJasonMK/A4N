export type VideoDurationMode = "range" | "fixed";
export type PromptOutputLanguage = "zh_cn" | "en";
export type VideoWorkflowMode =
  | "shared_segment_plan"
  | "boundary_frames"
  | "first_frame_chain"
  | "storyboard"
  | "reference_media"
  | "chat_video"
  | "mixed"
  | "canvas_video"
  | "canvas_reference_media"
  | "canvas_trim"
  | "canvas_assembly";
export type { DownstreamConflictStrategy } from "../dependencyConflict";

export type VideoGenConfig = Omit<VideoConfigSummary, "duration_mode" | "capabilities"> & {
  duration_mode: VideoDurationMode;
  supported_durations_json: number[] | null;
  capabilities?: VideoConfigSummary["capabilities"];
};

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

export type GeneratedVideo = Omit<GeneratedVideoSummary, "workflow_mode"> & {
  workflow_mode: VideoWorkflowMode | null;
};

export type VideoPlaybackData = ApiVideoPlaybackData;

export type AssembledChapterVideoPreview = ApiAssembledChapterVideoPreview;

export type AssemblyPushResult = Omit<ApiAssemblyPushResult, "video"> & {
  video: GeneratedVideo;
};

export type DeleteChapterVideoAssetsResult = ApiDeleteChapterVideoAssetsResult;
export type UploadChapterVideoInput = ApiUploadChapterVideoInput;
export type UploadReferenceMediaVideoInput = ApiUploadReferenceMediaVideoInput;
export type UploadAssemblyInsertVideoInput = ApiUploadAssemblyInsertVideoInput;
export type PushAssembledChapterVideoInput = Omit<
  ApiPushAssembledChapterVideoInput,
  "end_video_id"
> & {
  end_video_id?: number | string | null;
};
export type ApplyAssemblyPostprocessEditInput = Omit<
  ApiApplyAssemblyPostprocessEditInput,
  "operation" | "source_video_id"
> & {
  operation: "insert" | "delete" | "replace";
  source_video_id?: number | string | null;
};

export type CreateVideoConfigInput = Omit<VideoConfigInput, "duration_mode"> & {
  duration_mode: VideoDurationMode;
  supported_durations_json?: number[] | null;
};

export type UpdateVideoConfigInput = Omit<ApiUpdateVideoConfigInput, "duration_mode"> & {
  duration_mode: VideoDurationMode;
  supported_durations_json?: number[] | null;
};

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
import type {
  UpdateVideoConfigInput as ApiUpdateVideoConfigInput,
  VideoConfigInput,
  VideoConfigSummary,
} from "../generated/config";
import type {
  ApplyAssemblyPostprocessEditInput as ApiApplyAssemblyPostprocessEditInput,
  AssembledChapterVideoPreview as ApiAssembledChapterVideoPreview,
  AssemblyPushResult as ApiAssemblyPushResult,
  DeleteChapterVideoAssetsResult as ApiDeleteChapterVideoAssetsResult,
  GeneratedVideoSummary,
  PushAssembledChapterVideoInput as ApiPushAssembledChapterVideoInput,
  UploadAssemblyInsertVideoInput as ApiUploadAssemblyInsertVideoInput,
  UploadChapterVideoInput as ApiUploadChapterVideoInput,
  UploadReferenceMediaVideoInput as ApiUploadReferenceMediaVideoInput,
  VideoPlaybackData as ApiVideoPlaybackData,
} from "../generated/host";
import type {
  ReferenceMediaVideoInput as ApiReferenceMediaVideoInput,
} from "../generated/generationContracts";

export type ReferenceMediaVideoInput = ApiReferenceMediaVideoInput;
