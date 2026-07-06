import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import { idNum, integerNum, type ApiId as Id } from "../shared";
import { getLocalVideoConfig } from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";
import { SYSTEM_VIDEO_CONFIG_ID } from "./configs";
import type {
  AssemblyPushResult,
  AssembledChapterVideoPreview,
  GeneratedVideo,
  PreviewChapterVideoPromptOverrideInput,
  ReferenceMediaVideoInput,
  VideoPlaybackData,
} from "./types";

interface UploadChapterVideoInput {
  file_name: string;
  mime_type: string;
  data_base64: string;
  prompt_id: number;
}

interface UploadReferenceMediaVideoInput {
  file_name: string;
  mime_type: string;
  data_base64: string;
  prompt: string;
  duration_secs: number;
  aspect_ratio: string;
}

interface UploadAssemblyInsertVideoInput {
  file_name: string;
  mime_type: string;
  data_base64: string;
  insert_after_video_id?: number | null;
}

interface PushAssembledChapterVideoInput {
  end_video_id?: Id | null;
}

interface ApplyAssemblyPostprocessEditInput {
  operation: "insert" | "delete" | "replace";
  at_secs?: number | null;
  start_secs?: number | null;
  end_secs?: number | null;
  source_video_id?: Id | null;
  source_file_name?: string | null;
  source_mime_type?: string | null;
  source_data_base64?: string | null;
}

export interface UpdateVideoInput {
  prompt?: string;
  negative_prompt?: string | null;
  metadata_json?: unknown | null;
  is_favorite?: boolean;
  insert_after_video_id?: Id | null;
  move_assembly_insert_to_start?: boolean;
}

export interface DeleteChapterVideoAssetsResult {
  deleted_video_count: number;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问视频记录");
  return id;
};

export async function listVideosByChapter(
  projectId: Id,
  chapterId: Id,
): Promise<GeneratedVideo[]> {
  return apiInvoke<GeneratedVideo[]>("list_videos_by_chapter", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
  });
}

export async function toggleVideoFavorite(videoId: Id): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("toggle_video_favorite", {
    userId: currentUserId(),
    videoId: idNum(videoId),
  });
}

export async function updateVideo(videoId: Id, input: UpdateVideoInput): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("update_video", {
    userId: currentUserId(),
    videoId: idNum(videoId),
    input: toUpdateVideoPayload(input),
  });
}

export async function loadVideoPlaybackData(videoId: Id): Promise<VideoPlaybackData> {
  return apiInvoke<VideoPlaybackData>("load_video_playback_data", {
    userId: currentUserId(),
    videoId: idNum(videoId),
  });
}

export async function assembleChapterVideos(
  projectId: Id,
  chapterId: Id,
  workflowMode: string,
): Promise<AssembledChapterVideoPreview> {
  return apiInvoke<AssembledChapterVideoPreview>("assemble_chapter_videos", {
    userId: currentUserId(),
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
      userId: currentUserId(),
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
    userId: currentUserId(),
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
  action: "accept" | "reject" | "undo",
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("update_assembly_push_status", {
    userId: currentUserId(),
    videoId: idNum(videoId),
    input: { action },
  });
}

export async function applyAssemblyPostprocessEdit(
  pushVideoId: Id,
  input: ApplyAssemblyPostprocessEditInput,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("apply_assembly_postprocess_edit", {
    userId: currentUserId(),
    pushVideoId: idNum(pushVideoId),
    input: {
      operation: input.operation,
      at_secs: input.at_secs ?? null,
      start_secs: input.start_secs ?? null,
      end_secs: input.end_secs ?? null,
      source_video_id: input.source_video_id == null ? null : idNum(input.source_video_id),
      source_file_name: input.source_file_name ?? null,
      source_mime_type: input.source_mime_type ?? null,
      source_data_base64: input.source_data_base64 ?? null,
    },
  });
}

export async function deleteVideo(videoId: Id): Promise<void> {
  return apiInvoke<void>("delete_video", {
    userId: currentUserId(),
    videoId: idNum(videoId),
  });
}

export async function deleteChapterVideoAssets(
  projectId: Id,
  chapterId: Id,
): Promise<DeleteChapterVideoAssetsResult> {
  return apiInvoke<DeleteChapterVideoAssetsResult>("delete_chapter_video_assets", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
  });
}

export async function uploadChapterVideo(
  projectId: Id,
  chapterId: Id,
  input: UploadChapterVideoInput,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("upload_chapter_video", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    input: {
      file_name: input.file_name,
      mime_type: input.mime_type,
      data_base64: input.data_base64,
      prompt_id: idNum(input.prompt_id),
    },
  });
}

