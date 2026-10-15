import { apiInvoke } from "../index";
import type { ModelRequestPreview } from "../requestPreview";
import { idNum, type ApiId as Id } from "../shared";
import { getLocalVideoConfig } from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";
import { normalizeVideoConfigId } from "./configs";
import type {
  ApplyAssemblyPostprocessEditInput,
  AssembledChapterVideoPreview,
  AssemblyPushAction,
  AssemblyPushResult,
  GeneratedVideo,
  PreviewChapterVideoPromptOverrideInput,
  PushAssembledChapterVideoInput,
  ReferenceMediaVideoInput,
  UpdateVideoInput,
  VideoRecordDeleteResult,
  UploadAssemblyInsertVideoInput,
  UploadChapterVideoInput,
  UploadReferenceMediaVideoInput,
  VideoPlaybackData,
  VideoSegmentTarget,
} from "./types";

export async function listVideosByChapter(
  projectId: Id,
  chapterId: Id,
): Promise<GeneratedVideo[]> {
  return apiInvoke<GeneratedVideo[]>("list_videos_by_chapter", {
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
  });
}

export async function updateVideo(videoId: Id, input: UpdateVideoInput): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("update_video", {
    videoId: idNum(videoId),
    input,
  });
}

export async function selectSegmentVideo(
  projectId: Id,
  chapterId: Id,
  videoId: Id,
  segmentPlanPromptId: Id,
  targetSegment: VideoSegmentTarget,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("select_segment_video", {
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    videoId: idNum(videoId),
    input: {
      segment_plan_prompt_id: idNum(segmentPlanPromptId, "片段规划提示词 ID"),
      target_segment: targetSegment,
    },
  });
}

export async function loadVideoPlaybackData(videoId: Id): Promise<VideoPlaybackData> {
  return apiInvoke<VideoPlaybackData>("load_video_playback_data", {
    videoId: idNum(videoId),
  });
}

export async function loadVideoScrubPlaybackData(
  videoId: Id,
  includeAudio: boolean,
): Promise<VideoPlaybackData> {
  return apiInvoke<VideoPlaybackData>("load_video_scrub_playback_data", {
    videoId: idNum(videoId),
    includeAudio,
  });
}

export async function assembleChapterVideos(
  projectId: Id,
  chapterId: Id,
  workflowMode: string,
): Promise<AssembledChapterVideoPreview> {
  return apiInvoke<AssembledChapterVideoPreview>("assemble_chapter_videos", {
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    workflowMode,
  });
}

export async function loadCachedAssembledChapterVideoPreview(
  projectId: Id,
  chapterId: Id,
  workflowMode: string,
): Promise<AssembledChapterVideoPreview | null> {
  return apiInvoke<AssembledChapterVideoPreview | null>(
    "load_cached_assembled_chapter_video_preview",
    {
      projectId: idNum(projectId),
      chapterId: idNum(chapterId),
      workflowMode,
    },
  );
}

export async function pushAssembledChapterVideo(
  projectId: Id,
  chapterId: Id,
  workflowMode: string,
  input: PushAssembledChapterVideoInput = {},
): Promise<AssemblyPushResult> {
  return apiInvoke<AssemblyPushResult>("push_assembled_chapter_video", {
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    workflowMode,
    input: {
      end_video_id: input.end_video_id == null ? null : idNum(input.end_video_id),
    },
  });
}

export async function updateAssemblyPushStatus(
  videoId: Id,
  action: AssemblyPushAction,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("update_assembly_push_status", {
    videoId: idNum(videoId),
    input: { action },
  });
}

export async function applyAssemblyPostprocessEdit(
  pushVideoId: Id,
  input: ApplyAssemblyPostprocessEditInput,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("apply_assembly_postprocess_edit", {
    pushVideoId: idNum(pushVideoId),
    input: {
      ...input,
      source_video_id: input.source_video_id == null ? null : idNum(input.source_video_id),
    },
  });
}

export async function deleteVideo(videoId: Id): Promise<VideoRecordDeleteResult> {
  return apiInvoke<VideoRecordDeleteResult>("delete_video", {
    videoId: idNum(videoId),
  });
}

export async function uploadChapterVideo(
  projectId: Id,
  chapterId: Id,
  input: UploadChapterVideoInput,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("upload_chapter_video", {
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    input,
  });
}

export async function uploadReferenceMediaVideo(
  projectId: Id,
  chapterId: Id,
  input: UploadReferenceMediaVideoInput,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("upload_reference_media_video", {
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    input,
  });
}

export async function uploadAssemblyInsertVideo(
  projectId: Id,
  chapterId: Id,
  workflowMode: string,
  input: UploadAssemblyInsertVideoInput,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("upload_assembly_insert_video", {
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    workflowMode,
    input,
  });
}

export async function trimVideo(
  videoId: Id,
  startSecs: number,
  endSecs: number,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("trim_video", {
    videoId: idNum(videoId),
    startSecs,
    endSecs,
  });
}

export async function previewChapterVideoRequest(
  projectId: Id,
  chapterId: Id,
  promptId: Id,
  configId?: Id | null,
  promptOverride?: PreviewChapterVideoPromptOverrideInput,
): Promise<ModelRequestPreview> {
  const normalizedConfigId = normalizeVideoConfigId(configId);
  return apiInvoke<ModelRequestPreview>("preview_chapter_video_request", {
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    promptId: idNum(promptId, "视频提示词 ID"),
    configId: normalizedConfigId,
    ...(promptOverride === undefined ? {} : { promptOverride }),
    ...(usesLocalRuntimeSettings() && normalizedConfigId != null
      ? {
          runtime_config_snapshot: buildRuntimeConfigSnapshot({
            video: getLocalVideoConfig(normalizedConfigId),
          }),
        }
      : {}),
  });
}

export async function previewVideoProviderRequestByVideoId(
  projectId: Id,
  chapterId: Id,
  videoId: Id,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_video_provider_request_by_video_id", {
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    videoId: idNum(videoId),
  });
}

export async function previewReferenceMediaVideoRequest(
  projectId: Id,
  chapterId: Id,
  input: ReferenceMediaVideoInput,
): Promise<ModelRequestPreview> {
  const normalizedConfigId = normalizeVideoConfigId(input.config_id);
  return apiInvoke<ModelRequestPreview>("preview_reference_media_video_request", {
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    input: { ...input, config_id: normalizedConfigId },
    ...(usesLocalRuntimeSettings() && normalizedConfigId != null
      ? {
          runtime_config_snapshot: buildRuntimeConfigSnapshot({
            video: getLocalVideoConfig(normalizedConfigId),
          }),
        }
      : {}),
  });
}
