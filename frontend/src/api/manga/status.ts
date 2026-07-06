import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { PipelineStatus } from "./types";
import { idNum, type ApiId as Id } from "../shared";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问漫画流水线状态");
  return id;
};

export async function getMangaPipelineStatus(
  projectId: Id,
  chapterId: Id,
): Promise<PipelineStatus> {
  return apiInvoke<PipelineStatus>("get_manga_pipeline_status", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
  });
}

/** 后端 cancel_manga_pipeline(project_id, chapter_id) —— 返回取消的整页提示词数 */
export async function cancelMangaPipeline(projectId: Id, chapterId: Id): Promise<number> {
  return apiInvoke<number>("cancel_manga_pipeline", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
  });
}
