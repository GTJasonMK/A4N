import { apiInvoke } from "../index";
import type * as Host from "../generated/host";
import { idNum, type ApiId as Id } from "../shared";

export async function getMangaPipelineStatus(
  projectId: Id,
  chapterId: Id,
): Promise<Host.MangaPipelineStatusSummary> {
  return apiInvoke<Host.MangaPipelineStatusSummary>("get_manga_pipeline_status", {
    input: {
      project_id: idNum(projectId),
      chapter_id: idNum(chapterId),
    } satisfies Host.MangaChapterInput,
  });
}

/** 后端 cancel_manga_pipeline(project_id, chapter_id) —— 返回取消的整页提示词数 */
export async function cancelMangaPipeline(projectId: Id, chapterId: Id): Promise<number> {
  return apiInvoke<number>("cancel_manga_pipeline", {
    input: {
      project_id: idNum(projectId),
      chapter_id: idNum(chapterId),
    } satisfies Host.MangaChapterInput,
  });
}
