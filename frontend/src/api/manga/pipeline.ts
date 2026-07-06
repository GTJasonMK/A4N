import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type { MangaPipelineStartStage } from "./types";
import { idNum, type ApiId as Id } from "../shared";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法运行漫画流水线");
  return id;
};

/** 后端 start_manga_pipeline(project_id, chapter_number) —— 返回 task_id */
export async function startMangaPipeline(
  projectId: Id,
  chapterNumber: number,
  startFromStage?: MangaPipelineStartStage | null,
): Promise<string> {
  return apiInvoke<string>("start_manga_pipeline", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterNumber,
    startFromStage: startFromStage ?? null,
  });
}

export async function previewStartMangaPipelineRequest(
  projectId: Id,
  chapterNumber: number,
  startFromStage?: MangaPipelineStartStage | null,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_start_manga_pipeline_request", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterNumber,
    startFromStage: startFromStage ?? null,
  });
}

/** 后端 resume_manga_pipeline(project_id, chapter_number) */
export async function resumeMangaPipeline(
  projectId: Id,
  chapterNumber: number,
): Promise<string> {
  return apiInvoke<string>("resume_manga_pipeline", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterNumber,
  });
}

export async function previewResumeMangaPipelineRequest(
  projectId: Id,
  chapterNumber: number,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_resume_manga_pipeline_request", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterNumber,
  });
}
