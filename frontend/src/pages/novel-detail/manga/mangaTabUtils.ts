import type { MangaPagePromptSummary } from "../../../api/manga";
import type { ChapterRow } from "../shared";
import { nonEmptyText } from "../../../utils/text";

export type MangaPagePromptSource = {
  content_sha256: string;
  source_revision_key: string;
};

export const MANGA_PIPELINE_TOTAL_STEPS = 7;

export const chapterHeading = (chapter: ChapterRow): string => {
  const title = nonEmptyText(chapter.title);
  return title
    ? `第 ${chapter.chapter_number} 章 · ${title}`
    : `第 ${chapter.chapter_number} 章`;
};

export const extractMangaPagePromptSource = (
  prompt: MangaPagePromptSummary,
): MangaPagePromptSource => {
  return {
    content_sha256: prompt.source.content_sha256,
    source_revision_key: prompt.source.source_revision_key,
  };
};

export const isMangaPagePromptSourceStale = (
  source: MangaPagePromptSource,
  currentContentHash: string | null,
): boolean =>
  currentContentHash != null && source.content_sha256 !== currentContentHash;

export const countStaleMangaPagePrompts = (
  prompts: MangaPagePromptSummary[],
  currentContentHash: string | null,
): number =>
  prompts.filter((prompt) =>
    isMangaPagePromptSourceStale(
      extractMangaPagePromptSource(prompt),
      currentContentHash,
    ),
  ).length;

export const mangaStageLabel = (stage: string | null | undefined): string => {
  switch (stage) {
    case "character_binding":
      return "角色绑定";
    case "speech_selection":
      return "对话旁白";
    case "visual_anchor_extraction":
      return "视觉锚点";
    case "item_extraction":
      return "关键物品";
    case "comic_synthesis":
      return "章节综合";
    case "page_planning":
      return "分页规划";
    case "drawing_prompt":
      return "整页提示词";
    case "persisting":
      return "写入结果";
    case "completed":
      return "已完成";
    case "failed":
      return "失败";
    case "interrupted":
      return "已中断";
    case "cancelled":
      return "已取消";
    default:
      return stage || "未开始";
  }
};
