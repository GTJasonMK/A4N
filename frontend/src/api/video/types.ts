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
export type {
  ChapterVideoPrompt,
  DownstreamConflictStrategy,
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
  "operation" | "source_video_id"
> & {
  operation: "insert" | "delete" | "replace";
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
