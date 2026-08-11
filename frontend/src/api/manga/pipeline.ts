import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type * as Host from "../generated/host";
import type { ModelRequestPreview } from "../requestPreview";
import { idNum, type ApiId as Id } from "../shared";

/** 后端 start_manga_pipeline(project_id, chapter_number) —— 返回 task_id */
export async function startMangaPipeline(
  projectId: Id,
  chapterNumber: number,
): Promise<string> {
  return apiInvoke<string>("start_manga_pipeline", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      chapter_number: chapterNumber,
    } satisfies Host.MangaPipelineStartInput,
  });
}

export async function previewStartMangaPipelineRequest(
  projectId: Id,
  chapterNumber: number,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_start_manga_pipeline_request", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      chapter_number: chapterNumber,
    } satisfies Host.MangaPipelineStartInput,
  });
}

/** 后端 resume_manga_pipeline(project_id, chapter_number) */
export async function resumeMangaPipeline(
  projectId: Id,
  chapterNumber: number,
): Promise<string> {
  return apiInvoke<string>("resume_manga_pipeline", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      chapter_number: chapterNumber,
    } satisfies Host.MangaPipelineResumeInput,
  });
}

export async function previewResumeMangaPipelineRequest(
  projectId: Id,
  chapterNumber: number,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_resume_manga_pipeline_request", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      chapter_number: chapterNumber,
    } satisfies Host.MangaPipelineResumeInput,
  });
}
