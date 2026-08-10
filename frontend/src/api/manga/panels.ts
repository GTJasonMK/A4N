import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type * as Host from "../generated/host";
import { idNum, type ApiId as Id } from "../shared";

/** 后端 list_manga_panels(project_id, chapter_id) */
export async function listMangaPanels(
  projectId: Id,
  chapterId: Id,
): Promise<Host.MangaPromptSummary[]> {
  return apiInvoke<Host.MangaPromptSummary[]>("list_manga_panels", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      chapter_id: idNum(chapterId),
    } satisfies Host.MangaChapterInput,
  });
}

/** 后端 bind_manga_reference_image(input) */
export async function bindMangaReferenceImage(
  input: Host.BindMangaReferenceImageInput,
): Promise<Host.MangaPromptSummary> {
  return apiInvoke<Host.MangaPromptSummary>("bind_manga_reference_image", {
    userId: currentUserId(),
    input: {
      ...input,
      prompt_id: idNum(input.prompt_id),
      asset_id: idNum(input.asset_id),
    },
  });
}

/** 后端 update_manga_prompt(input) */
export async function updateMangaPrompt(
  input: Host.UpdateMangaPromptInput,
): Promise<Host.MangaPromptSummary> {
  return apiInvoke<Host.MangaPromptSummary>("update_manga_prompt", {
    userId: currentUserId(),
    input: {
      ...input,
      prompt_id: idNum(input.prompt_id),
    },
  });
}

export async function optimizeMangaPrompt(
  input: Host.OptimizeMangaPromptInput,
): Promise<Host.MangaPromptSummary> {
  return apiInvoke<Host.MangaPromptSummary>("optimize_manga_prompt", {
    userId: currentUserId(),
    input: {
      ...input,
      prompt_id: idNum(input.prompt_id),
    },
  });
}

export async function deleteMangaPrompt(
  promptId: Id,
  downstreamStrategy: Host.DownstreamConflictStrategy,
): Promise<void> {
  return apiInvoke<void>("delete_manga_prompt", {
    userId: currentUserId(),
    input: {
      prompt_id: idNum(promptId),
      downstream_strategy: downstreamStrategy,
    } satisfies Host.DeleteMangaPromptInput,
  });
}

/** 后端 delete_manga_by_chapter(project_id, chapter_id) */
export async function deleteMangaByChapter(projectId: Id, chapterId: Id): Promise<void> {
  return apiInvoke<void>("delete_manga_by_chapter", {
    userId: currentUserId(),
    input: {
      project_id: idNum(projectId),
      chapter_id: idNum(chapterId),
    } satisfies Host.MangaChapterInput,
  });
}
