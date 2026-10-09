import { apiInvoke } from "../index";
import type * as Host from "../generated/host";
import { idNum, type ApiId as Id } from "../shared";

/** 后端 list_manga_page_prompts(project_id, chapter_id) */
export async function listMangaPagePrompts(
  projectId: Id,
  chapterId: Id,
): Promise<Host.MangaPagePromptSummary[]> {
  return apiInvoke<Host.MangaPagePromptSummary[]>("list_manga_page_prompts", {
    input: {
      project_id: idNum(projectId),
      chapter_id: idNum(chapterId),
    } satisfies Host.MangaChapterInput,
  });
}

/** 后端 bind_manga_reference_image(input) */
export async function bindMangaReferenceImage(
  input: Host.BindMangaReferenceImageInput,
): Promise<Host.MangaPagePromptSummary> {
  return apiInvoke<Host.MangaPagePromptSummary>("bind_manga_reference_image", {
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
): Promise<Host.MangaPagePromptSummary> {
  return apiInvoke<Host.MangaPagePromptSummary>("update_manga_prompt", {
    input: {
      ...input,
      prompt_id: idNum(input.prompt_id),
    },
  });
}

export async function optimizeMangaPrompt(
  input: Host.OptimizeMangaPromptInput,
): Promise<Host.MangaPagePromptSummary> {
  return apiInvoke<Host.MangaPagePromptSummary>("optimize_manga_prompt", {
    input: {
      ...input,
      prompt_id: idNum(input.prompt_id),
    },
  });
}

export async function deleteMangaPage(
  input: Host.DeleteMangaPageInput,
): Promise<void> {
  return apiInvoke<void>("delete_manga_page", {
    input: {
      ...input,
      project_id: idNum(input.project_id),
      chapter_id: idNum(input.chapter_id),
      page_number: idNum(input.page_number),
    },
  });
}

/** 后端 delete_manga_by_chapter(project_id, chapter_id) */
export async function deleteMangaByChapter(projectId: Id, chapterId: Id): Promise<void> {
  return apiInvoke<void>("delete_manga_by_chapter", {
    input: {
      project_id: idNum(projectId),
      chapter_id: idNum(chapterId),
    } satisfies Host.MangaChapterInput,
  });
}
