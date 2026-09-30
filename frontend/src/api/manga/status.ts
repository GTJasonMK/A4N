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

export async function listMangaPipelineStatuses(
  projectId: Id,
  chapterIds: Id[],
): Promise<Host.MangaPipelineStatusSummary[]> {
  return apiInvoke<Host.MangaPipelineStatusSummary[]>("list_manga_pipeline_statuses", {
    input: {
      project_id: idNum(projectId),
      chapter_ids: chapterIds.map(idNum),
    } satisfies Host.MangaPipelineStatusesInput,
  });
}
