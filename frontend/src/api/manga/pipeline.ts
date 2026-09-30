import { apiInvoke } from "../index";
import type * as Host from "../generated/host";
import type { ModelRequestPreview } from "../requestPreview";
import { idNum, type ApiId as Id } from "../shared";
import { generationClient } from "../generationTasks";
import type { GenerationTaskHandle } from "../generated/generationContracts";
import type { ImageSize } from "../generated/config";

/** 后端 start_manga_pipeline(project_id, chapter_number) —— 返回 task_id */
export async function startMangaPipeline(
  projectId: Id,
  chapterNumber: number,
  stopAfter: Host.MangaPipelineStopAfter | null = null,
  imageSize: ImageSize | null = null,
): Promise<GenerationTaskHandle> {
  return generationClient.start((meta) => apiInvoke<GenerationTaskHandle>("start_manga_pipeline", {
    input: {
      meta,
      project_id: idNum(projectId),
      chapter_number: chapterNumber,
      stop_after: stopAfter,
      image_size: imageSize,
      resume_from_checkpoint: false,
    } satisfies Host.MangaPipelineStartInput,
  }));
}

export async function previewStartMangaPipelineRequest(
  projectId: Id,
  chapterNumber: number,
  stopAfter: Host.MangaPipelineStopAfter | null = null,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_start_manga_pipeline_request", {
    input: {
      project_id: idNum(projectId),
      chapter_number: chapterNumber,
      stop_after: stopAfter,
    } satisfies Host.MangaPipelineStartInput,
  });
}

/** 后端 resume_manga_pipeline(project_id, chapter_number) */
export async function resumeMangaPipeline(
  projectId: Id,
  chapterNumber: number,
  stopAfter: Host.MangaPipelineStopAfter | null = null,
  imageSize: ImageSize | null = null,
): Promise<GenerationTaskHandle> {
  return generationClient.start((meta) => apiInvoke<GenerationTaskHandle>("resume_manga_pipeline", {
    input: {
      meta,
      project_id: idNum(projectId),
      chapter_number: chapterNumber,
      stop_after: stopAfter,
      image_size: imageSize,
      resume_from_checkpoint: true,
    } satisfies Host.MangaPipelineResumeInput,
  }));
}

export async function previewResumeMangaPipelineRequest(
  projectId: Id,
  chapterNumber: number,
  stopAfter: Host.MangaPipelineStopAfter | null = null,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_resume_manga_pipeline_request", {
    input: {
      project_id: idNum(projectId),
      chapter_number: chapterNumber,
      stop_after: stopAfter,
    } satisfies Host.MangaPipelineResumeInput,
  });
}
