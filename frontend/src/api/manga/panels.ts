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

export interface CreateMangaPromptInput {
  page_number: number;
  panel_number?: number | null;
  image_prompt: string;
  dialogue_text?: string | null;
  characters_in_frame?: unknown | null;
  layout?: unknown | null;
  camera_angle?: string | null;
  dialogue_position?: string | null;
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

export async function createMangaPrompt(
  projectId: Id,
  chapterId: Id,
  input: CreateMangaPromptInput,
): Promise<MangaPanel> {
  return apiInvoke<MangaPanel>("create_manga_prompt", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: idNum(chapterId),
    input: {
      page_number: input.page_number,
      panel_number: input.panel_number ?? null,
      image_prompt: input.image_prompt,
      dialogue_text: input.dialogue_text ?? null,
      characters_in_frame: input.characters_in_frame ?? null,
      layout: input.layout ?? null,
      camera_angle: input.camera_angle ?? null,
      dialogue_position: input.dialogue_position ?? null,
    },
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
