import type { MangaPanel, MangaPipelineStartStage } from "../../../api/manga";
import type { ChapterRow } from "../shared";
import { nonEmptyText } from "../../../utils/text";

export type MangaPanelSource = {
  content_sha256: string | null;
  content_chars: number | null;
};

type MangaPanelSourceStatus = {
  sourcedCount: number;
  missingCount: number;
  staleCount: number;
  firstHash: string | null;
};

export type MangaPipelineStartStageOption = MangaPipelineStartStage | "";

export const MANGA_PIPELINE_START_STAGE_OPTIONS: Array<{
  value: MangaPipelineStartStageOption;
  label: string;
}> = [
  { value: "", label: "从头运行" },
  { value: "event_beats", label: "事件卡片" },
  { value: "character_binding", label: "角色绑定" },
  { value: "speech_selection", label: "对白旁白" },
  { value: "visual_anchor_extraction", label: "视觉锚点" },
  { value: "item_extraction", label: "关键物品" },
  { value: "comic_synthesis", label: "章节综合" },
  { value: "page_planning", label: "分页规划" },
  { value: "drawing_prompt", label: "整页提示词" },
];

export const pipelineStartStageValue = (
  value: MangaPipelineStartStageOption,
): MangaPipelineStartStage | null => {
  return value === "" ? null : value;
};

export const chapterHeading = (chapter: ChapterRow): string => {
  const title = nonEmptyText(chapter.title);
  return title ? `第 ${chapter.chapter_number} 章 · ${title}` : `第 ${chapter.chapter_number} 章`;
};

export const extractMangaPanelSource = (layout: unknown): MangaPanelSource | null => {
  if (!layout || typeof layout !== "object") return null;
  const source = (layout as { source?: unknown }).source;
  if (!source || typeof source !== "object") return null;
  const rawHash = (source as { content_sha256?: unknown }).content_sha256;
  const rawChars = (source as { content_chars?: unknown }).content_chars;
  const contentHash = typeof rawHash === "string" && rawHash.trim() ? rawHash.trim() : null;
  const contentChars = typeof rawChars === "number" && Number.isFinite(rawChars) ? rawChars : null;
  if (!contentHash && contentChars == null) return null;
  return {
    content_sha256: contentHash,
    content_chars: contentChars,
  };
};

export const isMangaPanelSourceStale = (
  source: MangaPanelSource | null,
  currentContentHash: string | null,
): boolean => {
  if (!source) return false;
  const contentStale =
    source.content_sha256 != null &&
    currentContentHash != null &&
    source.content_sha256 !== currentContentHash;
  return contentStale;
};

export const summarizePanelSources = (
  panels: MangaPanel[],
  currentContentHash: string | null,
): MangaPanelSourceStatus => {
  let sourcedCount = 0;
  let missingCount = 0;
  let staleCount = 0;
  let firstHash: string | null = null;
  for (const panel of panels) {
    const source = extractMangaPanelSource(panel.layout);
    if (!source) {
      missingCount += 1;
      continue;
    }
    sourcedCount += 1;
    firstHash ||= source.content_sha256;
    if (isMangaPanelSourceStale(source, currentContentHash)) {
      staleCount += 1;
    }
  }
  return { sourcedCount, missingCount, staleCount, firstHash };
};

export const mangaStageLabel = (stage: string | null | undefined): string => {
  switch (stage) {
    case "event_beats":
      return "事件卡片";
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
    case "cancelled":
      return "已取消";
    default:
      return stage || "未开始";
  }
};
