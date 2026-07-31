import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import { idNum, integerNum, type ApiId as Id } from "../shared";
import { generationClient } from "../generationTasks";
import { generationTaskKey, useGenerationTasksStore } from "../../store/generationTasks";
import { sha256Hex } from "../../utils/contentHash";
import type { GenerationTaskViewState } from "../../utils/generationEventReducer";
import type {
  GenerationTaskHandle,
  StartVideoGenerationInput,
  VideoGenerationOperation,
} from "../generated/generationContracts";
import { SYSTEM_VIDEO_CONFIG_ID } from "./configs";
import { listVideosByChapter } from "./records";
import type { GeneratedVideo } from "./types";

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

export type VideoGenerationInput = Omit<StartVideoGenerationInput, "meta">;

export async function startVideoGeneration(
  input: VideoGenerationInput,
): Promise<GenerationTaskHandle> {
  return generationClient.start((meta) =>
    apiInvoke<GenerationTaskHandle>("start_video_generation", {
      userId: currentUserId(),
      input: {
        ...input,
        meta,
        project_id: idNum(input.project_id, "项目 ID"),
        operation: normalizeVideoGenerationOperation(input.operation),
      },
    }),
  );
}

export async function verifyVideoGenerationResult(
  task: GenerationTaskViewState,
  projectId: Id,
  chapterId: Id,
): Promise<GeneratedVideo[]> {
  const project = idNum(projectId, "项目 ID");
  const chapter = idNum(chapterId, "章节 ID");
  const videos = await listVideosByChapter(project, chapter);
  const resultRef = task.resultRef;
  let generated: GeneratedVideo[] = [];
  let valid = false;

  if (resultRef?.kind === "generated_video") {
    const videoId = Number(resultRef.id);
    const video = videos.find(
      (item) =>
        item.id === videoId
        && item.project_id === project
        && item.chapter_id === chapter
        && item.status === "completed",
    );
    const artifact = task.artifacts.find(
      (item) =>
        item.artifact_id === `generated-video:${videoId}`
        && item.artifact_kind === "generated_video",
    );
    valid = video != null
      && artifact != null
      && !artifact.temporary
      && artifact.revision === resultRef.revision
      && artifact.sha256 === resultRef.revision;
    if (valid && video) generated = [video];
  } else if (
    resultRef?.kind === "generated_video_batch"
    && Number(resultRef.id) === project
  ) {
    const artifacts = task.artifacts.filter(
      (item) => item.artifact_kind === "generated_video" && !item.temporary,
    );
    const ids = artifacts
      .map((artifact) => artifact.artifact_id.match(/^generated-video:(\d+)$/)?.[1])
      .filter((id): id is string => id != null)
      .map(Number);
    const uniqueIds = [...new Set(ids)].sort((left, right) => left - right);
    const revision = await sha256Hex(JSON.stringify(uniqueIds));
    generated = uniqueIds
      .map((id) =>
        videos.find(
          (video) =>
            video.id === id
            && video.project_id === project
            && video.chapter_id === chapter
            && video.status === "completed",
        ),
      )
      .filter((video): video is GeneratedVideo => video != null);
    valid = uniqueIds.length > 0
      && uniqueIds.length === artifacts.length
      && artifacts.every((artifact) => artifact.revision === artifact.sha256)
      && generated.length === uniqueIds.length
      && revision != null
      && revision === resultRef.revision;
  }

  if (!valid) {
    useGenerationTasksStore.getState().dispatch(
      generationTaskKey(task.taskId, task.attempt),
      { type: "client_state", state: "consistency_error" },
    );
    throw new Error("视频生成已结束，但正式结果校验失败");
  }
  return generated;
}

export async function pollVideoJob(videoId: Id): Promise<GeneratedVideo> {
  return apiInvoke<GeneratedVideo>("poll_video_job", {
    userId: currentUserId(),
    videoId: idNum(videoId, "视频 ID"),
  });
}

function normalizeVideoGenerationOperation(
  operation: VideoGenerationOperation,
): VideoGenerationOperation {
  switch (operation.kind) {
    case "chapter":
      return {
        ...operation,
        chapter_id: idNum(operation.chapter_id, "章节 ID"),
        prompt_id: idNum(operation.prompt_id, "视频提示词 ID"),
        config_id: normalizeVideoConfigId(operation.config_id),
      };
    case "chapter_batch":
      return {
        ...operation,
        chapter_id: idNum(operation.chapter_id, "章节 ID"),
        prompt_ids: operation.prompt_ids.map((id) => idNum(id, "视频提示词 ID")),
        config_id: normalizeVideoConfigId(operation.config_id),
      };
    case "reference_media": {
      return {
        ...operation,
        chapter_id: idNum(operation.chapter_id, "章节 ID"),
        input: {
          ...operation.input,
          config_id: normalizeVideoConfigId(operation.input.config_id),
        },
      };
    }
    case "prompt_segment_plan":
    case "prompt_frames":
    case "prompt_videos":
      return {
        ...operation,
        chapter_id: idNum(operation.chapter_id, "章节 ID"),
      };
  }
}
