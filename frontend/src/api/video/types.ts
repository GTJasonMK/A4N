export type PromptOutputLanguage = "zh_cn" | "en";

export const VIDEO_TYPE_CHAPTER = "chapter_video" as const;
export const VIDEO_TYPE_REFERENCE_MEDIA = "reference_media_video" as const;
export const VIDEO_TYPE_TRIMMED_CLIP = "trimmed_clip" as const;
export const VIDEO_TYPE_ASSEMBLY_INSERT = "assembly_insert" as const;
export const VIDEO_TYPE_ASSEMBLY_PUSH = "assembly_push" as const;
export const VIDEO_TYPE_ASSEMBLY_POSTPROCESS_EDIT =
  "assembly_postprocess_edit" as const;
export const VIDEO_TYPE_CANVAS = "canvas_video" as const;

export const VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN =
  "shared_segment_plan" as const;
export const VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES = "boundary_frames" as const;
export const VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN =
  "first_frame_chain" as const;
export const VIDEO_WORKFLOW_MODE_STORYBOARD = "storyboard" as const;
export const VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA = "reference_media" as const;
export const VIDEO_WORKFLOW_MODE_CHAT_VIDEO = "chat_video" as const;
export const VIDEO_WORKFLOW_MODE_MIXED = "mixed" as const;
export const VIDEO_WORKFLOW_MODE_CANVAS_VIDEO = "canvas_video" as const;
export const VIDEO_WORKFLOW_MODE_CANVAS_TRIM = "canvas_trim" as const;
export const VIDEO_WORKFLOW_MODE_CANVAS_ASSEMBLY = "canvas_assembly" as const;
export const VIDEO_SEGMENT_WORKFLOW_MODES = [
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
] as const;

export type VideoSegmentWorkflowMode =
  (typeof VIDEO_SEGMENT_WORKFLOW_MODES)[number];
export type VideoWorkflowMode =
  | typeof VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN
  | VideoSegmentWorkflowMode
  | typeof VIDEO_WORKFLOW_MODE_MIXED
  | typeof VIDEO_WORKFLOW_MODE_CANVAS_VIDEO
  | typeof VIDEO_WORKFLOW_MODE_CANVAS_TRIM
  | typeof VIDEO_WORKFLOW_MODE_CANVAS_ASSEMBLY;
export type {
  AssemblyPushAction,
  ChapterVideoPrompt,
  DownstreamConflictStrategy,
  PreviewChapterVideoPromptOverrideInput,
  VideoPromptDeleteResult,
  VideoPromptDeletionEffects,
  VideoPromptUpdateResult,
  VideoRecordDeleteResult,
  ShootingEditingModeV1,
  ShootingTransitionTypeV1,
  UpdateChapterVideoSegmentPlanInput,
  VideoEmotionActionGoalV1,
  VideoPerformanceActionV1,
  VideoPerformanceTimeBlockV1,
  VideoSegmentPlanV2,
  VideoSpatialAnchorV1,
  VideoSegmentTarget,
} from "../generated/host";

type VideoGenConfigBase = Omit<
  VideoConfigSummary,
  "duration_mode" | "supported_durations_json"
> & {
  api_key_encrypted?: string | null;
};

export type VideoGenConfig = VideoGenConfigBase &
  (
    | {
        duration_mode: "range";
        supported_durations_json: null;
      }
    | {
        duration_mode: "fixed";
        supported_durations_json: number[];
      }
  );

export type GeneratedVideo = Omit<GeneratedVideoSummary, "workflow_mode"> & {
  workflow_mode: VideoWorkflowMode | null;
};

export type VideoPlaybackData = ApiVideoPlaybackData;

export type AssembledChapterVideoPreview = ApiAssembledChapterVideoPreview;

export type AssemblyPushResult = Omit<ApiAssemblyPushResult, "video"> & {
  video: GeneratedVideo;
};

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
  "source_video_id"
> & {
  source_video_id?: number | string | null;
};

export type CreateVideoConfigInput = Omit<VideoConfigInput, "supported_durations_json"> & {
  supported_durations_json?: number[] | null;
};

export type UpdateVideoConfigInput = Omit<
  ApiUpdateVideoConfigInput,
  "supported_durations_json"
> & {
  supported_durations_json?: number[] | null;
};

import type {
  UpdateVideoConfigInput as ApiUpdateVideoConfigInput,
  VideoConfigInput,
  VideoConfigSummary,
  VideoDurationMode,
} from "../generated/config";
export type { VideoDurationMode };
import type {
  ApplyAssemblyPostprocessEditInput as ApiApplyAssemblyPostprocessEditInput,
  AssembledChapterVideoPreview as ApiAssembledChapterVideoPreview,
  AssemblyPushResult as ApiAssemblyPushResult,
  GeneratedVideoSummary,
  PushAssembledChapterVideoInput as ApiPushAssembledChapterVideoInput,
  UpdateGeneratedVideoInput as ApiUpdateGeneratedVideoInput,
  UploadAssemblyInsertVideoInput as ApiUploadAssemblyInsertVideoInput,
  UploadChapterVideoInput as ApiUploadChapterVideoInput,
  UploadReferenceMediaVideoInput as ApiUploadReferenceMediaVideoInput,
  VideoPlaybackData as ApiVideoPlaybackData,
} from "../generated/host";
import type {
  ReferenceMediaVideoInput as ApiReferenceMediaVideoInput,
} from "../generated/generationContracts";

export type ReferenceMediaVideoInput = ApiReferenceMediaVideoInput;
export type UpdateVideoInput = ApiUpdateGeneratedVideoInput;
export type UpdateVideoFields = Omit<UpdateVideoInput, "expected_updated_at">;
