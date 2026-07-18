import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { BindMangaReferenceImageInput, DownstreamConflictStrategy, MangaPanel } from "./types";
import { idNum, type ApiId as Id } from "../shared";

interface UpdateMangaPromptInput {
  prompt_id: number;
  image_prompt: string;
  downstream_strategy?: DownstreamConflictStrategy;
}

interface OptimizeMangaPromptInput {
  prompt_id: number;
  instruction: string;
  downstream_strategy?: DownstreamConflictStrategy;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问漫画面板");
  return id;
};

/** 后端 list_manga_panels(project_id, chapter_id) */
export async function listMangaPanels(projectId: Id, chapterId: Id): Promise<MangaPanel[]> {
  return apiInvoke<MangaPanel[]>("list_manga_panels", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
  });
}

/** 后端 bind_manga_reference_image(input) */
export async function bindMangaReferenceImage(
  input: BindMangaReferenceImageInput,
): Promise<MangaPanel> {
  return apiInvoke<MangaPanel>("bind_manga_reference_image", {
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
  input: UpdateMangaPromptInput,
): Promise<MangaPanel> {
  return apiInvoke<MangaPanel>("update_manga_prompt", {
    userId: currentUserId(),
    input: {
      ...input,
      prompt_id: idNum(input.prompt_id),
    },
  });
}

export async function optimizeMangaPrompt(
  input: OptimizeMangaPromptInput,
): Promise<MangaPanel> {
  return apiInvoke<MangaPanel>("optimize_manga_prompt", {
    userId: currentUserId(),
    input: {
      ...input,
      prompt_id: idNum(input.prompt_id),
    },
  });
}

export async function deleteMangaPrompt(
  promptId: Id,
  downstreamStrategy: DownstreamConflictStrategy = "reject",
): Promise<void> {
  return apiInvoke<void>("delete_manga_prompt", {
    userId: currentUserId(),
    promptId: idNum(promptId),
    downstreamStrategy,
  });
}

/** 后端 delete_manga_by_chapter(project_id, chapter_id) */
export async function deleteMangaByChapter(projectId: Id, chapterId: Id): Promise<void> {
  return apiInvoke<void>("delete_manga_by_chapter", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
  });
}
