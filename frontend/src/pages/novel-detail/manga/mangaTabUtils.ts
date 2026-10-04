import type {
  MangaPagePipelineStage,
  MangaPagePromptSummary,
} from "../../../api/manga";
import type { MangaPipelineStopAfter } from "../../../api/generated/host";
import type { ChapterRow } from "../shared";
import { nonEmptyText } from "../../../utils/text";
import type { MessageTranslator } from "../../../i18n";

export type MangaPageWorkspaceStage =
  | "page_plan"
  | "page_adaptation"
  | "storyboard"
  | "prompts"
  | "images";

export const mangaStopAfterForStage = (
  stage: MangaPageWorkspaceStage,
): MangaPipelineStopAfter => {
  switch (stage) {
    case "page_plan":
      return "page_slots";
    case "page_adaptation":
      return "page_adaptation";
    case "storyboard":
      return "storyboard";
    case "prompts":
      return "drawing_prompt";
    case "images":
      return "image";
  }
};

export type MangaPagePromptSource = {
  content_sha256: string;
  source_revision_key: string;
};

export const chapterHeading = (
  chapter: ChapterRow,
  t: MessageTranslator,
): string => {
  const title = nonEmptyText(chapter.title);
  const heading = t("comic.chapter.heading", { number: chapter.chapter_number });
  return title ? `${heading} · ${title}` : heading;
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

export const mangaStageLabel = (
  stage: string | null | undefined,
  t: MessageTranslator,
): string => {
  switch (stage) {
    case "character_binding":
      return t("comic.pipelineStages.characterBinding");
    case "speech_selection":
      return t("comic.pipelineStages.dialogueNarration");
    case "visual_anchor_extraction":
      return t("comic.pipelineStages.visualAnchor");
    case "item_extraction":
      return t("comic.pipelineStages.keyObjects");
    case "comic_synthesis":
      return t("comic.pipelineStages.chapterSynthesis");
    case "page_planning":
      return t("comic.pipelineStages.pagination");
    case "page_content":
      return t("comic.pipelineStages.pageContent");
    case "adaptation":
      return t("comic.pipelineStages.pageAdaptation");
    case "storyboard":
      return t("comic.pipelineStages.storyboard");
    case "drawing_prompt":
      return t("comic.pipelineStages.pagePrompt");
    case "image":
      return t("comic.pipelineStages.pageImage");
    case "persisting":
      return t("comic.pipelineStages.resultWritten");
    case "completed":
      return t("comic.pipelineStages.completed");
    case "failed":
      return t("comic.pipelineStages.failed");
    case "interrupted":
      return t("comic.pipelineStages.interrupted");
    case "cancelled":
      return t("comic.pipelineStages.cancelled");
    default:
      return stage || t("comic.pipelineStages.notStarted");
  }
};

export const mangaPageStageLabel = (
  stage: MangaPagePipelineStage | null | undefined,
  t: MessageTranslator,
): string => {
  switch (stage) {
    case "slots":
      return t("comic.pipelineStages.pagination");
    case "content":
      return t("comic.pipelineStages.pageContent");
    case "adaptation":
      return t("comic.pipelineStages.pageAdaptation");
    case "storyboard":
      return t("comic.pipelineStages.storyboard");
    case "drawing_prompt":
      return t("comic.pipelineStages.pagePrompt");
    case "image":
      return t("comic.pipelineStages.pageImage");
    default:
      return t("comic.pipelineStages.notStarted");
  }
};

export const mangaPageImageBlockReason = (
  prompt: MangaPagePromptSummary,
  t: MessageTranslator,
): string | null => {
  const requiredStage: MangaPagePipelineStage | null = !prompt.page_content
    ? "content"
    : !prompt.page_adaptation
      ? "adaptation"
      : !prompt.page_plan
        ? "storyboard"
        : !prompt.drawing_prompt
          ? "drawing_prompt"
          : null;
  if (requiredStage) {
    return t("comic.prompt.imageStageRequired", {
      page: prompt.page_number,
      stage: mangaPageStageLabel(requiredStage, t),
    });
  }
  if (prompt.pipeline_stage == null) {
    return t("comic.prompt.imagePipelineRequired", { page: prompt.page_number });
  }
  if (prompt.pipeline_stage !== "image") {
    return t("comic.prompt.imageStageRequired", {
      page: prompt.page_number,
      stage: mangaPageStageLabel(prompt.pipeline_stage, t),
    });
  }
  return null;
};
