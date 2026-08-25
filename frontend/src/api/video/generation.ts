import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import { idNum, type ApiId as Id } from "../shared";
import { generationClient } from "../generationTasks";
import { verifyGeneratedMediaResult } from "../generationResultVerification";
import type { GenerationTaskViewState } from "../../utils/generationEventReducer";
import type {
  GenerationTaskHandle,
  GenerationStartRequestMeta,
  StartVideoGenerationInput,
  VideoGenerationOperation,
} from "../generated/generationContracts";
import { normalizeVideoConfigId } from "./configs";
import { listVideosByChapter } from "./records";
import type { GeneratedVideo } from "./types";
import { getActiveLocale, translateMessage } from "../../i18n";

type VideoGenerationInput = Omit<StartVideoGenerationInput, "meta">;

export async function startVideoGeneration(
  input: VideoGenerationInput,
  retryOfTaskId?: string | null,
): Promise<GenerationTaskHandle> {
  const start = (meta: GenerationStartRequestMeta) =>
    apiInvoke<GenerationTaskHandle>("start_video_generation", {
      input: {
        ...input,
        meta,
        project_id: idNum(input.project_id, "项目 ID"),
        operation: normalizeVideoGenerationOperation(input.operation),
      },
    });
  return retryOfTaskId == null
    ? generationClient.start(start)
    : generationClient.start(start, retryOfTaskId);
}

export async function verifyVideoGenerationResult(
  task: GenerationTaskViewState,
  projectId: Id,
  chapterId: Id,
): Promise<[GeneratedVideo, ...GeneratedVideo[]]> {
  const project = idNum(projectId, "项目 ID");
  const chapter = idNum(chapterId, "章节 ID");
  const videos = await listVideosByChapter(project, chapter);
  return verifyGeneratedMediaResult(task, videos, {
    projectId: project,
    singleKind: "generated_video",
    batchKind: "generated_video_batch",
    artifactKind: "generated_video",
    artifactIdPrefix: "generated-video:",
    recordInScope: (video) =>
      video.project_id === project
      && video.chapter_id === chapter
      && video.status === "completed",
    errorMessage: translateMessage(getActiveLocale(), "errors.mediaGenerationConsistencyFailed"),
  });
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
        prompt_ids: operation.prompt_ids.map((id) =>
          idNum(id, "视频提示词 ID"),
        ),
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
    case "prompt_inserted_segment":
    case "prompt_frames":
    case "prompt_videos":
      return {
        ...operation,
        chapter_id: idNum(operation.chapter_id, "章节 ID"),
      };
  }
}