export async function uploadReferenceMediaVideo(
  projectId: Id,
  chapterId: Id,
  input: UploadReferenceMediaVideoInput,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("upload_reference_media_video", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    input: {
      file_name: input.file_name,
      mime_type: input.mime_type,
      data_base64: input.data_base64,
      prompt: input.prompt,
      duration_secs: input.duration_secs,
      aspect_ratio: input.aspect_ratio,
    },
  });
}

export async function uploadAssemblyInsertVideo(
  projectId: Id,
  chapterId: Id,
  workflowMode: string,
  input: UploadAssemblyInsertVideoInput,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("upload_assembly_insert_video", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    workflowMode,
    input: {
      file_name: input.file_name,
      mime_type: input.mime_type,
      data_base64: input.data_base64,
      insert_after_video_id:
        input.insert_after_video_id == null ? null : idNum(input.insert_after_video_id),
    },
  });
}

export async function trimVideo(
  videoId: Id,
  startSecs: number,
  endSecs: number,
): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("trim_video", {
    userId: currentUserId(),
    videoId: idNum(videoId),
    startSecs,
    endSecs,
  });
}

function normalizeVideoConfigId(configId?: Id | null): number | null {
  if (configId == null) return null;
  const id = integerNum(configId, "视频配置 ID");
  return id === SYSTEM_VIDEO_CONFIG_ID ? null : idNum(id, "视频配置 ID");
}

export async function previewChapterVideoRequest(
  projectId: Id,
  chapterId: Id,
  promptId: Id,
  configId?: Id | null,
  promptOverride?: PreviewChapterVideoPromptOverrideInput | null,
): Promise<ModelRequestPreview> {
  const normalizedConfigId = normalizeVideoConfigId(configId);
  return apiInvoke<ModelRequestPreview>("preview_chapter_video_request", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    promptId: idNum(promptId, "视频提示词 ID"),
    configId: normalizedConfigId,
    promptOverride: promptOverride ?? null,
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
    userId: currentUserId(),
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
    userId: currentUserId(),
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

function toUpdateVideoPayload(input: UpdateVideoInput): Record<string, unknown> {
  const payload: Record<string, unknown> = {};
  if (input.prompt !== undefined) payload.prompt = input.prompt;
  if ("negative_prompt" in input) {
    payload.negative_prompt = input.negative_prompt ?? null;
    payload.clear_negative_prompt = input.negative_prompt == null;
  }
  if ("metadata_json" in input) {
    payload.metadata_json = input.metadata_json ?? null;
    payload.clear_metadata_json = input.metadata_json == null;
  }
  if (input.is_favorite !== undefined) payload.is_favorite = input.is_favorite;
  if ("insert_after_video_id" in input) {
    payload.insert_after_video_id =
      input.insert_after_video_id == null ? null : idNum(input.insert_after_video_id);
    payload.move_assembly_insert_to_start = input.insert_after_video_id == null;
  }
  if (input.move_assembly_insert_to_start !== undefined) {
    payload.move_assembly_insert_to_start = input.move_assembly_insert_to_start;
  }
  return payload;
}
