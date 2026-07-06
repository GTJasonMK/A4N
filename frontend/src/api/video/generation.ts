import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import { idNum, integerNum, type ApiId as Id } from "../shared";
import type {
  ChapterVideosBatchInput,
  ChapterVideosBatchResult,
  GeneratedVideo,
  ReferenceMediaVideoInput,
} from "./types";
import { getLocalVideoConfig } from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import { assertRemoteBackendCanUsePaths } from "../remotePathGuard";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";
import { SYSTEM_VIDEO_CONFIG_ID } from "./configs";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法生成视频");
  return id;
};

function normalizeVideoConfigId(configId?: Id | null): number | null {
  if (configId == null) return null;
  const id = integerNum(configId, "视频配置 ID");
  return id === SYSTEM_VIDEO_CONFIG_ID ? null : idNum(id, "视频配置 ID");
}

export async function generateChapterVideo(
  projectId: Id,
  chapterId: Id,
  promptId: Id,
  configId?: Id | null,
  taskId?: string | null,
): Promise<GeneratedVideo> {
  const normalizedConfigId = normalizeVideoConfigId(configId);
  return apiInvoke<GeneratedVideo>("generate_chapter_video", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    promptId: idNum(promptId, "视频提示词 ID"),
    configId: normalizedConfigId,
    taskId: taskId ?? null,
    ...(usesLocalRuntimeSettings() && normalizedConfigId != null
      ? {
          runtime_config_snapshot: buildRuntimeConfigSnapshot({
            video: getLocalVideoConfig(normalizedConfigId),
          }),
        }
      : {}),
  });
}

export async function generateChapterVideosBatch(
  projectId: Id,
  chapterId: Id,
  input: ChapterVideosBatchInput,
  taskId?: string | null,
): Promise<ChapterVideosBatchResult> {
  const normalizedConfigId = normalizeVideoConfigId(input.config_id);
  return apiInvoke<ChapterVideosBatchResult>("generate_chapter_videos_batch", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    promptIds: input.prompt_ids.map((id) => idNum(id, "视频提示词 ID")),
    configId: normalizedConfigId,
    taskId: taskId ?? null,
    ...(usesLocalRuntimeSettings() && normalizedConfigId != null
      ? {
          runtime_config_snapshot: buildRuntimeConfigSnapshot({
            video: getLocalVideoConfig(normalizedConfigId),
          }),
        }
      : {}),
  });
}

export async function pollVideoJob(videoId: Id): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("poll_video_job", {
    userId: currentUserId(),
    videoId: idNum(videoId, "视频 ID"),
  });
}

export async function generateReferenceMediaVideo(
  projectId: Id,
  chapterId: Id,
  input: ReferenceMediaVideoInput,
  taskId?: string | null,
): Promise<GeneratedVideo> {
  assertRemoteBackendCanUsePaths(input.image_files, "参考媒体图片");
  assertRemoteBackendCanUsePaths(input.video_files, "参考媒体视频");
  assertRemoteBackendCanUsePaths(input.audio_files, "参考媒体音频");
  const normalizedConfigId = normalizeVideoConfigId(input.config_id);
  return apiInvoke<GeneratedVideo>("generate_reference_media_video", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    input: { ...input, config_id: normalizedConfigId },
    taskId: taskId ?? null,
    ...(usesLocalRuntimeSettings() && normalizedConfigId != null
      ? {
          runtime_config_snapshot: buildRuntimeConfigSnapshot({
            video: getLocalVideoConfig(normalizedConfigId),
          }),
        }
      : {}),
  });
}
